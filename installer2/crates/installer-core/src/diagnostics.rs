use crate::{
    InstallationLock, TargetPlatform,
    discovery::{self, Discovery},
    install, patch, platform, target, transaction,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

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
        discovery: discovery::inspect(),
        steam_running: platform::steam_running(),
    })
}

pub fn run_diagnostic_action(action: DiagnosticAction) -> Result<String> {
    let root = platform::steam_root()?;
    let _lock = InstallationLock::acquire(&root)?;
    let discovery = discovery::discover_from_for(root, TargetPlatform::current())?;
    install::with_stopped_steam(&discovery, |discovery| {
        run_diagnostic_action_at(action, &discovery.game_path, &discovery.config_paths)
    })
}

/// Explicit fixture paths; the native entry point also checks the game and closes/restarts Steam.
pub fn run_diagnostic_action_at(
    action: DiagnosticAction,
    game: &Path,
    configs: &[PathBuf],
) -> Result<String> {
    run_diagnostic_action_for(action, game, configs, TargetPlatform::current())
}

pub fn run_diagnostic_action_for(
    action: DiagnosticAction,
    game: &Path,
    configs: &[PathBuf],
    target: TargetPlatform,
) -> Result<String> {
    let _lock = InstallationLock::acquire(game)?;
    install::recover_for(game, configs, target)?;
    let mut changes = Vec::new();
    let message = match action {
        DiagnosticAction::Steam => {
            let count = install::add_steam_patches(&mut changes, configs, target)?;
            format!("Updated launch options in {count} Steam configuration(s).")
        }
        DiagnosticAction::Json => {
            install::add_patch(&mut changes, target::launcher_json_path(game)?, |input| {
                patch::launcher_json_for(input, target)
            })?;
            "Updated Project Zomboid JSON launcher.".into()
        }
        DiagnosticAction::Launcher => {
            install::add_patch(&mut changes, target.launcher(game), |input| match target {
                TargetPlatform::Windows => patch::launcher_bat(input),
                _ => patch::launcher_shell(input, target),
            })?;
            "Updated Project Zomboid alternate launcher.".into()
        }
    };
    transaction::apply_at(game, &changes, transaction::atomic_write)?;
    Ok(message)
}
