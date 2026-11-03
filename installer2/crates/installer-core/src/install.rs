use crate::{
    Branch, discovery, patch, platform,
    transaction::{self, Change},
};
use anyhow::{Context, Result, ensure};
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
    thread,
    time::Duration,
};

#[derive(Clone, Serialize)]
pub struct Progress {
    pub step: u8,
    pub message: String,
}

pub fn install(branch: Branch, report: impl FnMut(Progress)) -> Result<String> {
    install_with_cancel(branch, report, || false)
}

pub fn install_with_cancel(
    branch: Branch,
    mut report: impl FnMut(Progress),
    is_cancelled: impl Fn() -> bool,
) -> Result<String> {
    let mut progress = |step, message: &str| {
        report(Progress {
            step,
            message: message.to_owned(),
        })
    };
    progress(1, "Locating Steam, Project Zomboid and Workshop files...");
    let environment = discovery::discover()?;
    platform::ensure_game_closed()?;
    let workshop = loop {
        ensure!(!is_cancelled(), "Installation cancelled");
        let path = discovery::workshop_path(&environment.libraries, branch);
        if let Some(path) = path.as_ref()
            && discovery::workshop_item_installed(&environment.libraries, branch)
            && discovery::workshop_ready(path)
        {
            break path.clone();
        }
        progress(
            1,
            &format!(
                "Waiting for Steam to finish downloading the {} Workshop build…",
                if matches!(branch, Branch::Lts) {
                    "LTS"
                } else {
                    "Beta"
                }
            ),
        );
        thread::sleep(Duration::from_secs(2));
    };
    progress(2, "Closing Steam before reading its configuration...");
    platform::stop_steam(&environment.steam_path)?;
    let result = (|| {
        platform::ensure_game_closed()?;
        // Re-read configuration only after Steam has flushed its final writes.
        let changes =
            prepare_changes(&environment.game_path, &environment.config_paths, &workshop)?;
        let mut step = 2;
        transaction::apply(&changes, |path, bytes| {
            step += 1;
            progress(
                step.min(7),
                &format!(
                    "Updating {}",
                    path.file_name().unwrap_or_default().to_string_lossy()
                ),
            );
            transaction::atomic_write(path, bytes)
        })
    })();
    if let Err(error) = result {
        // Rollback has completed before Steam can reopen the configuration.
        let restart = platform::start_steam(&environment.steam_path);
        return match restart {
            Ok(()) => Err(error),
            Err(restart) => {
                Err(error).context(format!("Steam also failed to restart: {restart:#}"))
            }
        };
    }
    progress(8, "Installation completed. Starting Steam...");
    // Restart is outside the file transaction: a launch failure must not undo a valid install.
    match platform::start_steam(&environment.steam_path) {
        Ok(()) => Ok("Installation completed successfully. Steam is starting.".into()),
        Err(error) => Ok(format!(
            "Installation completed. Start Steam manually: {error:#}"
        )),
    }
}

/// Prepare and transactionally apply installation changes using explicit, isolated paths.
/// The caller is responsible for closing the game and Steam first.
pub fn install_from_paths(
    game_path: &Path,
    config_paths: &[PathBuf],
    workshop: &Path,
) -> Result<()> {
    let changes = prepare_changes(game_path, config_paths, workshop)?;
    transaction::apply(&changes, transaction::atomic_write)
}

pub fn uninstall() -> Result<String> {
    let environment = discovery::discover()?;
    platform::ensure_game_closed()?;
    platform::stop_steam(&environment.steam_path)?;
    let result = uninstall_from_paths(&environment.game_path, &environment.config_paths);
    let restart = platform::start_steam(&environment.steam_path);
    match (result, restart) {
        (Ok(()), Ok(())) => {
            Ok("ZombieBuddy was uninstalled successfully. Steam is starting.".into())
        }
        (Ok(()), Err(error)) => Ok(format!(
            "ZombieBuddy was uninstalled. Start Steam manually: {error:#}"
        )),
        (Err(error), Ok(())) => Err(error),
        (Err(error), Err(restart)) => {
            Err(error.context(format!("Steam also failed to restart: {restart:#}")))
        }
    }
}

