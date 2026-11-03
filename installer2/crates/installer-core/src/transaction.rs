use anyhow::{Context, Result, bail};
use std::{
    fs,
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
};

pub(crate) struct Change {
    pub path: PathBuf,
    pub bytes: Vec<u8>,
    pub remove: bool,
}

struct Snapshot {
    path: PathBuf,
    bytes: Option<Vec<u8>>,
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
    Ok(())
}

/// Capture every target before the first write. Attempt every restore on failure.
pub(crate) fn apply(
    changes: &[Change],
    mut write: impl FnMut(&Path, &[u8]) -> Result<()>,
) -> Result<()> {
    let snapshots: Vec<Snapshot> = changes
        .iter()
        .map(|change| {
            let bytes = match fs::read(&change.path) {
                Ok(bytes) => Some(bytes),
                Err(e) if e.kind() == ErrorKind::NotFound => None,
                Err(e) => {
                    return Err(e)
                        .with_context(|| format!("Snapshot failed: {}", change.path.display()));
                }
            };
            Ok(Snapshot {
                path: change.path.clone(),
                bytes,
            })
        })
        .collect::<Result<_>>()?;
    for (index, change) in changes.iter().enumerate() {
        let result = if change.remove {
            match fs::remove_file(&change.path) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
                Err(error) => Err(error.into()),
            }
        } else {
            write(&change.path, &change.bytes)
        };
        if let Err(error) = result {
            let mut failures = Vec::new();
            for snapshot in snapshots[..=index].iter().rev() {
                let result = match &snapshot.bytes {
                    Some(bytes) => atomic_write(&snapshot.path, bytes),
                    None => match fs::remove_file(&snapshot.path) {
                        Ok(()) => Ok(()),
                        Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
                        Err(e) => Err(e.into()),
                    },
                };
                if let Err(e) = result {
                    failures.push(format!("{}: {e:#}", snapshot.path.display()));
                }
            }
            if failures.is_empty() {
                return Err(error).context("Installation failed; original files restored");
            }
            bail!("{error:#}; rollback incomplete: {}", failures.join("; "));
        }
    }
    Ok(())
}

/// Remove multiple files as one rollback-capable operation.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failure_restores_existing_and_removes_new_files() {
        let root = tempfile::tempdir().unwrap();
        let existing = root.path().join("existing");
        let new = root.path().join("new");
        fs::write(&existing, b"original").unwrap();
        let changes = vec![
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
        ];
        let result = apply(&changes, |path, bytes| {
            atomic_write(path, bytes)?;
            if path == new {
                bail!("Injected write failure");
            }
            Ok(())
        });
        assert!(result.is_err());
        assert_eq!(fs::read(existing).unwrap(), b"original");
        assert!(!new.exists());
    }
}
