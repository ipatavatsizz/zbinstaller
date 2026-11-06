use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Explicit target selection keeps all platform layouts testable on every host.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TargetPlatform {
    Windows,
    Linux,
    Macos,
}

impl TargetPlatform {
    pub fn current() -> Self {
        if cfg!(windows) {
            Self::Windows
        } else if cfg!(target_os = "macos") {
            Self::Macos
        } else {
            Self::Linux
        }
    }

    pub fn files(self) -> [&'static str; 2] {
        [
            match self {
                Self::Windows => "zbNative.dll",
                Self::Linux => "libzbNative.so",
                Self::Macos => "libzbNative.dylib",
            },
            "ZombieBuddy.jar",
        ]
    }

    /// Steam launch options require the separator after the managed agent.
    pub fn flags(self) -> &'static str {
        match self {
            Self::Windows => "-agentlib:zbNative --",
            _ => "-javaagent:ZombieBuddy.jar --",
        }
    }

    /// Direct JVM launchers accept only the agent argument, without Steam's separator.
    pub fn agent_flags(self) -> &'static str {
        match self {
            Self::Windows => "-agentlib:zbNative",
            _ => "-javaagent:ZombieBuddy.jar",
        }
    }

    pub fn launcher(self, root: &Path) -> PathBuf {
        let names: &[&str] = match self {
            Self::Windows => &["ProjectZomboid64.bat"],
            _ => &[
                "ProjectZomboid64.sh",
                "ProjectZomboid64.command",
                "start-server.sh",
            ],
        };
        names
            .iter()
            .map(|name| root.join(name))
            .find(|path| path.is_file())
            .unwrap_or_else(|| root.join(names[0]))
    }
}

/// Match Go's root-first, case-insensitive recursive JSON search. Do not follow directory symlinks.
pub(crate) fn launcher_json_path(root: &Path) -> Result<PathBuf> {
    find_launcher_json(root)?.ok_or_else(|| {
        anyhow::anyhow!(
            "Could not find ProjectZomboid64.json under {}",
            root.display()
        )
    })
}

pub(crate) fn find_launcher_json(root: &Path) -> Result<Option<PathBuf>> {
    let direct = root.join("ProjectZomboid64.json");
    if direct.is_file() {
        return Ok(Some(direct));
    }
    fn visit(root: &Path) -> Result<Option<PathBuf>> {
        let mut entries = std::fs::read_dir(root)?.collect::<std::io::Result<Vec<_>>>()?;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let kind = entry.file_type()?;
            if entry
                .file_name()
                .to_string_lossy()
                .eq_ignore_ascii_case("ProjectZomboid64.json")
                && entry.path().is_file()
            {
                return Ok(Some(entry.path()));
            }
            if kind.is_dir()
                && let Some(path) = visit(&entry.path())?
            {
                return Ok(Some(path));
            }
        }
        Ok(None)
    }
    visit(root).with_context(|| format!("Could not search {}", root.display()))
}

pub(crate) fn ensure_native_target() -> Result<()> {
    if cfg!(any(windows, target_os = "linux", target_os = "macos")) {
        Ok(())
    } else {
        bail!("Unsupported operating system")
    }
}