pub fn uninstall_from_paths(game_path: &Path, config_paths: &[PathBuf]) -> Result<()> {
    let mut changes = Vec::new();
    for name in ["ProjectZomboid64.json", "ProjectZomboid64.bat"] {
        let path = game_path.join(name);
        if !path.is_file() {
            continue;
        }
        let input = fs::read(&path)?;
        let bytes = if name.ends_with(".json") {
            patch::uninstall_launcher_json(&input)?
        } else {
            patch::uninstall_launcher_bat(&input)?
        };
        if bytes != input {
            changes.push(Change {
                path,
                bytes,
                remove: false,
            });
        }
    }
    for path in config_paths {
        let input = fs::read(path)?;
        if let Some(bytes) = patch::uninstall_steam_config(&input)?
            && bytes != input
        {
            changes.push(Change {
                path: path.clone(),
                bytes,
                remove: false,
            });
        }
    }
    for name in ["zbNative.dll", "ZombieBuddy.jar"] {
        let path = game_path.join(name);
        if path.is_file() {
            changes.push(Change {
                path,
                bytes: Vec::new(),
                remove: true,
            });
        }
    }
    ensure!(
        !changes.is_empty(),
        "No installed ZombieBuddy files or launch options were found"
    );
    transaction::apply(&changes, transaction::atomic_write)
}

fn prepare_changes(
    game_path: &Path,
    config_paths: &[PathBuf],
    workshop: &Path,
) -> Result<Vec<Change>> {
    let mut changes = Vec::new();
    for name in ["zbNative.dll", "ZombieBuddy.jar"] {
        let source = workshop.join("mods/ZombieBuddy/libs").join(name);
        let bytes =
            fs::read(&source).with_context(|| format!("Could not read {}", source.display()))?;
        ensure!(!bytes.is_empty(), "Workshop file {name} is empty");
        changes.push(Change {
            path: game_path.join(name),
            bytes,
            remove: false,
        });
    }
    let mut patched = 0;
    for path in config_paths {
        let input = fs::read(path)?;
        if let Some(bytes) = patch::steam_config(&input)
            .with_context(|| format!("Invalid Steam config: {}", path.display()))?
        {
            changes.push(Change {
                path: path.clone(),
                bytes,
                remove: false,
            });
            patched += 1;
        }
    }
    ensure!(
        patched > 0,
        "Project Zomboid was not found in any Steam user configuration; launch the game from Steam once"
    );
    let json = game_path.join("ProjectZomboid64.json");
    changes.push(Change {
        bytes: patch::launcher_json(
            &fs::read(&json).context("Could not read ProjectZomboid64.json")?,
        )?,
        path: json,
        remove: false,
    });
    let bat = game_path.join("ProjectZomboid64.bat");
    changes.push(Change {
        bytes: patch::launcher_bat(
            &fs::read(&bat).context("Could not read ProjectZomboid64.bat")?,
        )?,
        path: bat,
        remove: false,
    });
    Ok(changes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_launcher_does_not_modify_any_installation_files() {
        let root = tempfile::tempdir().unwrap();
        let game = root.path().join("game");
        let workshop = root.path().join("workshop");
        fs::create_dir_all(&game).unwrap();
        fs::create_dir_all(workshop.join("mods/ZombieBuddy/libs")).unwrap();
        for name in ["zbNative.dll", "ZombieBuddy.jar"] {
            fs::write(
                workshop.join("mods/ZombieBuddy/libs").join(name),
                b"payload",
            )
            .unwrap();
        }
        let config = root.path().join("localconfig.vdf");
        let original = br#""UserLocalConfigStore" { "Software" { "Valve" { "Steam" { "apps" { "108600" { "LaunchOptions" "" } } } } } }"#;
        fs::write(&config, original).unwrap();
        fs::write(game.join("ProjectZomboid64.json"), b"invalid json").unwrap();
        assert!(prepare_changes(&game, std::slice::from_ref(&config), &workshop).is_err());
        assert!(!game.join("zbNative.dll").exists());
        assert_eq!(fs::read(config).unwrap(), original);
    }
}
