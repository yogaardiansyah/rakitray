use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

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

/// Shareable project stack manifest (`.rakitray.toml`), analogous to `docker-compose.yml`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectManifest {
    pub schema_version: u32,
    pub name: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
    #[serde(default)]
    pub services: BTreeMap<String, ServiceSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServiceSpec {
    pub version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
    /// Generic per-service config overrides (e.g. `php.ini`, `my.cnf`, `redis.conf`, `nginx.conf`).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub config: BTreeMap<String, String>,
}

impl ProjectManifest {
    pub fn load(path: &Path) -> Result<Self, String> {
        let raw = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
        toml::from_str(&raw).map_err(|e| format!("Invalid {}: {}", path.display(), e))
    }

    pub fn to_toml_string(&self) -> Result<String, String> {
        toml::to_string_pretty(self).map_err(|e| format!("Failed to serialize manifest: {}", e))
    }
}

/// Renders per-service config overrides using engine-appropriate syntax.
pub fn render_service_config(engine: &str, config: &BTreeMap<String, String>) -> String {
    let lines: Vec<String> = config
        .iter()
        .map(|(k, v)| match engine {
            "redis" | "caddy" => format!("{} {}", k, v),
            "nginx" => format!("{} {};", k, v),
            _ => format!("{} = {}", k, v),
        })
        .collect();
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_project_manifest_with_ports_and_configs() {
        let mut services = BTreeMap::new();
        services.insert(
            "php".to_string(),
            ServiceSpec {
                version: "8.3.12".to_string(),
                port: Some(9000),
                env: BTreeMap::new(),
                config: BTreeMap::from([
                    ("memory_limit".to_string(), "512M".to_string()),
                    ("upload_max_filesize".to_string(), "64M".to_string()),
                ]),
            },
        );
        services.insert(
            "mariadb".to_string(),
            ServiceSpec {
                version: "11.4.3".to_string(),
                port: Some(3307),
                env: BTreeMap::from([("MYSQL_DATABASE".to_string(), "app".to_string())]),
                config: BTreeMap::from([("max_connections".to_string(), "200".to_string())]),
            },
        );

        let manifest = ProjectManifest {
            schema_version: 1,
            name: "laravel-app".to_string(),
            env: BTreeMap::from([("APP_ENV".to_string(), "local".to_string())]),
            services,
        };

        let toml_str = manifest.to_toml_string().unwrap();
        let parsed: ProjectManifest = toml::from_str(&toml_str).unwrap();
        assert_eq!(manifest, parsed);
    }

    #[test]
    fn render_service_config_respects_engine_syntax() {
        let cfg = BTreeMap::from([("maxmemory".to_string(), "256mb".to_string())]);
        assert_eq!(render_service_config("redis", &cfg), "maxmemory 256mb");
        assert_eq!(render_service_config("nginx", &cfg), "maxmemory 256mb;");
        assert_eq!(render_service_config("php", &cfg), "maxmemory = 256mb");
    }
}
