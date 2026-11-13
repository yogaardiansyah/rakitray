use crate::models::{ExtensionCategory, ExtensionKind, ExtensionSource, PhpExtension};
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanOutcome {
    pub extensions: Vec<PhpExtension>,
    pub warnings: Vec<String>,
}

pub fn normalize_ext_name(raw: &str) -> String {
    let trimmed = raw.trim();
    let lower = trimmed.to_lowercase();
    if lower == "zend opcache" {
        "opcache".to_string()
    } else {
        lower
    }
}

pub fn parse_shared_lib_name(file_name: &str) -> Option<String> {
    let lower = file_name.to_lowercase();
    let base = if let Some(stripped) = lower.strip_suffix(".dll") {
        stripped
    } else {
        lower.strip_suffix(".so")?
    };

    let name = if let Some(stripped) = base.strip_prefix("php_") {
        stripped
    } else {
        base
    };

    if name.is_empty() {
        None
    } else {
        Some(normalize_ext_name(name))
    }
}

pub fn categorize_extension(name: &str) -> ExtensionCategory {
    match name {
        "pdo" | "pdo_mysql" | "mbstring" | "openssl" | "curl" | "fileinfo" | "bcmath"
        | "tokenizer" | "xml" | "dom" | "zip" | "ctype" | "filter" | "hash" | "session"
        | "pcre" => ExtensionCategory::LaravelCore,

        "redis" | "sqlite3" | "pdo_sqlite" | "pgsql" | "pdo_pgsql" | "memcached" | "mysqli"
        | "mysqlnd" | "mongodb" => ExtensionCategory::DatabaseCache,

        "opcache" | "zend opcache" | "xdebug" | "pcov" => ExtensionCategory::PerformanceDebug,

        "gd" | "intl" | "exif" | "imagick" => ExtensionCategory::MediaFormat,

        _ => ExtensionCategory::Other,
    }
}

pub fn extension_kind(name: &str) -> ExtensionKind {
    match name {
        "opcache" | "zend opcache" | "xdebug" | "pcov" => ExtensionKind::ZendExtension,
        _ => ExtensionKind::Extension,
    }
}

pub fn extension_dependencies(name: &str) -> Vec<String> {
    match name {
        "exif" => vec!["mbstring".to_string()],
        "pdo_mysql" | "pdo_pgsql" | "pdo_sqlite" => vec!["pdo".to_string()],
        _ => vec![],
    }
}

fn category_order(cat: &ExtensionCategory) -> u8 {
    match cat {
        ExtensionCategory::LaravelCore => 0,
        ExtensionCategory::DatabaseCache => 1,
        ExtensionCategory::PerformanceDebug => 2,
        ExtensionCategory::MediaFormat => 3,
        ExtensionCategory::Other => 4,
    }
}

fn find_php_binary(php_root: &Path) -> Option<PathBuf> {
    let candidates = [
        "php",
        "php.exe",
        "bin/php",
        "bin/php.exe",
        "php-cgi",
        "php-cgi.exe",
        "bin/php-cgi",
        "bin/php-cgi.exe",
    ];

    for candidate in candidates {
        let p = php_root.join(candidate);
        if p.exists() {
            return Some(p);
        }
    }
    None
}

