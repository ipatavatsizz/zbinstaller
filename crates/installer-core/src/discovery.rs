use crate::{Branch, TargetPlatform, platform, target, vdf};
use anyhow::{Result, ensure};
use serde::Serialize;
use std::{
    fs,
    path::{Component, Path, PathBuf},
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Discovery {
    pub platform: TargetPlatform,
    pub warnings: Vec<String>,
    pub steam_path: PathBuf,
    pub game_path: PathBuf,
    pub libraries: Vec<PathBuf>,
    pub config_paths: Vec<PathBuf>,
    pub branches: Vec<BranchInfo>,
    pub files: Vec<FileInfo>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileInfo {
    pub name: String,
    pub exists: bool,
    pub size: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct BranchInfo {
    pub branch: Branch,
    pub path: Option<PathBuf>,
    pub version: Option<String>,
    pub ready: bool,
}

pub fn discover() -> Result<Discovery> {
    platform::ensure_supported()?;
    discover_from_for(platform::steam_root()?, TargetPlatform::current())
}

/// Diagnostics remains useful even when Steam or the game has not been installed yet.
pub fn inspect() -> Discovery {
    let target = TargetPlatform::current();
    let root = platform::steam_root();
    match root {
        Ok(root) => match inspect_from(&root, target) {
            Ok(found) => found,
            Err(error) => empty_discovery(root, target, format!("{error:#}")),
        },
        Err(error) => empty_discovery(PathBuf::new(), target, format!("{error:#}")),
    }
}

fn empty_discovery(steam_path: PathBuf, platform: TargetPlatform, warning: String) -> Discovery {
    Discovery {
        platform,
        warnings: vec![warning],
        steam_path,
        game_path: PathBuf::new(),
        libraries: vec![],
        config_paths: vec![],
        branches: [Branch::Lts, Branch::Beta]
            .into_iter()
            .map(|branch| BranchInfo {
                branch,
                path: None,
                version: None,
                ready: false,
            })
            .collect(),
        files: vec![],
    }
}

pub fn inspect_from(steam_path: impl Into<PathBuf>, target: TargetPlatform) -> Result<Discovery> {
    scan_from(steam_path.into(), target, false)
}

/// Discover a Steam installation rooted at an explicit path.
pub fn discover_from(steam_path: impl Into<PathBuf>) -> Result<Discovery> {
    discover_from_for(steam_path, TargetPlatform::current())
}

pub fn discover_from_for(
    steam_path: impl Into<PathBuf>,
    target: TargetPlatform,
) -> Result<Discovery> {
    scan_from(steam_path.into(), target, true)
}

fn scan_from(steam_path: PathBuf, target: TargetPlatform, require_game: bool) -> Result<Discovery> {
    let mut libraries = vec![steam_path.clone()];
    let index = steam_path.join("steamapps/libraryfolders.vdf");
    if index.exists() {
        let entries = vdf::parse(&fs::read_to_string(&index)?)?;
        if let Some(folders) = vdf::object_at(&entries, &["libraryfolders"]) {
            for folder in folders {
                let path = match &folder.value {
                    vdf::Value::Object(children) => vdf::text_at(children, "path"),
                    vdf::Value::Text(path, _) if folder.key.parse::<u32>().is_ok() => {
                        Some(path.as_str())
                    }
                    _ => None,
                };
                if let Some(path) = path {
                    let path = PathBuf::from(path);
                    if path.is_absolute() && path.is_dir() && !libraries.contains(&path) {
                        libraries.push(path);
                    }
                }
            }
        }
    }
    let mut game_path = None;
    for library in &libraries {
        let manifest = library.join("steamapps/appmanifest_108600.acf");
        if !manifest.exists() {
            continue;
        }
        let Some(entries) = fs::read_to_string(&manifest)
            .ok()
            .and_then(|input| vdf::parse(&input).ok())
        else {
            continue;
        };
        if let Some(app) = vdf::object_at(&entries, &["AppState"])
            && let Some(dir) = vdf::text_at(app, "installdir")
        {
            ensure!(
                !dir.is_empty()
                    && Path::new(dir)
                        .components()
                        .all(|c| matches!(c, Component::Normal(_))),
                "Invalid game directory in Steam manifest"
            );
            let candidate = library.join("steamapps/common").join(dir);
            if candidate.is_dir() {
                game_path = Some(candidate);
                break;
            }
        }
    }
    let game_path = game_path.or_else(|| {
        libraries
            .iter()
            .map(|p| p.join("steamapps/common/ProjectZomboid"))
            .find(|p| p.is_dir())
    });
    ensure!(
        !require_game || game_path.is_some(),
        "Project Zomboid installation was not found"
    );
    let warnings = if game_path.is_none() {
        vec!["Project Zomboid installation was not found".to_owned()]
    } else {
        vec![]
    };
    let game_path = game_path.unwrap_or_default();
    let mut config_paths = Vec::new();
    let userdata = steam_path.join("userdata");
    if userdata.is_dir() {
        for user in fs::read_dir(userdata)? {
            let path = user?.path().join("config/localconfig.vdf");
            if path.is_file() {
                config_paths.push(path);
            }
        }
    }
    config_paths.sort();
    let branches = [Branch::Lts, Branch::Beta]
        .into_iter()
        .map(|branch| {
            let path = ready_workshop_path(&libraries, branch, target)
                .or_else(|| workshop_path(&libraries, branch));
            let version = path
                .as_ref()
                .and_then(|p| fs::read_to_string(p.join("mods/ZombieBuddy/java/VERSION")).ok())
                .map(|v| v.trim().to_owned());
            let ready = path
                .as_ref()
                .is_some_and(|p| workshop_ready(p, target) && workshop_record_ready(p, branch));
            BranchInfo {
                branch,
                path,
                version,
                ready,
            }
        })
        .collect();
    let json = if game_path.as_os_str().is_empty() {
        None
    } else {
        target::launcher_json_path(&game_path).ok()
    }
    .unwrap_or_else(|| game_path.join("ProjectZomboid64.json"));
    let files = [
        json,
        target.launcher(&game_path),
        game_path.join(target.files()[0]),
        game_path.join("ZombieBuddy.jar"),
    ]
    .into_iter()
    .map(|path| {
        let metadata = if game_path.as_os_str().is_empty() {
            None
        } else {
            fs::metadata(&path).ok()
        };
        FileInfo {
            name: path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            exists: metadata.as_ref().is_some_and(|m| m.is_file()),
            size: metadata.map(|m| m.len()),
        }
    })
    .collect();
    Ok(Discovery {
        platform: target,
        warnings,
        steam_path,
        game_path,
        libraries,
        config_paths,
        branches,
        files,
    })
}

pub(crate) fn workshop_path(libraries: &[PathBuf], branch: Branch) -> Option<PathBuf> {
    libraries
        .iter()
        .map(|p| {
            p.join("steamapps/workshop/content/108600")
                .join(branch.workshop_id())
        })
        .find(|p| p.is_dir())
}

pub(crate) fn workshop_ready(path: &Path, target: TargetPlatform) -> bool {
    target.files().iter().all(|file| {
        fs::metadata(path.join("mods/ZombieBuddy/libs").join(file))
            .is_ok_and(|m| m.is_file() && m.len() > 0)
    })
}

pub fn workshop_manifest_contains_item(input: &str, branch: Branch) -> Result<bool> {
    let entries = vdf::parse(input)?;
    Ok(
        vdf::object_at(&entries, &["AppWorkshop", "WorkshopItemsInstalled"]).is_some_and(|items| {
            items.iter().any(|item| {
                item.key == branch.workshop_id() && matches!(item.value, vdf::Value::Object(_))
            })
        }),
    )
}

fn workshop_record_ready(path: &Path, branch: Branch) -> bool {
    let Some(workshop) = path.ancestors().nth(3) else {
        return false;
    };
    let manifest = workshop.join("appworkshop_108600.acf");
    // Preserve Go compatibility with a locally downloaded payload without an ACF.
    if !manifest.exists() {
        return true;
    }
    fs::read_to_string(manifest)
        .ok()
        .and_then(|input| workshop_manifest_contains_item(&input, branch).ok())
        .unwrap_or(false)
}

pub(crate) fn ready_workshop_path(
    libraries: &[PathBuf],
    branch: Branch,
    target: TargetPlatform,
) -> Option<PathBuf> {
    libraries
        .iter()
        .map(|p| {
            p.join("steamapps/workshop/content/108600")
                .join(branch.workshop_id())
        })
        .find(|p| workshop_ready(p, target) && workshop_record_ready(p, branch))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovers_game_and_workshop_in_secondary_library() {
        let root = tempfile::tempdir().unwrap();
        let steam = root.path().join("steam");
        let library = root.path().join("other library");
        fs::create_dir_all(steam.join("steamapps")).unwrap();
        fs::create_dir_all(library.join("steamapps/common/ProjectZomboid")).unwrap();
        fs::write(
            steam.join("steamapps/libraryfolders.vdf"),
            format!(
                "\"libraryfolders\" {{ \"1\" {{ \"path\" {} }} }}",
                vdf::quote(&library.to_string_lossy())
            ),
        )
        .unwrap();
        fs::write(
            library.join("steamapps/appmanifest_108600.acf"),
            r#""AppState" { "installdir" "ProjectZomboid" }"#,
        )
        .unwrap();
        let libs =
            library.join("steamapps/workshop/content/108600/3619862853/mods/ZombieBuddy/libs");
        fs::create_dir_all(&libs).unwrap();
        fs::write(libs.join("zbNative.dll"), b"dll").unwrap();
        fs::write(libs.join("ZombieBuddy.jar"), b"jar").unwrap();
        let result = discover_from_for(steam, TargetPlatform::Windows).unwrap();
        assert_eq!(
            result.game_path,
            library.join("steamapps/common/ProjectZomboid")
        );
        assert!(result.branches[0].ready);
        assert!(!result.branches[1].ready);
    }

    #[test]
    fn ready_payload_and_installed_record_must_belong_to_the_same_library() {
        let root = tempfile::tempdir().unwrap();
        let stale = root.path().join("stale");
        let ready = root.path().join("ready");
        let branch = Branch::Lts;
        for library in [&stale, &ready] {
            let libs = library
                .join("steamapps/workshop/content/108600")
                .join(branch.workshop_id())
                .join("mods/ZombieBuddy/libs");
            fs::create_dir_all(&libs).unwrap();
            fs::write(libs.join("zbNative.dll"), b"dll").unwrap();
            fs::write(libs.join("ZombieBuddy.jar"), b"jar").unwrap();
        }
        fs::write(
            stale.join("steamapps/workshop/appworkshop_108600.acf"),
            "AppWorkshop { WorkshopItemsInstalled { } }",
        )
        .unwrap();
        let libraries = [stale.clone(), ready.clone()];
        let found = ready_workshop_path(&libraries, branch, TargetPlatform::Windows).unwrap();
        assert!(found.starts_with(&ready));
        fs::write(
            ready.join("steamapps/workshop/appworkshop_108600.acf"),
            "AppWorkshop { WorkshopItemsInstalled { } }",
        )
        .unwrap();
        assert!(ready_workshop_path(&libraries, branch, TargetPlatform::Windows).is_none());
    }
}
