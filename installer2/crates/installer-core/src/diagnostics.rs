use crate::{
    discovery::{self, Discovery},
    patch, platform, transaction,
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticAction {
    Steam,
    Json,
    Launcher,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticReport {
    pub discovery: Discovery,
    pub steam_running: bool,
}

pub fn diagnostic_report() -> Result<DiagnosticReport> {
    Ok(DiagnosticReport {
        discovery: discovery::discover()?,
        steam_running: platform::steam_running(),
    })
}

pub fn run_diagnostic_action(action: DiagnosticAction) -> Result<String> {
    let discovery = discovery::discover()?;
    run_diagnostic_action_at(action, &discovery.game_path, &discovery.config_paths)
}

/// Run one diagnostics patch against explicit targets, without consulting host Steam paths.
pub fn run_diagnostic_action_at(
    action: DiagnosticAction,
    game_path: &Path,
    config_paths: &[std::path::PathBuf],
) -> Result<String> {
    match action {
        DiagnosticAction::Steam => patch_steam(config_paths),
        DiagnosticAction::Json => patch_one(
            &game_path.join("ProjectZomboid64.json"),
            patch::launcher_json,
        ),
        DiagnosticAction::Launcher => {
            patch_one(&game_path.join("ProjectZomboid64.bat"), patch::launcher_bat)
        }
    }
}

fn patch_one(path: &Path, transform: fn(&[u8]) -> Result<Vec<u8>>) -> Result<String> {
    let input = fs::read(path).with_context(|| format!("Could not read {}", path.display()))?;
    let output = transform(&input)?;
    backup_once(path, &input)?;
    transaction::atomic_write(path, &output)?;
    Ok(format!("Updated {}", path.display()))
}

fn patch_steam(config_paths: &[std::path::PathBuf]) -> Result<String> {
    let mut patched = 0;
    for path in config_paths {
        let input = fs::read(path).with_context(|| format!("Could not read {}", path.display()))?;
        let Some(output) = patch::steam_config(&input)? else {
            continue;
        };
        backup_once(path, &input)?;
        transaction::atomic_write(path, &output)?;
        patched += 1;
    }
    ensure!(
        patched > 0,
        "Project Zomboid app 108600 was not found in Steam user configurations"
    );
    Ok(format!(
        "Updated launch options in {patched} Steam configuration(s)"
    ))
}

fn backup_once(path: &Path, original: &[u8]) -> Result<()> {
    let backup = path.with_extension(format!(
        "{}.backup",
        path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default()
    ));
    if !backup.exists() {
        transaction::atomic_write(&backup, original)?;
    }
    Ok(())
}
