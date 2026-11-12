use super::service::{ServiceCategory, ServiceStatus};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceInstance {
    pub instance_id: String,
    pub engine: String,
    pub display_name: String,
    pub category: ServiceCategory,
    pub version: String,
    pub profile: String,
    pub status: ServiceStatus,
    pub pid: Option<u32>,
    pub port: Option<u16>,
    pub socket_path: Option<String>,
    pub memory_bytes: u64,
    pub cpu_percent: f32,
    pub uptime_secs: u64,
    pub pinned_to_rack: bool,
    pub last_error: Option<String>,
}
