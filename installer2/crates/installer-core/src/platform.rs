use crate::{Branch, InstallationLock, TargetPlatform};
use anyhow::{Context, Result, ensure};
use std::{
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::{Duration, Instant},
};
use sysinfo::{ProcessesToUpdate, System};

pub(crate) fn ensure_supported() -> Result<()> {
    crate::target::ensure_native_target()
}

#[cfg(windows)]
pub(crate) fn steam_root() -> Result<PathBuf> {
    use winreg::{RegKey, enums::*};
    for hive in [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
        for view in [KEY_WOW64_32KEY, KEY_WOW64_64KEY] {
            if let Ok(key) = RegKey::predef(hive)
                .open_subkey_with_flags(r"Software\Valve\Steam", KEY_READ | view)
            {
                for name in ["SteamPath", "InstallPath"] {
                    if let Ok(value) = key.get_value::<String, _>(name) {
                        let path = PathBuf::from(value.trim());
                        if path.is_absolute() && path.join("steam.exe").is_file() {
                            return Ok(path);
                        }
                    }
                }
            }
        }
    }
    anyhow::bail!("Steam installation was not found in the Windows registry")
}

#[cfg(not(windows))]
pub(crate) fn steam_root() -> Result<PathBuf> {
    let home = home_dir().context("Could not locate the home directory")?;
    steam_candidates(&home, TargetPlatform::current())
        .into_iter()
        .find(|path| {
            path.is_dir()
                && ["steamapps", "steam.sh", "steam"]
                    .iter()
                    .any(|name| path.join(name).exists())
        })
        .context("Could not find a Steam installation")
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map(PathBuf::from)
}

pub fn steam_candidates(home: &Path, target: TargetPlatform) -> Vec<PathBuf> {
    match target {
        TargetPlatform::Windows => Vec::new(),
        TargetPlatform::Macos => vec![home.join("Library/Application Support/Steam")],
        TargetPlatform::Linux => [
            ".steam/steam",
            ".local/share/Steam",
            ".local/share/steam",
            ".var/app/com.valvesoftware.Steam/data/Steam",
            "snap/steam/common/.steam/steam",
        ]
        .iter()
        .map(|p| home.join(p))
        .collect(),
    }
}

fn system() -> System {
    let mut system = System::new();
    system.refresh_processes(ProcessesToUpdate::All, true);
    system
}

pub(crate) fn ensure_game_closed() -> Result<()> {
    ensure!(
        !system().processes().values().any(|p| {
            let name = p.name().to_string_lossy();
            [
                "ProjectZomboid64.exe",
                "ProjectZomboid32.exe",
                "ProjectZomboid64",
                "ProjectZomboid32",
                "ProjectZomboid",
            ]
            .iter()
            .any(|n| name.eq_ignore_ascii_case(n))
                || p.cmd().iter().any(|arg| {
                    arg.to_string_lossy()
                        .contains("zombie.gameStates.MainScreenState")
                })
        }),
        "Close Project Zomboid before installing"
    );
    Ok(())
}

pub fn steam_running() -> bool {
    system().processes().values().any(|p| {
        ["steam.exe", "steam", "steam_osx", "Steam"]
            .iter()
            .any(|name| p.name().to_string_lossy().eq_ignore_ascii_case(name))
    })
}

fn steam_command(root: &Path) -> Result<Command> {
    let mut command = if cfg!(windows) {
        let executable = root.join("steam.exe");
        ensure!(executable.is_file(), "Steam executable was not found");
        Command::new(executable)
    } else if cfg!(target_os = "macos") {
        let mut apps = vec![
            root.join("Steam.app"),
            PathBuf::from("/Applications/Steam.app"),
        ];
        if let Some(home) = home_dir() {
            apps.push(home.join("Applications/Steam.app"));
        }
        let executable = apps
            .into_iter()
            .flat_map(|app| {
                [
                    app.join("Contents/MacOS/steam_osx"),
                    app.join("Contents/MacOS/steam"),
                ]
            })
            .find(|p| p.is_file())
            .context("Steam executable was not found")?;
        Command::new(executable)
    } else if root
        .components()
        .any(|p| p.as_os_str() == "com.valvesoftware.Steam")
    {
        let mut command = Command::new("flatpak");
        command.args(["run", "com.valvesoftware.Steam"]);
        command
    } else {
        let executable = [
            root.join("steam.sh"),
            root.join("steam"),
            PathBuf::from("/snap/bin/steam"),
            PathBuf::from("/usr/bin/steam"),
        ]
        .into_iter()
        .find(|p| p.is_file())
        .context("Steam executable was not found")?;
        Command::new(executable)
    };
    command.current_dir(root);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    Ok(command)
}

