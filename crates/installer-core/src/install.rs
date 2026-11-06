use crate::{
    Branch, InstallationLock, TargetPlatform,
    discovery::{self, Discovery},
    patch, platform, target,
    transaction::{self, Change},
};
use anyhow::{Context, Result, ensure};
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};

#[derive(Clone, Serialize)]
pub struct Progress {
    pub step: u8,
    pub message: String,
}

// Keep the production orchestration testable without redirecting global Steam
// discovery or ever changing the developer's Steam/game files.
trait InstallRuntime {
    fn steam_root(&self) -> Result<PathBuf>;
    fn ensure_game_closed(&self) -> Result<()>;
    fn stop_steam(&self, root: &Path) -> Result<()>;
    fn start_steam(&self, root: &Path) -> Result<()>;
    fn write(&self, path: &Path, bytes: &[u8]) -> Result<()> {
        transaction::atomic_write(path, bytes)
    }
    fn download_timing(&self) -> (Duration, Duration) {
        (Duration::from_secs(60), Duration::from_millis(500))
    }
}

struct NativeRuntime;

impl InstallRuntime for NativeRuntime {
    fn steam_root(&self) -> Result<PathBuf> {
        platform::steam_root()
    }
    fn ensure_game_closed(&self) -> Result<()> {
        platform::ensure_game_closed()
    }
    fn stop_steam(&self, root: &Path) -> Result<()> {
        platform::stop_steam(root)
    }
    fn start_steam(&self, root: &Path) -> Result<()> {
        platform::start_steam(root)
    }
}

pub fn install(branch: Branch, report: impl FnMut(Progress)) -> Result<String> {
    install_with_cancel(branch, report, || false)
}

pub fn install_with_cancel(
    branch: Branch,
    report: impl FnMut(Progress),
    is_cancelled: impl Fn() -> bool,
) -> Result<String> {
    platform::ensure_supported()?;
    install_with_runtime(
        branch,
        TargetPlatform::current(),
        report,
        is_cancelled,
        &NativeRuntime,
    )
}

fn install_with_runtime(
    branch: Branch,
    target: TargetPlatform,
    mut report: impl FnMut(Progress),
    is_cancelled: impl Fn() -> bool,
    runtime: &impl InstallRuntime,
) -> Result<String> {
    let mut progress = |step, message: &str| {
        report(Progress {
            step,
            message: message.to_owned(),
        })
    };
    progress(1, "Locating Steam, Project Zomboid and Workshop files...");
    let root = runtime.steam_root()?;
    let _lock = InstallationLock::acquire(&root)?;
    let mut environment = discovery::discover_from_for(&root, target)?;
    runtime.ensure_game_closed()?;
    ensure!(
        discovery::workshop_path(&environment.libraries, branch).is_some(),
        "Selected Workshop branch is not subscribed; open its Workshop page first"
    );
    let (timeout, interval) = runtime.download_timing();
    let workshop = wait_for_workshop(timeout, interval, is_cancelled, || {
        // Steam can add a library or finish a subscription while the installer waits.
        environment = discovery::discover_from_for(&root, target)?;
        let ready = discovery::ready_workshop_path(&environment.libraries, branch, target);
        if ready.is_none() {
            progress(
                1,
                "Waiting for Steam to finish downloading the Workshop build…",
            );
        }
        Ok(ready)
    })?;
    progress(2, "Closing Steam before reading its configuration...");
    with_stopped_steam_using(&environment, runtime, |environment| {
        let _game_lock = InstallationLock::acquire(&environment.game_path)?;
        recover_for(&environment.game_path, &environment.config_paths, target)?;
        let changes = prepare_changes(
            &environment.game_path,
            &environment.config_paths,
            &workshop,
            target,
        )?;
        let total = changes.len().max(1);
        let mut completed = 0;
        transaction::apply_at(&environment.game_path, &changes, |path, bytes| {
            runtime.write(path, bytes)?;
            completed += 1;
            progress(
                (2 + completed * 5 / total) as u8,
                &format!(
                    "Updating {}",
                    path.file_name().unwrap_or_default().to_string_lossy()
                ),
            );
            Ok(())
        })?;
        progress(8, "Installation completed. Starting Steam...");
        Ok("Installation completed successfully.".into())
    })
}

