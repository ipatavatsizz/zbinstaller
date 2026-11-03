use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
};

pub(crate) const JOURNAL: &str = ".zombiebuddy-transaction.json";
pub(crate) struct Change {
    pub path: PathBuf,
    pub bytes: Vec<u8>,
    pub remove: bool,
}
#[derive(Serialize, Deserialize)]
struct Snapshot {
    path: PathBuf,
    bytes: Option<Vec<u8>>,
    readonly: bool,
    mode: Option<u32>,
}
#[derive(Serialize, Deserialize)]
struct Journal {
    version: u8,
    snapshots: Vec<Snapshot>,
}

fn sync_parent(path: &Path) -> Result<()> {
    #[cfg(unix)]
    fs::File::open(path.parent().context("Missing parent directory")?)?.sync_all()?;
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

pub(crate) fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut temporary =
        tempfile::NamedTempFile::new_in(path.parent().context("Missing parent directory")?)?;
    temporary.write_all(bytes)?;
    if let Ok(metadata) = fs::metadata(path) {
        temporary
            .as_file()
            .set_permissions(metadata.permissions())?;
    }
    temporary.as_file().sync_all()?;
    temporary
        .persist(path)
        .map_err(|e| e.error)
        .with_context(|| format!("Could not replace {}", path.display()))?;
    sync_parent(path)
}

pub(crate) fn backup_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(".backup");
    name.into()
}

/// First-original backups belong to the patch transaction. Existing Go backups are preserved.
pub(crate) fn preserve_original(
    changes: &mut Vec<Change>,
    path: &Path,
    input: &[u8],
) -> Result<()> {
    let backup = backup_path(path);
    match fs::symlink_metadata(&backup) {
        Ok(metadata) => ensure!(
            metadata.is_file(),
            "Backup is not a regular file: {}",
            backup.display()
        ),
        Err(error) if error.kind() == ErrorKind::NotFound => changes.push(Change {
            path: backup,
            bytes: input.to_vec(),
            remove: false,
        }),
        Err(error) => {
            return Err(error).with_context(|| format!("Could not inspect {}", backup.display()));
        }
    }
    Ok(())
}

fn capture(changes: &[Change]) -> Result<Vec<Snapshot>> {
    let mut seen = std::collections::HashSet::new();
    changes
        .iter()
        .map(|change| {
            ensure!(
                seen.insert(&change.path),
                "Duplicate transaction target: {}",
                change.path.display()
            );
            ensure!(
                change.path.is_absolute(),
                "Transaction paths must be absolute"
            );
            let (bytes, readonly, mode) = match fs::symlink_metadata(&change.path) {
                Ok(metadata) => {
                    ensure!(
                        metadata.is_file(),
                        "Transaction target must be a regular file: {}",
                        change.path.display()
                    );
                    #[cfg(unix)]
                    let mode = {
                        use std::os::unix::fs::PermissionsExt;
                        Some(metadata.permissions().mode())
                    };
                    #[cfg(not(unix))]
                    let mode = None;
                    (
                        Some(fs::read(&change.path)?),
                        metadata.permissions().readonly(),
                        mode,
                    )
                }
                Err(e) if e.kind() == ErrorKind::NotFound => (None, false, None),
                Err(e) => {
                    return Err(e)
                        .with_context(|| format!("Snapshot failed: {}", change.path.display()));
                }
            };
            Ok(Snapshot {
                path: change.path.clone(),
                bytes,
                readonly,
                mode,
            })
        })
        .collect()
}

fn remove(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => sync_parent(path),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

fn restore(snapshots: &[Snapshot]) -> Result<()> {
    let mut failures = Vec::new();
    for snapshot in snapshots.iter().rev() {
        let result: Result<()> = (|| match &snapshot.bytes {
            Some(bytes) => {
                atomic_write(&snapshot.path, bytes)?;
                let mut permissions = fs::metadata(&snapshot.path)?.permissions();
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if let Some(mode) = snapshot.mode {
                        permissions.set_mode(mode);
                    }
                }
                #[cfg(not(unix))]
                permissions.set_readonly(snapshot.readonly);
                fs::set_permissions(&snapshot.path, permissions)?;
                #[cfg(unix)]
                fs::File::open(&snapshot.path)?.sync_all()?;
                Ok(())
            }
            None => remove(&snapshot.path),
        })();
        if let Err(e) = result {
            failures.push(format!("{}: {e:#}", snapshot.path.display()));
        }
    }
    ensure!(
        failures.is_empty(),
        "Rollback incomplete: {}",
        failures.join("; ")
    );
    Ok(())
}

