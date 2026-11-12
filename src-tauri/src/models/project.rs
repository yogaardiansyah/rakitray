use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSite {
    pub name: String,
    pub domain: String,
    pub root_path: String,
    pub web_server_id: String,
    pub runtime_id: String,
    pub port: u16,
}
