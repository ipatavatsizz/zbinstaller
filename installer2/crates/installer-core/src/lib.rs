mod diagnostics;
mod discovery;
mod install;
mod patch;
mod platform;
mod transaction;
mod vdf;

pub use diagnostics::{
    DiagnosticAction, DiagnosticReport, diagnostic_report, run_diagnostic_action,
    run_diagnostic_action_at,
};
pub use discovery::{
    Discovery, FileInfo, discover, discover_from, workshop_manifest_contains_item,
};
pub use install::{
    Progress, install, install_from_paths, install_with_cancel, uninstall, uninstall_from_paths,
};
pub use patch::{launcher_bat, launcher_json, steam_config};
pub use platform::open_workshop;

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
