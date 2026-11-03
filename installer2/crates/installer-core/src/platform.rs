use crate::Branch;
use anyhow::{Context, Result, bail, ensure};
use std::{
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::{Duration, Instant},
};
use sysinfo::{ProcessesToUpdate, System};

pub(crate) fn ensure_supported() -> Result<()> {
    ensure!(
        cfg!(windows),
        "This migration currently supports Windows only"
    );
    Ok(())
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
                        let path = PathBuf::from(value);
                        if path.is_absolute() && path.join("steam.exe").is_file() {
                            return Ok(path);
                        }
                    }
                }
            }
        }
    }
    bail!("Steam installation was not found in the Windows registry")
}

#[cfg(not(windows))]
pub(crate) fn steam_root() -> Result<PathBuf> {
    bail!("Steam discovery is not yet ported for this platform")
}

fn system() -> System {
    let mut system = System::new();
    system.refresh_processes(ProcessesToUpdate::All, true);
    system
}

pub(crate) fn ensure_game_closed() -> Result<()> {
    ensure!(
        !system().processes().values().any(|p| p
            .name()
            .to_string_lossy()
            .eq_ignore_ascii_case("ProjectZomboid64.exe")),
        "Close Project Zomboid before installing"
    );
    Ok(())
}

pub fn steam_running() -> bool {
    system()
        .processes()
        .values()
        .any(|p| p.name().to_string_lossy().eq_ignore_ascii_case("steam.exe"))
}

fn steam_command(root: &Path) -> Result<Command> {
    let executable = root.join("steam.exe");
    ensure!(executable.is_file(), "Steam executable was not found");
    let mut command = Command::new(executable);
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
    steam_command(&steam_root()?)?
        .arg(format!(
            "steam://url/CommunityFilePage/{}",
            branch.workshop_id()
        ))
        .spawn()
        .context("Could not open Workshop")?;
    Ok(())
}