pub async fn scan(php_root: &Path, enabled_exts: &HashSet<String>) -> ScanOutcome {
    let mut warnings = Vec::new();
    let mut shared_exts = HashSet::new();

    let ext_dir = if php_root.join("ext").is_dir() {
        Some(php_root.join("ext"))
    } else if php_root.join("lib/php/extensions").is_dir() {
        Some(php_root.join("lib/php/extensions"))
    } else {
        None
    };

    if let Some(ref dir) = ext_dir {
        match tokio::fs::read_dir(dir).await {
            Ok(mut reader) => {
                while let Ok(Some(entry)) = reader.next_entry().await {
                    if let Ok(file_type) = entry.file_type().await {
                        if file_type.is_file() || file_type.is_symlink() {
                            let fname = entry.file_name();
                            let fname_str = fname.to_string_lossy();
                            if let Some(norm) = parse_shared_lib_name(&fname_str) {
                                shared_exts.insert(norm);
                            }
                        }
                    }
                }
            }
            Err(e) => {
                warnings.push(format!(
                    "Failed to read extension directory at '{}': {e}",
                    dir.display()
                ));
            }
        }
    } else {
        warnings.push(format!(
            "No 'ext' folder found under '{}'",
            php_root.display()
        ));
    }

    let mut builtin_exts = HashSet::new();
    if let Some(php_bin) = find_php_binary(php_root) {
        let mut cmd = Command::new(&php_bin);
        cmd.args(["-n", "-m"]);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        cmd.kill_on_drop(true);

        let timeout_fut = tokio::time::timeout(Duration::from_secs(5), cmd.output());
        match timeout_fut.await {
            Ok(Ok(output)) => {
                if output.status.success() {
                    let stdout_str = String::from_utf8_lossy(&output.stdout);
                    for line in stdout_str.lines() {
                        let trimmed = line.trim();
                        if trimmed.is_empty() || trimmed.starts_with('[') {
                            continue;
                        }
                        let norm = normalize_ext_name(trimmed);
                        builtin_exts.insert(norm);
                    }
                } else {
                    let err = String::from_utf8_lossy(&output.stderr);
                    warnings.push(format!(
                        "php -m exited with status {}: {}",
                        output.status,
                        err.trim()
                    ));
                }
            }
            Ok(Err(e)) => {
                warnings.push(format!(
                    "Failed to execute '{} -n -m': {e}",
                    php_bin.display()
                ));
            }
            Err(_) => {
                warnings.push(format!(
                    "Command '{} -n -m' timed out after 5s",
                    php_bin.display()
                ));
            }
        }
    }

    let mut all_exts: BTreeMap<String, PhpExtension> = BTreeMap::new();

    for name in &shared_exts {
        let category = categorize_extension(name);
        let kind = extension_kind(name);
        let requires = extension_dependencies(name);
        let enabled = enabled_exts.contains(name);

        all_exts.insert(
            name.clone(),
            PhpExtension {
                name: name.clone(),
                kind,
                source: ExtensionSource::SharedLib,
                category,
                enabled,
                toggleable: true,
                requires,
            },
        );
    }

    for name in &builtin_exts {
        if !all_exts.contains_key(name) {
            let category = categorize_extension(name);
            let kind = extension_kind(name);
            let requires = extension_dependencies(name);

            all_exts.insert(
                name.clone(),
                PhpExtension {
                    name: name.clone(),
                    kind,
                    source: ExtensionSource::BuiltIn,
                    category,
                    enabled: true,
                    toggleable: false,
                    requires,
                },
            );
        }
    }

    let mut extensions: Vec<PhpExtension> = all_exts.into_values().collect();
    extensions.sort_by(|a, b| {
        let ord_a = category_order(&a.category);
        let ord_b = category_order(&b.category);
        ord_a.cmp(&ord_b).then_with(|| a.name.cmp(&b.name))
    });

    ScanOutcome {
        extensions,
        warnings,
    }
}

pub fn parse_enabled_from_ini(content: &str) -> HashSet<String> {
    let mut enabled = HashSet::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty()
            || trimmed.starts_with(';')
            || trimmed.starts_with('#')
            || trimmed.starts_with('[')
        {
            continue;
        }

        if let Some((directive, raw_val)) = trimmed.split_once('=') {
            let d = directive.trim();
            if d == "extension" || d == "zend_extension" {
                let v = raw_val.trim().trim_matches('"').trim_matches('\'');
                let file_part = Path::new(v)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(v);

                if let Some(parsed) = parse_shared_lib_name(file_part) {
                    enabled.insert(parsed);
                } else {
                    let cleaned = file_part.strip_prefix("php_").unwrap_or(file_part);
                    enabled.insert(normalize_ext_name(cleaned));
                }
            }
        }
    }

    enabled
}

