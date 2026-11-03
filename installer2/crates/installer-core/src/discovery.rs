use crate::{Branch, platform, vdf};
use anyhow::{Context, Result, ensure};
use serde::Serialize;
use std::{
    fs,
    path::{Component, Path, PathBuf},
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Discovery {
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
    discover_from(platform::steam_root()?)
}

/// Discover a Steam installation rooted at an explicit path.
pub fn discover_from(steam_path: impl Into<PathBuf>) -> Result<Discovery> {
    let steam_path = steam_path.into();
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
        let entries = vdf::parse(&fs::read_to_string(&manifest)?)?;
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
    let game_path = game_path
        .or_else(|| {
            libraries
                .iter()
                .map(|p| p.join("steamapps/common/ProjectZomboid"))
                .find(|p| p.is_dir())
        })
        .context("Project Zomboid installation was not found")?;
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
            let path = workshop_path(&libraries, branch);
            let version = path
                .as_ref()
                .and_then(|p| fs::read_to_string(p.join("mods/ZombieBuddy/java/VERSION")).ok())
                .map(|v| v.trim().to_owned());
            let ready = path.as_ref().is_some_and(|p| workshop_ready(p));
            BranchInfo {
                branch,
                path,
                version,
                ready,
            }
        })
        .collect();
    let files = [
        "ProjectZomboid64.json",
        "ProjectZomboid64.bat",
        "zbNative.dll",
        "ZombieBuddy.jar",
    ]
    .into_iter()
    .map(|name| {
        let metadata = fs::metadata(game_path.join(name)).ok();
        FileInfo {
            name: name.to_owned(),
            exists: metadata.as_ref().is_some_and(|m| m.is_file()),
            size: metadata.map(|m| m.len()),
        }
    })
    .collect();
    Ok(Discovery {
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

pub(crate) fn workshop_ready(path: &Path) -> bool {
    ["zbNative.dll", "ZombieBuddy.jar"].iter().all(|file| {
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

pub(crate) fn workshop_item_installed(libraries: &[PathBuf], branch: Branch) -> bool {
    libraries.iter().any(|library| {
        let manifest = library.join("steamapps/workshop/appworkshop_108600.acf");
        fs::read_to_string(manifest)
            .ok()
            .and_then(|input| workshop_manifest_contains_item(&input, branch).ok())
            .unwrap_or(false)
    })
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
        let result = discover_from(steam).unwrap();
        assert_eq!(
            result.game_path,
            library.join("steamapps/common/ProjectZomboid")
        );
        assert!(result.branches[0].ready);
        assert!(!result.branches[1].ready);
    }
}
