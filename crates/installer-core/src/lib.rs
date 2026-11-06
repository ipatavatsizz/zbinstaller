mod diagnostics;
mod discovery;
mod install;
mod lock;
mod patch;
mod platform;
mod target;
mod transaction;
mod vdf;
pub use lock::InstallationLock;
pub use target::TargetPlatform;

pub use diagnostics::{
    DiagnosticAction, DiagnosticReport, diagnostic_report, run_diagnostic_action,
    run_diagnostic_action_at, run_diagnostic_action_for,
};
pub use discovery::{
    Discovery, FileInfo, discover, discover_from, discover_from_for, inspect, inspect_from,
    workshop_manifest_contains_item,
};
pub use install::{
    Progress, install, install_from_paths, install_from_paths_for, install_with_cancel, uninstall,
    uninstall_from_paths, uninstall_from_paths_for,
};
pub use patch::{
    launcher_bat, launcher_json, launcher_json_for, launcher_shell, steam_config, steam_config_for,
};
pub use platform::{open_workshop, steam_candidates};

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Branch {
    Lts,
    Beta,
}

impl Branch {
    pub fn workshop_id(self) -> &'static str {
        match self {
            Self::Lts => "3619862853",
            Self::Beta => "3812624292",
        }
    }
}
