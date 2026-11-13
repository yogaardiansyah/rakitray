use super::service::ServiceCategory;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredRuntime {
    pub category: ServiceCategory,
    pub engine: String,
    pub version: String,
    pub binary_path: PathBuf,
    pub is_valid: bool,
}