fn wait_for_workshop(
    timeout: Duration,
    interval: Duration,
    is_cancelled: impl Fn() -> bool,
    mut poll: impl FnMut() -> Result<Option<PathBuf>>,
) -> Result<PathBuf> {
    let start = Instant::now();
    loop {
        ensure!(!is_cancelled(), "Installation cancelled");
        if let Some(path) = poll()? {
            return Ok(path);
        }
        ensure!(
            start.elapsed() < timeout,
            "Workshop item did not finish downloading within {} seconds; retry after Steam finishes",
            timeout.as_secs()
        );
        thread::sleep(interval.min(timeout.saturating_sub(start.elapsed())));
    }
}

/// All native mutations share shutdown, game check, post-shutdown rediscovery and restart handling.
pub(crate) fn with_stopped_steam(
    environment: &Discovery,
    action: impl FnOnce(&Discovery) -> Result<String>,
) -> Result<String> {
    with_stopped_steam_using(environment, &NativeRuntime, action)
}

fn with_stopped_steam_using(
    environment: &Discovery,
    runtime: &impl InstallRuntime,
    action: impl FnOnce(&Discovery) -> Result<String>,
) -> Result<String> {
    runtime.ensure_game_closed()?;
    runtime.stop_steam(&environment.steam_path)?;
    let result = (|| {
        runtime.ensure_game_closed()?;
        let refreshed =
            discovery::discover_from_for(&environment.steam_path, environment.platform)?;
        action(&refreshed)
    })();
    if result.is_err() && environment.game_path.join(transaction::JOURNAL).exists() {
        return result.context("Steam was left closed because recovery is incomplete; retry the installer before starting Steam");
    }
    let restart = runtime.start_steam(&environment.steam_path);
    match (result, restart) {
        (Ok(message), Ok(())) => Ok(format!("{message} Steam is starting.")),
        (Ok(message), Err(error)) => Ok(format!("{message} Start Steam manually: {error:#}")),
        (Err(error), Ok(())) => Err(error),
        (Err(error), Err(restart)) => {
            Err(error.context(format!("Steam also failed to restart: {restart:#}")))
        }
    }
}

#[cfg(test)]
#[path = "install_scenarios.rs"]
mod scenario_tests;

pub fn install_from_paths(game: &Path, configs: &[PathBuf], workshop: &Path) -> Result<()> {
    install_from_paths_for(game, configs, workshop, TargetPlatform::current())
}

/// Explicit, isolated targets for fixture tests. Caller closes Steam/the game for real installations.
pub fn install_from_paths_for(
    game: &Path,
    configs: &[PathBuf],
    workshop: &Path,
    target: TargetPlatform,
) -> Result<()> {
    let _lock = InstallationLock::acquire(game)?;
    recover_for(game, configs, target)?;
    let changes = prepare_changes(game, configs, workshop, target)?;
    transaction::apply_at(game, &changes, transaction::atomic_write)
}

pub fn uninstall() -> Result<String> {
    let root = platform::steam_root()?;
    let _lock = InstallationLock::acquire(&root)?;
    let environment = discovery::discover_from_for(root, TargetPlatform::current())?;
    with_stopped_steam(&environment, |environment| {
        uninstall_from_paths(&environment.game_path, &environment.config_paths)?;
        Ok("ZombieBuddy was uninstalled successfully.".into())
    })
}

pub fn uninstall_from_paths(game: &Path, configs: &[PathBuf]) -> Result<()> {
    uninstall_from_paths_for(game, configs, TargetPlatform::current())
}