pub(crate) fn stop_steam(root: &Path) -> Result<()> {
    if !steam_running() {
        return Ok(());
    }
    steam_command(root)?
        .arg("-shutdown")
        .spawn()
        .context("Could not ask Steam to shut down")?;
    let start = Instant::now();
    while steam_running() {
        ensure!(
            start.elapsed() < Duration::from_secs(30),
            "Steam is still running; close it manually and retry"
        );
        thread::sleep(Duration::from_millis(500));
    }
    Ok(())
}

pub(crate) fn start_steam(root: &Path) -> Result<()> {
    steam_command(root)?
        .spawn()
        .context("Could not start Steam")?;
    Ok(())
}

pub fn open_workshop(branch: Branch) -> Result<()> {
    ensure_supported()?;
    let root = steam_root()?;
    let _lock = InstallationLock::acquire(&root)?;
    steam_command(&root)?
        .arg(format!(
            "steam://url/CommunityFilePage/{}",
            branch.workshop_id()
        ))
        .spawn()
        .context("Could not open Workshop")?;
    Ok(())
}

// These tests invoke the real client, so normal cargo test and CI must skip them.
#[cfg(all(test, windows))]
mod native_tests {
    use super::*;

    struct RestoreRunningSteam(PathBuf);

    impl Drop for RestoreRunningSteam {
        fn drop(&mut self) {
            if !steam_running()
                && let Err(error) = start_steam(&self.0)
            {
                eprintln!("Could not restore Steam after native test: {error:#}");
            }
        }
    }

    struct RestoreSteamFiles {
        root: PathBuf,
        snapshots: Vec<(PathBuf, Option<Vec<u8>>)>,
        armed: bool,
    }

    impl RestoreSteamFiles {
        fn new(root: PathBuf) -> Self {
            Self {
                root,
                snapshots: Vec::new(),
                armed: true,
            }
        }

        fn capture(&mut self, paths: &[PathBuf]) -> Result<()> {
            for path in paths {
                if self.snapshots.iter().any(|(saved, _)| saved == path) {
                    continue;
                }
                let contents = match std::fs::symlink_metadata(path) {
                    Ok(metadata) => {
                        ensure!(metadata.is_file(), "Not a regular file: {}", path.display());
                        Some(std::fs::read(path)?)
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                    Err(error) => return Err(error.into()),
                };
                self.snapshots.push((path.clone(), contents));
            }
            Ok(())
        }

        fn restore(&mut self) -> Result<()> {
            if steam_running() {
                stop_steam(&self.root)?;
            }
            let mut failures = Vec::new();
            for (path, contents) in self.snapshots.iter().rev() {
                let result = match contents {
                    Some(bytes) => crate::transaction::atomic_write(path, bytes),
                    None => match std::fs::remove_file(path) {
                        Ok(()) => Ok(()),
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                        Err(error) => Err(error.into()),
                    },
                };
                if let Err(error) = result {
                    failures.push(format!("{}: {error:#}", path.display()));
                }
            }
            ensure!(
                failures.is_empty(),
                "Could not restore Steam files: {}",
                failures.join("; ")
            );

            start_steam(&self.root)?;
            let started = Instant::now();
            while !steam_running() {
                ensure!(
                    started.elapsed() < Duration::from_secs(30),
                    "Steam did not restart after restoring its files"
                );
                thread::sleep(Duration::from_millis(500));
            }
            Ok(())
        }
    }

