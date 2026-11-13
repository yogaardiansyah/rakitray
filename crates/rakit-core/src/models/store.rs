use crate::models::{DiscoveredRuntime, ServiceCategory};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Platform {
    LinuxX64,
    LinuxArm64,
    WindowsX64,
    MacosX64,
    MacosArm64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveFormat {
    TarGz,
    TarXz,
    Zip,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogEntry {
    pub id: String,
    pub engine: String,
    pub category: ServiceCategory,
    pub version: String,
    pub platform: Platform,
    pub download_url: String,
    pub sha256: String,
    pub archive_format: ArchiveFormat,
    pub estimated_size_mb: u32,
    pub strip_components: u8,
    pub strip_profile: String,
    pub blueprint_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogItem {
    pub entry: CatalogEntry,
    pub installed: bool,
    pub install_path: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallState {
    Queued,
    Downloading,
    VerifyingSha256,
    ExtractingLean,
    Registering,
    Installed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoreErrorInfo {
    pub code: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallProgress {
    pub install_id: String,
    pub catalog_id: String,
    pub state: InstallState,
    pub bytes_downloaded: u64,
    pub total_bytes: u64,
    pub speed_bps: u64,
    pub eta_secs: Option<u64>,
    pub stripped_bytes: u64,
    pub message: Option<String>,
    pub error: Option<StoreErrorInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "camelCase")]
pub enum StoreEvent {
    Progress(InstallProgress),
    InstalledRuntime(DiscoveredRuntime),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigPatch {
    pub port: Option<u16>,
    pub bind_address: Option<String>,
    pub profile: Option<String>,
    pub tunables: Option<BTreeMap<String, String>>,
    pub restart: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbCreateRequest {
    pub name: String,
    pub charset: Option<String>,
    pub collation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbUserRequest {
    pub username: String,
    pub password: String,
    pub grant_database: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbOverview {
    pub databases: Vec<String>,
    pub users: Vec<String>,
    pub root_password_set: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StackTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub members: Vec<String>,
}
