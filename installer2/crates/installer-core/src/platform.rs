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
}