    impl Drop for RestoreSteamFiles {
        fn drop(&mut self) {
            if self.armed
                && let Err(error) = self.restore()
            {
                eprintln!("Could not restore Steam files after native test: {error:#}");
            }
        }
    }

    #[test]
    #[ignore = "Opt-in native acceptance: gracefully closes and restarts the real Steam client"]
    fn native_windows_steam_shutdown_restart() -> Result<()> {
        ensure_game_closed()?;
        let root = steam_root()?;
        ensure!(
            steam_running(),
            "Start Steam before running native acceptance; no process was changed"
        );
        // Arm restoration before shutdown so errors and panic unwinding both retry startup.
        let _restore = RestoreRunningSteam(root.clone());
        let shutdown = Instant::now();
        stop_steam(&root)?;
        ensure!(!steam_running(), "Steam remained running after shutdown");
        println!("PASS: Steam exited gracefully in {:?}", shutdown.elapsed());

        // Closing an already stopped client must also succeed without starting it.
        stop_steam(&root)?;
        ensure!(
            !steam_running(),
            "Repeated shutdown unexpectedly started Steam"
        );

        let startup = Instant::now();
        start_steam(&root)?;
        while !steam_running() {
            ensure!(
                startup.elapsed() < Duration::from_secs(30),
                "Steam did not appear within 30 seconds after startup"
            );
            thread::sleep(Duration::from_millis(500));
        }
        println!("PASS: Steam process restarted in {:?}", startup.elapsed());
        println!("No installer, patch, Workshop, download or configuration commands were invoked.");
        Ok(())
    }

    #[test]
    #[ignore = "Opt-in native acceptance: patches and restores real Steam localconfig backups"]
    fn native_windows_steam_localconfig_backup_roundtrip() -> Result<()> {
        use crate::{
            TargetPlatform, discovery, install,
            transaction::{self, Change},
        };

        ensure_game_closed()?;
        let root = steam_root()?;
        ensure!(
            steam_running(),
            "Start Steam before running native acceptance; no process was changed"
        );
        let mut restore = RestoreSteamFiles::new(root.clone());
        stop_steam(&root)?;
        ensure!(!steam_running(), "Steam remained running after shutdown");

        let environment = discovery::discover_from_for(&root, TargetPlatform::Windows)?;
        ensure!(
            !environment.config_paths.is_empty(),
            "No Steam user localconfig.vdf files were found"
        );
        let mut guarded_paths = Vec::new();
        for config in &environment.config_paths {
            guarded_paths.push(config.clone());
            guarded_paths.push(transaction::backup_path(config));
        }
        restore.capture(&guarded_paths)?;

        let original_configs: Vec<_> = environment
            .config_paths
            .iter()
            .map(|path| Ok((path.clone(), std::fs::read(path)?)))
            .collect::<Result<_>>()?;
        let original_backups: Vec<_> = environment
            .config_paths
            .iter()
            .map(|config| {
                let backup = transaction::backup_path(config);
                let contents = match std::fs::read(&backup) {
                    Ok(bytes) => Some(bytes),
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                    Err(error) => return Err(error.into()),
                };
                Ok((backup, contents))
            })
            .collect::<Result<_>>()?;

        let mut changes: Vec<Change> = Vec::new();
        let patched = install::add_steam_patches(
            &mut changes,
            &environment.config_paths,
            TargetPlatform::Windows,
        )?;
        ensure!(
            patched > 0,
            "No Project Zomboid launch options were patched"
        );
        let transaction_root = tempfile::tempdir()?;
        transaction::apply_at(transaction_root.path(), &changes, transaction::atomic_write)?;

        for ((config, original), (backup, prior_backup)) in
            original_configs.iter().zip(&original_backups)
        {
            let patched_config = crate::patch::steam_config_for(original, TargetPlatform::Windows)?;
            if let Some(patched_config) = patched_config {
                ensure!(
                    std::fs::read(config)? == patched_config,
                    "Steam config patch did not persist to {}",
                    config.display()
                );
                let expected_backup = prior_backup.as_deref().unwrap_or(original);
                ensure!(
                    std::fs::read(backup)? == expected_backup,
                    "First-original backup was not preserved for {}",
                    config.display()
                );
            }
        }
        println!("PASS: real Steam launch options and first-original backups were verified.");

        restore.restore()?;
        for (path, original) in &original_configs {
            ensure!(
                std::fs::read(path)? == *original,
                "Could not restore {}",
                path.display()
            );
        }
        for (path, original) in &original_backups {
            match original {
                Some(bytes) => ensure!(
                    std::fs::read(path)? == *bytes,
                    "Could not restore {}",
                    path.display()
                ),
                None => ensure!(
                    !path.exists(),
                    "Temporary backup remains at {}",
                    path.display()
                ),
            }
        }
        ensure!(
            steam_running(),
            "Steam was not running after the native test"
        );
        restore.armed = false;
        println!("PASS: original Steam configs/backups were restored and Steam restarted.");
        Ok(())
    }