pub fn uninstall_from_paths_for(
    game: &Path,
    configs: &[PathBuf],
    target: TargetPlatform,
) -> Result<()> {
    let _lock = InstallationLock::acquire(game)?;
    recover_for(game, configs, target)?;
    let mut changes = Vec::new();
    if let Some(path) = target::find_launcher_json(game)? {
        let input = fs::read(&path)?;
        let bytes = patch::uninstall_launcher_json(&input, target)?;
        if bytes != input {
            changes.push(Change {
                path,
                bytes,
                remove: false,
            });
        }
    }
    let path = target.launcher(game);
    if path.is_file() {
        let input = fs::read(&path)?;
        let bytes = match target {
            TargetPlatform::Windows => patch::uninstall_launcher_bat(&input)?,
            _ => patch::uninstall_launcher_shell(&input, target)?,
        };
        if bytes != input {
            changes.push(Change {
                path,
                bytes,
                remove: false,
            });
        }
    }
    for path in configs {
        let input = fs::read(path)?;
        if let Some(bytes) = patch::uninstall_steam_config(&input, target)?
            && bytes != input
        {
            changes.push(Change {
                path: path.clone(),
                bytes,
                remove: false,
            });
        }
    }
    for name in target.files() {
        let path = game.join(name);
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
    transaction::apply_at(game, &changes, transaction::atomic_write)
}

pub(crate) fn recover_for(game: &Path, configs: &[PathBuf], target: TargetPlatform) -> Result<()> {
    let mut allowed = vec![
        target.launcher(game),
        game.join(target.files()[0]),
        game.join("ZombieBuddy.jar"),
    ];
    allowed.push(
        target::find_launcher_json(game)?.unwrap_or_else(|| game.join("ProjectZomboid64.json")),
    );
    allowed.extend_from_slice(configs);
    let backups: Vec<_> = allowed
        .iter()
        .map(|path| transaction::backup_path(path))
        .collect();
    allowed.extend(backups);
    transaction::recover(game, &allowed)?;
    Ok(())
}

pub(crate) fn add_patch(
    changes: &mut Vec<Change>,
    path: PathBuf,
    transform: impl FnOnce(&[u8]) -> Result<Vec<u8>>,
) -> Result<()> {
    let input = fs::read(&path).with_context(|| format!("Could not read {}", path.display()))?;
    let bytes = transform(&input).with_context(|| format!("Could not patch {}", path.display()))?;
    transaction::preserve_original(changes, &path, &input)?;
    if bytes != input {
        changes.push(Change {
            path,
            bytes,
            remove: false,
        });
    }
    Ok(())
}

pub(crate) fn add_steam_patches(
    changes: &mut Vec<Change>,
    configs: &[PathBuf],
    target: TargetPlatform,
) -> Result<usize> {
    let mut patched = 0;
    for path in configs {
        let input = fs::read(path)?;
        if let Some(bytes) = patch::steam_config_for(&input, target)
            .with_context(|| format!("Invalid Steam config: {}", path.display()))?
        {
            transaction::preserve_original(changes, path, &input)?;
            if bytes != input {
                changes.push(Change {
                    path: path.clone(),
                    bytes,
                    remove: false,
                });
            }
            patched += 1;
        }
    }
    ensure!(
        patched > 0,
        "Project Zomboid was not found in any Steam user configuration; launch the game from Steam once"
    );
    Ok(patched)
}

fn prepare_changes(
    game: &Path,
    configs: &[PathBuf],
    workshop: &Path,
    target: TargetPlatform,
) -> Result<Vec<Change>> {
    let mut changes = Vec::new();
    for name in target.files() {
        let source = workshop.join("mods/ZombieBuddy/libs").join(name);
        let bytes =
            fs::read(&source).with_context(|| format!("Could not read {}", source.display()))?;
        ensure!(!bytes.is_empty(), "Workshop file {name} is empty");
        changes.push(Change {
            path: game.join(name),
            bytes,
            remove: false,
        });
    }
    add_steam_patches(&mut changes, configs, target)?;
    add_patch(&mut changes, target::launcher_json_path(game)?, |input| {
        patch::launcher_json_for(input, target)
    })?;
    add_patch(&mut changes, target.launcher(game), |input| match target {
        TargetPlatform::Windows => patch::launcher_bat(input),
        _ => patch::launcher_shell(input, target),
    })?;
    Ok(changes)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn download_wait_can_be_cancelled_and_times_out_without_platform_operations() {
        assert!(
            wait_for_workshop(
                Duration::from_secs(1),
                Duration::ZERO,
                || true,
                || panic!("cancel must precede poll")
            )
            .unwrap_err()
            .to_string()
            .contains("cancelled")
        );
        assert!(
            wait_for_workshop(Duration::ZERO, Duration::ZERO, || false, || Ok(None))
                .unwrap_err()
                .to_string()
                .contains("did not finish")
        );
        let path = PathBuf::from("fixture");
        assert_eq!(
            wait_for_workshop(
                Duration::ZERO,
                Duration::ZERO,
                || false,
                || Ok(Some(path.clone()))
            )
            .unwrap(),
            path
        );
    }
}
