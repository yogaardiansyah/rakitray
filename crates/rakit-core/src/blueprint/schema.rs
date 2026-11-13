use crate::models::ServiceCategory;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServiceBlueprint {
    pub engine: BlueprintEngine,
    pub binaries: BlueprintBinaries,
    pub lifecycle: BlueprintLifecycle,
    pub config: Option<BlueprintConfig>,
    pub hooks: Option<BlueprintHooks>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlueprintEngine {
    pub name: String,
    pub category: ServiceCategory,
    pub version: String,
    pub default_port: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlueprintBinaries {
    pub daemon: String,
    pub client: Option<String>,
    pub installer: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ReadyCheck {
    Tcp { timeout_secs: u64 },
    UnixSocket { timeout_secs: u64 },
    LogRegex { pattern: String, timeout_secs: u64 },
}

impl ReadyCheck {
    pub fn timeout_secs(&self) -> u64 {
        match self {
            Self::Tcp { timeout_secs }
            | Self::UnixSocket { timeout_secs }
            | Self::LogRegex { timeout_secs, .. } => *timeout_secs,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlueprintLifecycle {
    pub bootstrap_check_dir: Option<String>,
    pub bootstrap_cmd: Option<Vec<String>>,
    pub start_args: Vec<String>,
    pub ready_check: ReadyCheck,
    pub stop_signal: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlueprintConfig {
    pub template: String,
    pub output: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlueprintHooks {
    pub post_start: Option<String>,
}