    #[test]
    #[ignore = "Opt-in native acceptance: installs, uninstalls and restores the real game files"]
    fn native_windows_game_install_uninstall_roundtrip() -> Result<()> {
        use crate::{TargetPlatform, discovery, install, target, transaction};

        ensure_game_closed()?;
        let root = steam_root()?;
        ensure!(
            steam_running(),
            "Start Steam before running native acceptance; no process was changed"
        );
        let mut restore = RestoreSteamFiles::new(root.clone());
        stop_steam(&root)?;
        ensure!(!steam_running(), "Steam remained running after shutdown");

        let environment = discovery::discover_from_for(&root, TargetPlatform::Windows)?;
        ensure!(
            !environment.config_paths.is_empty(),
            "No Steam user localconfig.vdf files were found"
        );
        ensure!(
            !environment.game_path.join(transaction::JOURNAL).exists(),
            "An unfinished installation transaction exists; recover it before testing"
        );
        let branch = environment
            .branches
            .iter()
            .find(|branch| branch.ready)
            .map(|branch| branch.branch)
            .context("No subscribed, ready ZombieBuddy Workshop branch is available")?;
        let workshop =
            discovery::ready_workshop_path(&environment.libraries, branch, TargetPlatform::Windows)
                .context("The selected Workshop payload is not ready")?;
        let game = &environment.game_path;
        let json = target::launcher_json_path(game)?;
        let batch = TargetPlatform::Windows.launcher(game);
        ensure!(json.is_file(), "The game's JSON launcher is missing");
        ensure!(batch.is_file(), "The game's BAT launcher is missing");

        let mut guarded_paths = vec![
            game.join(transaction::JOURNAL),
            json.clone(),
            batch.clone(),
            game.join("zbNative.dll"),
            game.join("ZombieBuddy.jar"),
        ];
        let mut patch_paths = vec![json.clone(), batch.clone()];
        for config in &environment.config_paths {
            guarded_paths.push(config.clone());
            patch_paths.push(config.clone());
        }
        for path in &patch_paths {
            guarded_paths.push(transaction::backup_path(path));
        }
        restore.capture(&guarded_paths)?;

        let original_patches: Vec<_> = patch_paths
            .iter()
            .map(|path| Ok((path.clone(), std::fs::read(path)?)))
            .collect::<Result<_>>()?;
        let previous_backups: Vec<_> = patch_paths
            .iter()
            .map(|path| {
                let backup = transaction::backup_path(path);
                let contents = match std::fs::read(&backup) {
                    Ok(bytes) => Some(bytes),
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                    Err(error) => return Err(error.into()),
                };
                Ok((backup, contents))
            })
            .collect::<Result<_>>()?;
        let original_payloads: Vec<_> = ["zbNative.dll", "ZombieBuddy.jar"]
            .into_iter()
            .map(|name| {
                let source = workshop.join("mods/ZombieBuddy/libs").join(name);
                Ok((game.join(name), std::fs::read(source)?))
            })
            .collect::<Result<_>>()?;

        let expected_patches: Vec<_> = original_patches
            .iter()
            .map(|(path, original)| {
                let patched = if path == &json {
                    Some(crate::patch::launcher_json_for(
                        original,
                        TargetPlatform::Windows,
                    )?)
                } else if path == &batch {
                    Some(crate::patch::launcher_bat(original)?)
                } else {
                    crate::patch::steam_config_for(original, TargetPlatform::Windows)?
                };
                Ok((path.clone(), original.clone(), patched))
            })
            .collect::<Result<_>>()?;
        ensure!(
            expected_patches
                .iter()
                .any(|(_, _, patched)| patched.is_some()),
            "No game or Steam configuration contains patchable launch options"
        );

        install::install(branch, |_| {})?;
        stop_steam(&root)?;
        ensure!(!steam_running(), "Steam remained open after installation");
        for (path, payload) in &original_payloads {
            ensure!(
                std::fs::read(path)? == *payload,
                "Payload copy failed: {}",
                path.display()
            );
        }
        for (path, _, patched) in &expected_patches {
            if let Some(patched) = patched {
                ensure!(
                    std::fs::read(path)? == *patched,
                    "Patch write failed: {}",
                    path.display()
                );
            }
        }
        for ((path, original, patched), (backup, old_backup)) in
            expected_patches.iter().zip(&previous_backups)
        {
            if patched.is_some() {
                let expected_backup = old_backup.as_deref().unwrap_or(original);
                ensure!(
                    std::fs::read(backup)? == expected_backup,
                    "First-original backup mismatch: {}",
                    path.display()
                );
            }
        }

        let installed: Vec<_> = guarded_paths
            .iter()
            .filter(|path| path.is_file())
            .map(|path| Ok((path.clone(), std::fs::read(path)?)))
            .collect::<Result<_>>()?;
        install::install(branch, |_| {})?;
        stop_steam(&root)?;
        ensure!(!steam_running(), "Steam remained open after reinstall");
        for (path, bytes) in &installed {
            ensure!(
                std::fs::read(path)? == *bytes,
                "Install is not idempotent: {}",
                path.display()
            );
        }

        install::uninstall()?;
        stop_steam(&root)?;
        ensure!(!steam_running(), "Steam remained open after uninstall");
        ensure!(
            !game.join("zbNative.dll").exists(),
            "DLL was not removed on uninstall"
        );
        ensure!(
            !game.join("ZombieBuddy.jar").exists(),
            "JAR was not removed on uninstall"
        );
        for (path, original, patched) in &expected_patches {
            let installed = patched.as_deref().unwrap_or(original);
            let expected_clean = if path == &json {
                crate::patch::uninstall_launcher_json(installed, TargetPlatform::Windows)?
            } else if path == &batch {
                crate::patch::uninstall_launcher_bat(installed)?
            } else {
                crate::patch::uninstall_steam_config(installed, TargetPlatform::Windows)?
                    .unwrap_or_else(|| installed.to_vec())
            };
            ensure!(
                std::fs::read(path)? == expected_clean,
                "Uninstall output mismatch: {}",
                path.display()
            );
        }

        println!(
            "PASS: real game discovery, {:?} Workshop payload, install, backups, idempotency and uninstall were verified.",
            branch
        );
        let snapshots = restore.snapshots.clone();
        restore.restore()?;
        for (path, contents) in snapshots {
            match contents {
                Some(original) => ensure!(
                    std::fs::read(&path)? == original,
                    "Could not restore pre-test file: {}",
                    path.display()
                ),
                None => ensure!(
                    !path.exists(),
                    "Test-created file remains after restoration: {}",
                    path.display()
                ),
            }
        }
        ensure!(
            steam_running(),
            "Steam was not running after the native test"
        );
        restore.armed = false;
        println!("PASS: all pre-test game/Steam files were restored and Steam restarted.");
        Ok(())
    }
}
