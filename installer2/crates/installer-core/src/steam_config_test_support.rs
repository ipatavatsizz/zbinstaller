// Persistent recovery support used only by the opt-in Windows Steam config test.
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Serialize, Deserialize)]
struct Entry {
    target: PathBuf,
    copy: Option<PathBuf>,
    sha256: Option<String>,
    readonly: bool,
    dacl: Option<String>,
}

pub(super) struct Recovery {
    pub directory: PathBuf,
    entries: Vec<Entry>,
}

fn sha256(path: &Path) -> Result<String> {
    let output = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", "$ErrorActionPreference='Stop'; [BitConverter]::ToString([Security.Cryptography.SHA256]::Create().ComputeHash([IO.File]::ReadAllBytes($env:ZB_HASH_TARGET))).Replace('-','')"])
        .env("ZB_HASH_TARGET", path).output()?;
    ensure!(
        output.status.success(),
        "Could not hash recovery file: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let hash = String::from_utf8(output.stdout)?.trim().to_owned();
    ensure!(
        hash.len() == 64 && hash.bytes().all(|c| c.is_ascii_hexdigit()),
        "Invalid SHA-256 output"
    );
    Ok(hash)
}

fn dacl(path: &Path) -> Result<String> {
    let output = Command::new("powershell.exe").args(["-NoProfile", "-NonInteractive", "-Command", "$ErrorActionPreference='Stop'; [IO.File]::GetAccessControl($env:ZB_ACL_TARGET).GetSecurityDescriptorSddlForm([Security.AccessControl.AccessControlSections]::Access)"])
        .env("ZB_ACL_TARGET", path).output()?;
    ensure!(output.status.success(), "Could not read target DACL");
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

// Windows may change the informational AUTO_INHERITED control bit when setting an ACL.
// Compare the exact ordered binary ACEs and inheritance-protection bit, not that marker.
fn dacl_semantics(sddl: &str) -> Result<String> {
    let output = Command::new("powershell.exe").args(["-NoProfile", "-NonInteractive", "-Command", "$ErrorActionPreference='Stop'; $sd=New-Object Security.AccessControl.RawSecurityDescriptor($env:ZB_ACL_SDDL); $acl=$sd.DiscretionaryAcl; $bytes=New-Object byte[] $acl.BinaryLength; $acl.GetBinaryForm($bytes,0); ([int]$sd.ControlFlags -band [int][Security.AccessControl.ControlFlags]::DiscretionaryAclProtected).ToString()+':'+[Convert]::ToBase64String($bytes)"])
        .env("ZB_ACL_SDDL", sddl).output()?;
    ensure!(output.status.success(), "Could not normalize DACL");
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

fn restore_dacl(path: &Path, sddl: &str) -> Result<()> {
    let output = Command::new("powershell.exe").args(["-NoProfile", "-NonInteractive", "-Command", "$ErrorActionPreference='Stop'; $acl=[IO.File]::GetAccessControl($env:ZB_ACL_TARGET); $acl.SetSecurityDescriptorSddlForm($env:ZB_ACL_SDDL,[Security.AccessControl.AccessControlSections]::Access); [IO.File]::SetAccessControl($env:ZB_ACL_TARGET,$acl)"])
        .env("ZB_ACL_TARGET", path).env("ZB_ACL_SDDL", sddl).output()?;
    ensure!(output.status.success(), "Could not restore target DACL");
    Ok(())
}

fn regular_file(path: &Path) -> Result<Option<fs::Metadata>> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            ensure!(
                metadata.is_file() && !metadata.file_type().is_symlink(),
                "Recovery target must be a regular file: {}",
                path.display()
            );
            Ok(Some(metadata))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

impl Recovery {
    pub fn capture(targets: &[PathBuf], forbidden_root: &Path) -> Result<Self> {
        let directory = tempfile::Builder::new()
            .prefix("zb-steam-config-recovery-")
            .tempdir()?
            .keep();
        ensure!(
            !directory
                .canonicalize()?
                .starts_with(forbidden_root.canonicalize()?),
            "Recovery directory must be outside Steam"
        );
        let mut recovery = Self {
            directory,
            entries: Vec::new(),
        };
        for (index, target) in targets.iter().enumerate() {
            ensure!(
                target.is_absolute()
                    && !recovery.entries.iter().any(|entry| entry.target == *target),
                "Invalid or duplicate recovery target"
            );
            let metadata = regular_file(target)?;
            let (copy, hash, readonly) = if let Some(metadata) = metadata {
                let copy = recovery.directory.join(format!("{index}.original"));
                let bytes = fs::read(target)?;
                let mut file = fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&copy)?;
                file.write_all(&bytes)?;
                file.sync_all()?;
                drop(file);
                ensure!(
                    fs::read(&copy)? == bytes,
                    "Independent backup verification failed"
                );
                let hash = sha256(&copy)?;
                ensure!(
                    sha256(target)? == hash,
                    "Target changed while capturing backup"
                );
                (Some(copy), Some(hash), metadata.permissions().readonly())
            } else {
                (None, None, false)
            };
            let dacl = if copy.is_some() {
                Some(dacl(target)?)
            } else {
                None
            };
            recovery.entries.push(Entry {
                target: target.clone(),
                copy,
                sha256: hash,
                readonly,
                dacl,
            });
        }
        let manifest = recovery.directory.join("manifest.json");
        let bytes = serde_json::to_vec_pretty(&recovery.entries)?;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&manifest)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        let saved: Vec<Entry> = serde_json::from_slice(&fs::read(manifest)?)?;
        ensure!(saved.len() == targets.len(), "Recovery manifest incomplete");
        recovery.validate()?;
        Ok(recovery)
    }

    pub fn verify_backups_after_patch(&self, baselines: &[Vec<u8>]) -> Result<()> {
        for (pair, baseline) in self.entries.as_chunks::<2>().0.iter().zip(baselines) {
            let backup = &pair[1];
            let has_game =
                crate::patch::steam_config_for(baseline, crate::TargetPlatform::Windows)?.is_some();
            if backup.copy.is_some() || has_game {
                let expected = match &backup.copy {
                    Some(copy) => fs::read(copy)?,
                    None => baseline.clone(),
                };
                ensure!(
                    fs::read(&backup.target)? == expected,
                    "First-original backup was changed"
                );
            }
        }
        Ok(())
    }

    fn validate(&self) -> Result<()> {
        for entry in &self.entries {
            if let Some(copy) = &entry.copy {
                ensure!(regular_file(copy)?.is_some(), "Independent backup missing");
                ensure!(
                    Some(sha256(copy)?) == entry.sha256,
                    "Independent backup hash mismatch"
                );
            }
        }
        Ok(())
    }

    pub fn restore(&self) -> Result<()> {
        // Validate every backup before the first restoration write.
        self.validate()
            .with_context(|| format!("Recovery evidence: {}", self.directory.display()))?;
        let mut failures = Vec::new();
        for entry in &self.entries {
            let result: Result<()> = (|| {
                if self.verify_entry(entry).is_ok() {
                    return Ok(());
                }
                regular_file(&entry.target)?;
                match &entry.copy {
                    Some(copy) => {
                        crate::transaction::atomic_write(&entry.target, &fs::read(copy)?)?;
                        let mut permissions = fs::metadata(&entry.target)?.permissions();
                        permissions.set_readonly(entry.readonly);
                        fs::set_permissions(&entry.target, permissions)?;
                        restore_dacl(
                            &entry.target,
                            entry.dacl.as_deref().context("Missing DACL")?,
                        )?;
                    }
                    None => match fs::remove_file(&entry.target) {
                        Ok(()) => (),
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
                        Err(error) => return Err(error.into()),
                    },
                }
                self.verify_entry(entry)
            })();
            if let Err(error) = result {
                failures.push(format!("{}: {error:#}", entry.target.display()));
            }
        }
        ensure!(
            failures.is_empty(),
            "Cleanup failed; Steam must stay closed. Recovery: {}; {}",
            self.directory.display(),
            failures.join("; ")
        );
        Ok(())
    }

    fn verify_entry(&self, entry: &Entry) -> Result<()> {
        let metadata = regular_file(&entry.target)?;
        if let Some(copy) = &entry.copy {
            let metadata = metadata.context("Restored target missing")?;
            ensure!(
                fs::read(&entry.target)? == fs::read(copy)?
                    && Some(sha256(&entry.target)?) == entry.sha256,
                "Restored bytes/hash differ"
            );
            ensure!(
                metadata.permissions().readonly() == entry.readonly,
                "Restored readonly permission differs"
            );
            ensure!(
                dacl_semantics(&dacl(&entry.target)?)?
                    == dacl_semantics(entry.dacl.as_deref().context("Missing original DACL")?)?,
                "Restored DACL differs"
            );
        } else {
            ensure!(metadata.is_none(), "Originally absent target still exists");
        }
        Ok(())
    }
}

#[test]
fn recovery_restores_config_existing_backup_and_absence() -> Result<()> {
    let root = tempfile::tempdir()?;
    let config = root.path().join("localconfig.vdf");
    let backup = root.path().join("localconfig.vdf.backup");
    let absent = root.path().join("absent.backup");
    fs::write(&config, b"original config")?;
    fs::write(&backup, b"older backup")?;
    let recovery = Recovery::capture(
        &[config.clone(), backup.clone(), absent.clone()],
        root.path(),
    )?;
    fs::write(&config, b"patched")?;
    fs::remove_file(&backup)?;
    fs::write(&absent, b"temporary")?;
    recovery.restore()?;
    assert_eq!(fs::read(config)?, b"original config");
    assert_eq!(fs::read(backup)?, b"older backup");
    assert!(!absent.exists());
    fs::remove_dir_all(recovery.directory)?;
    Ok(())
}

#[test]
fn corrupt_backup_prevents_all_restore_writes() -> Result<()> {
    let root = tempfile::tempdir()?;
    let config = root.path().join("localconfig.vdf");
    fs::write(&config, b"original")?;
    let recovery = Recovery::capture(std::slice::from_ref(&config), root.path())?;
    fs::write(&config, b"patched")?;
    fs::write(recovery.entries[0].copy.as_ref().unwrap(), b"corrupt")?;
    assert!(recovery.restore().is_err());
    assert_eq!(fs::read(config)?, b"patched");
    fs::remove_dir_all(recovery.directory)?;
    Ok(())
}

#[test]
fn cleanup_failure_is_an_error_and_keeps_recovery() -> Result<()> {
    let root = tempfile::tempdir()?;
    let config = root.path().join("localconfig.vdf");
    fs::write(&config, b"original")?;
    let recovery = Recovery::capture(std::slice::from_ref(&config), root.path())?;
    fs::remove_file(&config)?;
    fs::create_dir(&config)?;
    assert!(recovery.restore().is_err());
    assert!(recovery.directory.join("manifest.json").exists());
    fs::remove_dir(&config)?;
    recovery.restore()?;
    fs::remove_dir_all(recovery.directory)?;
    Ok(())
}

/// Independently compare bytes outside the PZ LaunchOptions value and preserve its user options.
pub(super) fn verify_scoped_patch(before: &[u8], after: &[u8]) -> Result<()> {
    fn options(bytes: &[u8]) -> Result<(String, std::ops::Range<usize>)> {
        let entries = crate::vdf::parse(std::str::from_utf8(bytes)?)?;
        let app = crate::vdf::unique_object_at(
            &entries,
            &[
                "UserLocalConfigStore",
                "Software",
                "Valve",
                "Steam",
                "apps",
                "108600",
            ],
        )?
        .context("Missing PZ entry")?;
        let option = app
            .iter()
            .find(|entry| entry.key.eq_ignore_ascii_case("LaunchOptions"))
            .context("Missing launch options")?;
        match &option.value {
            crate::vdf::Value::Text(value, span) => Ok((value.clone(), span.clone())),
            _ => anyhow::bail!("Launch options must be text"),
        }
    }
    let (old, old_span) = options(before)?;
    let (new, new_span) = options(after)?;
    ensure!(
        before[..old_span.start] == after[..new_span.start]
            && before[old_span.end..] == after[new_span.end..],
        "Unrelated config bytes changed"
    );
    let flags = crate::TargetPlatform::Windows.flags();
    let expected = if old.contains(flags) {
        old
    } else {
        format!("{flags} {}", old.trim()).trim_end().to_owned()
    };
    ensure!(new == expected, "Unrelated launch options changed");
    Ok(())
}

#[test]
fn scoped_verification_rejects_unrelated_changes() -> Result<()> {
    let original = br#"UserLocalConfigStore { Software { Valve { Steam { apps { 108600 { LaunchOptions "-debug" } 42 { LaunchOptions keep } } } } } }"#;
    let patched =
        crate::patch::steam_config_for(original, crate::TargetPlatform::Windows)?.unwrap();
    verify_scoped_patch(original, &patched)?;
    let tampered = String::from_utf8(patched)?.replace("keep", "changed");
    assert!(verify_scoped_patch(original, tampered.as_bytes()).is_err());
    Ok(())
}

#[test]
fn dacl_comparison_preserves_rights_order_and_inheritance() -> Result<()> {
    let inherited = "D:AI(A;ID;FR;;;WD)";
    assert_eq!(
        dacl_semantics(inherited)?,
        dacl_semantics("D:(A;ID;FR;;;WD)")?
    );
    for different in ["D:P(A;ID;FR;;;WD)", "D:(A;;FR;;;WD)", "D:(A;ID;FA;;;WD)"] {
        assert_ne!(dacl_semantics(inherited)?, dacl_semantics(different)?);
    }
    assert_ne!(
        dacl_semantics("D:(A;;FR;;;WD)(D;;FW;;;WD)")?,
        dacl_semantics("D:(D;;FW;;;WD)(A;;FR;;;WD)")?
    );
    Ok(())
}
