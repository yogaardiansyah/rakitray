pub mod php_ext;

use crate::models::{DiscoveredRuntime, ServiceCategory};
use std::path::{Path, PathBuf};

/// Resolves the canonical runtime root directory.
/// In dev, guarantees resolving to the workspace root (`<repo>/runtime`),
/// never inside `src-tauri/runtime`.
pub fn resolve_runtime_root() -> PathBuf {
    if let Ok(env_val) = std::env::var("RAKITRAY_RUNTIME_DIR") {
        let p = PathBuf::from(env_val);
        if p.exists() {
            return p;
        }
    }

    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let manifest_path = PathBuf::from(manifest_dir);
        let parent_runtime = manifest_path.join("../runtime");
        if parent_runtime.exists() {
            if let Ok(canonical) = parent_runtime.canonicalize() {
                return canonical;
            }
            return parent_runtime;
        }
    }

    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    if cwd.file_name().map(|n| n == "src-tauri").unwrap_or(false) {
        let parent_runtime = cwd.join("../runtime");
        if let Ok(canonical) = parent_runtime.canonicalize() {
            return canonical;
        }
        return parent_runtime;
    }

    let local_runtime = cwd.join("runtime");
    if local_runtime.exists() {
        if let Ok(canonical) = local_runtime.canonicalize() {
            return canonical;
        }
        return local_runtime;
    }

    cwd.join("runtime")
}

pub fn scan_runtimes(runtime_root: &Path) -> Vec<DiscoveredRuntime> {
    let mut runtimes = Vec::new();
    let bin_dir = runtime_root.join("bin");

    if bin_dir.exists() && bin_dir.is_dir() {
        if let Ok(categories) = std::fs::read_dir(&bin_dir) {
            for cat_entry in categories.flatten() {
                let cat_path = cat_entry.path();
                if !cat_path.is_dir() {
                    continue;
                }
                let cat_name = cat_entry.file_name().to_string_lossy().to_string();
                let category = match cat_name.as_str() {
                    "web_server" | "webserver" => ServiceCategory::WebServer,
                    "runtime" => ServiceCategory::Runtime,
                    "database" => ServiceCategory::Database,
                    "cache" => ServiceCategory::Cache,
                    _ => ServiceCategory::Tool,
                };

                if let Ok(engines) = std::fs::read_dir(&cat_path) {
                    for eng_entry in engines.flatten() {
                        let eng_path = eng_entry.path();
                        if !eng_path.is_dir() {
                            continue;
                        }
                        let engine = eng_entry.file_name().to_string_lossy().to_string();

                        if let Ok(versions) = std::fs::read_dir(&eng_path) {
                            for ver_entry in versions.flatten() {
                                let ver_path = ver_entry.path();
                                if !ver_path.is_dir() {
                                    continue;
                                }
                                let version = ver_entry.file_name().to_string_lossy().to_string();
                                let manifest_path = ver_path.join("rakit-manifest.toml");
                                let has_manifest = manifest_path.exists()
                                    && std::fs::read_to_string(&manifest_path)
                                        .map(|c| crate::blueprint::parse_blueprint(&c).is_ok())
                                        .unwrap_or(false);

                                let binary_path = find_engine_binary(&engine, &ver_path);

                                runtimes.push(DiscoveredRuntime {
                                    category,
                                    engine: engine.clone(),
                                    version,
                                    is_valid: has_manifest && binary_path.is_some(),
                                    binary_path: binary_path.unwrap_or(ver_path),
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    runtimes
}

fn find_engine_binary(engine: &str, dir: &Path) -> Option<PathBuf> {
    let candidates = match engine {
        "nginx" => vec!["nginx", "nginx.exe", "bin/nginx", "sbin/nginx"],
        "caddy" => vec!["caddy", "caddy.exe"],
        "php-cgi" | "php" => vec![
            "php-cgi",
            "php-cgi.exe",
            "bin/php-cgi",
            "php",
            "bin/php",
            "sbin/php-fpm",
            "bin/php-fpm",
        ],
        "mariadb" => vec![
            "bin/mariadbd",
            "bin/mariadbd.exe",
            "bin/mysqld",
            "mariadbd",
            "mysqld",
        ],
        "redis" => vec!["bin/redis-server", "redis-server", "redis-server.exe"],
        _ => vec![engine],
    };

    for candidate in candidates {
        let p = dir.join(candidate);
        if p.exists() {
            return Some(p);
        }
    }
    None
}