/// Caller holds the OS lock and has stopped Steam. Reject every target outside the managed paths.
pub(crate) fn recover(root: &Path, allowed: &[PathBuf]) -> Result<bool> {
    let path = root.join(JOURNAL);
    let input = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(e.into()),
    };
    let journal: Journal = serde_json::from_slice(&input)
        .context("Invalid recovery journal; keep it for manual recovery")?;
    ensure!(journal.version == 1, "Unsupported recovery journal version");
    for snapshot in &journal.snapshots {
        ensure!(
            allowed.contains(&snapshot.path),
            "Recovery target is outside the managed installation: {}",
            snapshot.path.display()
        );
    }
    restore(&journal.snapshots)?;
    remove(&path)?;
    Ok(true)
}

pub(crate) fn apply_at(
    root: &Path,
    changes: &[Change],
    mut write: impl FnMut(&Path, &[u8]) -> Result<()>,
) -> Result<()> {
    let journal_path = root.join(JOURNAL);
    ensure!(
        !journal_path.exists(),
        "Unfinished transaction must be recovered before installing"
    );
    let journal = Journal {
        version: 1,
        snapshots: capture(changes)?,
    };
    atomic_write(&journal_path, &serde_json::to_vec(&journal)?)?;
    let mut attempted = 0;
    let result: Result<()> = (|| {
        for change in changes {
            attempted += 1;
            if change.remove {
                remove(&change.path)?;
            } else {
                write(&change.path, &change.bytes)?;
            }
        }
        remove(&journal_path)?;
        Ok(())
    })();
    if let Err(error) = result {
        if let Err(rollback) = restore(&journal.snapshots[..attempted]) {
            bail!("{error:#}; {rollback:#}; keep the recovery journal and retry");
        }
        remove(&journal_path)
            .context("Files restored but recovery journal could not be removed")?;
        return Err(error).context("Installation failed; original files restored");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failure_restores_existing_and_removes_new_files_and_backups() {
        let root = tempfile::tempdir().unwrap();
        let existing = root.path().join("existing");
        let new = root.path().join("new");
        fs::write(&existing, b"original").unwrap();
        let mut changes = Vec::new();
        preserve_original(&mut changes, &existing, b"original").unwrap();
        changes.extend([
            Change {
                path: existing.clone(),
                bytes: b"changed".to_vec(),
                remove: false,
            },
            Change {
                path: new.clone(),
                bytes: b"new".to_vec(),
                remove: false,
            },
        ]);
        let result = apply_at(root.path(), &changes, |path, bytes| {
            atomic_write(path, bytes)?;
            if path == new {
                bail!("Injected write failure");
            }
            Ok(())
        });
        assert!(result.is_err());
        assert_eq!(fs::read(existing).unwrap(), b"original");
        assert!(!new.exists());
        assert!(!root.path().join("existing.backup").exists());
        assert!(!root.path().join(JOURNAL).exists());
    }
    #[test]
    fn next_run_recovers_interrupted_writes_and_rejects_foreign_paths() {
        let root = tempfile::tempdir().unwrap();
        let existing = root.path().join("existing");
        let new = root.path().join("new");
        fs::write(&existing, b"original").unwrap();
        let changes = [
            Change {
                path: existing.clone(),
                bytes: vec![],
                remove: false,
            },
            Change {
                path: new.clone(),
                bytes: vec![],
                remove: false,
            },
        ];
        let journal = Journal {
            version: 1,
            snapshots: capture(&changes).unwrap(),
        };
        atomic_write(
            &root.path().join(JOURNAL),
            &serde_json::to_vec(&journal).unwrap(),
        )
        .unwrap();
        fs::write(&existing, b"partial").unwrap();
        fs::write(&new, b"partial").unwrap();
        assert!(recover(root.path(), std::slice::from_ref(&existing)).is_err());
        assert_eq!(fs::read(&existing).unwrap(), b"partial");
        assert!(recover(root.path(), &[existing.clone(), new.clone()]).unwrap());
        assert_eq!(fs::read(existing).unwrap(), b"original");
        assert!(!new.exists());
        assert!(!recover(root.path(), &[]).unwrap());
    }
}
