use anyhow::{Context, Result};
use std::{
    fs::{File, OpenOptions},
    path::Path,
};

/// OS file lock; the OS releases it even if the installer crashes. Never unlink the lock file.
pub struct InstallationLock {
    _file: File,
}

impl InstallationLock {
    pub fn acquire(directory: &Path) -> Result<Self> {
        let path = directory.join(".zombiebuddy-installer.lock");
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .with_context(|| format!("Could not open installer lock {}", path.display()))?;
        file.try_lock()
            .context("Another ZombieBuddy installer is running; close it and retry")?;
        Ok(Self { _file: file })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn independent_handles_exclude_each_other_and_release_on_drop() {
        let root = tempfile::tempdir().unwrap();
        let guard = InstallationLock::acquire(root.path()).unwrap();
        assert!(InstallationLock::acquire(root.path()).is_err());
        drop(guard);
        assert!(InstallationLock::acquire(root.path()).is_ok());
    }

    #[test]
    fn child_lock_probe() {
        let Some(root) = std::env::var_os("ZB_TEST_LOCK_ROOT") else {
            return;
        };
        assert!(InstallationLock::acquire(Path::new(&root)).is_err());
    }

    #[test]
    fn a_second_process_cannot_acquire_the_installation_lock() {
        let root = tempfile::tempdir().unwrap();
        let _guard = InstallationLock::acquire(root.path()).unwrap();
        let child = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "lock::tests::child_lock_probe"])
            .env("ZB_TEST_LOCK_ROOT", root.path())
            .output()
            .unwrap();
        assert!(
            child.status.success(),
            "{}",
            String::from_utf8_lossy(&child.stderr)
        );
    }
}
