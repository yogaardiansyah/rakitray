use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::models::{CatalogEntry, CatalogItem, Platform};
use crate::store::StoreError;

pub const DEFAULT_CATALOG_JSON: &str = include_str!("catalog.json");

pub const TEMPLATE_MARIADB: &str = include_str!("../blueprint/templates/mariadb.toml");
pub const TEMPLATE_NGINX: &str = include_str!("../blueprint/templates/nginx.toml");
pub const TEMPLATE_CADDY: &str = include_str!("../blueprint/templates/caddy.toml");
pub const TEMPLATE_PHP_CGI: &str = include_str!("../blueprint/templates/php_cgi.toml");
pub const TEMPLATE_POSTGRES: &str = include_str!("../blueprint/templates/postgres.toml");
pub const TEMPLATE_REDIS: &str = include_str!("../blueprint/templates/redis.toml");
pub const TEMPLATE_MAILPIT: &str = include_str!("../blueprint/templates/mailpit.toml");

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallReceipt {
    pub catalog_id: String,
    pub engine: String,
    pub version: String,
    pub sha256: String,
    pub installed_at: u64,
    pub stripped_bytes: u64,
    pub install_path: String,
}

/// Detects the host platform from target OS and architecture.
pub fn detect_host_platform() -> Result<Platform, StoreError> {
    let os = env::consts::OS;
    let arch = env::consts::ARCH;

    match (os, arch) {
        ("linux", "x86_64") => Ok(Platform::LinuxX64),
        ("linux", "aarch64") => Ok(Platform::LinuxArm64),
        ("windows", "x86_64") => Ok(Platform::WindowsX64),
        ("macos", "x86_64") => Ok(Platform::MacosX64),
        ("macos", "aarch64") => Ok(Platform::MacosArm64),
        _ => Err(StoreError::Extraction(format!(
            "Unsupported platform architecture: {}-{}",
            os, arch
        ))),
    }
}

pub struct CatalogManager;