pub fn parse_enabled_from_matrix_toml(content: &str) -> HashSet<String> {
    let mut enabled = HashSet::new();
    if let Ok(value) = toml::from_str::<toml::Value>(content) {
        if let Some(arr) = value.get("enabled_extensions").and_then(|v| v.as_array()) {
            for item in arr {
                if let Some(s) = item.as_str() {
                    enabled.insert(normalize_ext_name(s));
                }
            }
        }
        if let Some(arr) = value.get("extensions").and_then(|v| v.as_array()) {
            for item in arr {
                if let Some(s) = item.as_str() {
                    enabled.insert(normalize_ext_name(s));
                }
            }
        }
        if let Some(table) = value.get("extensions").and_then(|v| v.as_table()) {
            for (k, v) in table {
                if v.as_bool() == Some(true) {
                    enabled.insert(normalize_ext_name(k));
                }
            }
        }
    }
    enabled
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TestTempDir(PathBuf);

    impl TestTempDir {
        fn new(prefix: &str) -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "rakitray_test_{prefix}_{}_{}",
                std::process::id(),
                nanos
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestTempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[tokio::test]
    async fn scan_windows_dll_fixture() {
        let dir = TestTempDir::new("win");
        let ext_dir = dir.path().join("ext");
        fs::create_dir_all(&ext_dir).unwrap();

        fs::write(ext_dir.join("php_mbstring.dll"), b"").unwrap();
        fs::write(ext_dir.join("php_exif.dll"), b"").unwrap();
        fs::write(ext_dir.join("php_xdebug.dll"), b"").unwrap();
        fs::write(ext_dir.join("php_redis.dll"), b"").unwrap();
        fs::write(ext_dir.join("php_curl.dll"), b"").unwrap();

        let mut enabled = HashSet::new();
        enabled.insert("mbstring".to_string());
        enabled.insert("redis".to_string());

        let outcome = scan(dir.path(), &enabled).await;

        let find_ext = |n: &str| {
            outcome
                .extensions
                .iter()
                .find(|e| e.name == n)
                .cloned()
                .unwrap()
        };

        let mb = find_ext("mbstring");
        assert_eq!(mb.source, ExtensionSource::SharedLib);
        assert_eq!(mb.category, ExtensionCategory::LaravelCore);
        assert!(mb.enabled);
        assert!(mb.toggleable);

        let exif = find_ext("exif");
        assert_eq!(exif.category, ExtensionCategory::MediaFormat);
        assert_eq!(exif.requires, vec!["mbstring".to_string()]);
        assert!(!exif.enabled);

        let xdebug = find_ext("xdebug");
        assert_eq!(xdebug.kind, ExtensionKind::ZendExtension);
        assert_eq!(xdebug.category, ExtensionCategory::PerformanceDebug);

        let redis = find_ext("redis");
        assert_eq!(redis.category, ExtensionCategory::DatabaseCache);
        assert!(redis.enabled);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn scan_linux_so_and_builtin_stub_fixture() {
        use std::os::unix::fs::PermissionsExt;

        let dir = TestTempDir::new("unix");
        let ext_dir = dir.path().join("ext");
        let bin_dir = dir.path().join("bin");
        fs::create_dir_all(&ext_dir).unwrap();
        fs::create_dir_all(&bin_dir).unwrap();

        fs::write(ext_dir.join("redis.so"), b"").unwrap();

        let php_script = bin_dir.join("php");
        fs::write(
            &php_script,
            b"#!/bin/sh\ncat << 'EOF'\n[PHP Modules]\nctype\nmbstring\n[Zend Modules]\nZend OPcache\nEOF\n",
        )
        .unwrap();

        let mut perms = fs::metadata(&php_script).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&php_script, perms).unwrap();

        let mut enabled = HashSet::new();
        enabled.insert("redis".to_string());

        let outcome = scan(dir.path(), &enabled).await;

        let redis = outcome
            .extensions
            .iter()
            .find(|e| e.name == "redis")
            .unwrap();
        assert_eq!(redis.source, ExtensionSource::SharedLib);
        assert!(redis.toggleable);
        assert!(redis.enabled);

        let ctype = outcome
            .extensions
            .iter()
            .find(|e| e.name == "ctype")
            .unwrap();
        assert_eq!(ctype.source, ExtensionSource::BuiltIn);
        assert!(!ctype.toggleable);
        assert!(ctype.enabled);
        assert_eq!(ctype.category, ExtensionCategory::LaravelCore);

        let opcache = outcome
            .extensions
            .iter()
            .find(|e| e.name == "opcache")
            .unwrap();
        assert_eq!(opcache.kind, ExtensionKind::ZendExtension);
        assert_eq!(opcache.source, ExtensionSource::BuiltIn);
        assert!(!opcache.toggleable);
        assert!(opcache.enabled);
    }

    #[tokio::test]
    async fn scan_missing_ext_dir_does_not_panic() {
        let dir = TestTempDir::new("missing");
        let outcome = scan(dir.path(), &HashSet::new()).await;
        assert!(outcome.extensions.is_empty());
        assert_eq!(outcome.warnings.len(), 1);
        assert!(outcome.warnings[0].contains("No 'ext' folder found"));
    }

    #[test]
    fn parse_enabled_from_ini_and_toml_test() {
        let ini = r#"
; comment
extension=mbstring
extension = "php_curl.dll"
zend_extension = /usr/local/lib/xdebug.so
# another comment
extension=redis.so
;extension=disabled_ext
"#;
        let set = parse_enabled_from_ini(ini);
        assert!(set.contains("mbstring"));
        assert!(set.contains("curl"));
        assert!(set.contains("xdebug"));
        assert!(set.contains("redis"));
        assert!(!set.contains("disabled_ext"));

        let toml_str = r#"
enabled_extensions = ["pdo_mysql", "zip"]
[extensions]
bcmath = true
gd = false
"#;
        let set_toml = parse_enabled_from_matrix_toml(toml_str);
        assert!(set_toml.contains("pdo_mysql"));
        assert!(set_toml.contains("zip"));
        assert!(set_toml.contains("bcmath"));
        assert!(!set_toml.contains("gd"));
    }
}