impl CatalogManager {
    /// Gets embedded blueprint TOML template by blueprint_id.
    pub fn get_blueprint_template(blueprint_id: &str) -> Option<&'static str> {
        match blueprint_id {
            "mariadb" => Some(TEMPLATE_MARIADB),
            "nginx" => Some(TEMPLATE_NGINX),
            "caddy" => Some(TEMPLATE_CADDY),
            "php_cgi" | "php" => Some(TEMPLATE_PHP_CGI),
            "postgres" => Some(TEMPLATE_POSTGRES),
            "redis" => Some(TEMPLATE_REDIS),
            "mailpit" => Some(TEMPLATE_MAILPIT),
            _ => None,
        }
    }

    /// Writes `rakit-manifest.toml` into destination bin directory.
    pub fn write_blueprint_to_bin(dest_dir: &Path, blueprint_id: &str) -> Result<(), StoreError> {
        let content = Self::get_blueprint_template(blueprint_id).ok_or_else(|| {
            StoreError::Extraction(format!("Unknown blueprint template id: {}", blueprint_id))
        })?;

        let manifest_path = dest_dir.join("rakit-manifest.toml");
        let mut file = File::create(&manifest_path).map_err(StoreError::Io)?;
        file.write_all(content.as_bytes()).map_err(StoreError::Io)?;
        file.flush().map_err(StoreError::Io)?;
        Ok(())
    }

    /// Loads the catalog entries, falling back to embedded defaults.
    pub fn load_catalog(runtime_root: &Path) -> Result<Vec<CatalogEntry>, StoreError> {
        let cached_path = runtime_root.join("store/catalog.json");
        let json_str = if cached_path.exists() {
            fs::read_to_string(&cached_path).unwrap_or_else(|_| DEFAULT_CATALOG_JSON.to_string())
        } else {
            DEFAULT_CATALOG_JSON.to_string()
        };

        serde_json::from_str(&json_str)
            .map_err(|e| StoreError::Extraction(format!("Failed to parse catalog JSON: {}", e)))
    }

    /// Loads installed receipts mapping `catalog_id` -> `InstallReceipt`.
    pub fn load_installed_receipts(runtime_root: &Path) -> HashMap<String, InstallReceipt> {
        let receipts_path = runtime_root.join("store/installed.json");
        if !receipts_path.exists() {
            return HashMap::new();
        }

        let content = match fs::read_to_string(&receipts_path) {
            Ok(c) => c,
            Err(_) => return HashMap::new(),
        };

        serde_json::from_str(&content).unwrap_or_default()
    }

    /// Records an installed receipt.
    pub fn record_install(runtime_root: &Path, receipt: InstallReceipt) -> Result<(), StoreError> {
        let store_dir = runtime_root.join("store");
        fs::create_dir_all(&store_dir).map_err(StoreError::Io)?;

        let mut receipts = Self::load_installed_receipts(runtime_root);
        receipts.insert(receipt.catalog_id.clone(), receipt);

        let receipts_path = store_dir.join("installed.json");
        let json = serde_json::to_string_pretty(&receipts)
            .map_err(|e| StoreError::Extraction(e.to_string()))?;
        fs::write(&receipts_path, json).map_err(StoreError::Io)?;
        Ok(())
    }

    /// Removes an installed receipt.
    pub fn record_uninstall(runtime_root: &Path, catalog_id: &str) -> Result<(), StoreError> {
        let store_dir = runtime_root.join("store");
        let mut receipts = Self::load_installed_receipts(runtime_root);
        receipts.remove(catalog_id);

        let receipts_path = store_dir.join("installed.json");
        let json = serde_json::to_string_pretty(&receipts)
            .map_err(|e| StoreError::Extraction(e.to_string()))?;
        fs::write(&receipts_path, json).map_err(StoreError::Io)?;
        Ok(())
    }

    /// Returns catalog items filtered for host platform with installed state.
    pub fn list_catalog_items(
        runtime_root: &Path,
        host_platform: Platform,
    ) -> Result<Vec<CatalogItem>, StoreError> {
        let catalog = Self::load_catalog(runtime_root)?;
        let receipts = Self::load_installed_receipts(runtime_root);

        let items = catalog
            .into_iter()
            .filter(|entry| entry.platform == host_platform)
            .map(|entry| {
                let receipt = receipts.get(&entry.id);
                let (installed, install_path) = match receipt {
                    Some(r) => {
                        let path = PathBuf::from(&r.install_path);
                        if path.exists() && path.join("rakit-manifest.toml").exists() {
                            (true, Some(r.install_path.clone()))
                        } else {
                            (false, None)
                        }
                    }
                    None => (false, None),
                };

                CatalogItem {
                    entry,
                    installed,
                    install_path,
                }
            })
            .collect();

        Ok(items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_detection() {
        let res = detect_host_platform();
        assert!(res.is_ok());
    }

    #[test]
    fn test_embedded_catalog_valid_json_and_https() {
        let entries: Vec<CatalogEntry> = serde_json::from_str(DEFAULT_CATALOG_JSON).unwrap();
        assert!(!entries.is_empty());

        for entry in &entries {
            assert!(
                entry.download_url.starts_with("https://"),
                "Entry {} must use HTTPS",
                entry.id
            );
            assert_eq!(
                entry.sha256.len(),
                64,
                "SHA256 must be 64 hex chars for {}",
                entry.id
            );
            assert!(
                CatalogManager::get_blueprint_template(&entry.blueprint_id).is_some(),
                "Blueprint {} must exist for entry {}",
                entry.blueprint_id,
                entry.id
            );
        }
    }

    #[test]
    fn test_all_blueprint_templates_parse_and_validate() {
        let templates = [
            TEMPLATE_MARIADB,
            TEMPLATE_NGINX,
            TEMPLATE_CADDY,
            TEMPLATE_PHP_CGI,
            TEMPLATE_POSTGRES,
            TEMPLATE_REDIS,
            TEMPLATE_MAILPIT,
        ];

        for tpl in templates {
            let blueprint = crate::blueprint::parse_blueprint(tpl)
                .expect("Template must pass syntax and variable validation");
            assert!(!blueprint.engine.name.is_empty());
            assert!(!blueprint.binaries.daemon.is_empty());
        }
    }

    #[test]
    fn test_catalog_filtering_and_receipts() {
        let temp_dir = std::env::temp_dir().join("rakit_test_catalog_receipts");
        let _ = fs::remove_dir_all(&temp_dir);
        let _ = fs::create_dir_all(&temp_dir);

        let items = CatalogManager::list_catalog_items(&temp_dir, Platform::LinuxX64).unwrap();
        assert!(!items.is_empty());
        assert!(items.iter().all(|i| i.entry.platform == Platform::LinuxX64));
        assert!(items.iter().all(|i| !i.installed));

        // Create a mock install receipt with a valid rakit-manifest.toml
        let mock_bin = temp_dir.join("bin/database/mariadb/11.4.3");
        fs::create_dir_all(&mock_bin).unwrap();
        CatalogManager::write_blueprint_to_bin(&mock_bin, "mariadb").unwrap();
        assert!(mock_bin.join("rakit-manifest.toml").exists());

        let receipt = InstallReceipt {
            catalog_id: "mariadb@11.4.3".to_string(),
            engine: "mariadb".to_string(),
            version: "11.4.3".to_string(),
            sha256: "fake".to_string(),
            installed_at: 12345678,
            stripped_bytes: 50000000,
            install_path: mock_bin.to_string_lossy().to_string(),
        };

        CatalogManager::record_install(&temp_dir, receipt).unwrap();

        let updated_items =
            CatalogManager::list_catalog_items(&temp_dir, Platform::LinuxX64).unwrap();
        let mariadb = updated_items
            .iter()
            .find(|i| i.entry.id == "mariadb@11.4.3")
            .unwrap();
        assert!(mariadb.installed);
        assert!(mariadb.install_path.is_some());

        // Test uninstall record
        CatalogManager::record_uninstall(&temp_dir, "mariadb@11.4.3").unwrap();
        let uninstalled_items =
            CatalogManager::list_catalog_items(&temp_dir, Platform::LinuxX64).unwrap();
        let mariadb_after = uninstalled_items
            .iter()
            .find(|i| i.entry.id == "mariadb@11.4.3")
            .unwrap();
        assert!(!mariadb_after.installed);

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
