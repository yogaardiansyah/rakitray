use rakitray_lib::commands::php_matrix::{
    apply_laravel_preset_with_diff, apply_php_profile_inner, clone_php_matrix_inner,
    get_php_matrix_inner, toggle_php_extension_inner, RakitrayError,
};
use rakitray_lib::models::{
    ExtensionCategory, ExtensionKind, ExtensionSource, PhpIniLimits, PhpProfileMode,
    ServiceCategory, ServiceInstance, ServiceStatus,
};
use rakitray_lib::supervisor::Supervisor;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

struct TestFixture {
    root: PathBuf,
    supervisor: Arc<Supervisor>,
}

impl TestFixture {
    async fn setup(name: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("rakit_e2e_{name}_{}_{}", std::process::id(), nanos));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();

        // 1. Setup PHP 8.3 runtime hierarchy
        let php83_dir = root.join("bin/runtime/php/8.3.14");
        let php83_ext = php83_dir.join("ext");
        let php83_bin = php83_dir.join("bin");
        fs::create_dir_all(&php83_ext).unwrap();
        fs::create_dir_all(&php83_bin).unwrap();

        for ext in &[
            "php_mbstring.dll",
            "php_exif.dll",
            "php_curl.dll",
            "php_pdo_mysql.dll",
            "php_zip.dll",
            "php_bcmath.dll",
            "php_intl.dll",
            "php_gd.dll",
            "php_redis.dll",
            "php_opcache.dll",
            "php_broken_ext.dll",
        ] {
            fs::write(php83_ext.join(ext), b"").unwrap();
        }

        Self::create_stub_php_binary(&php83_bin);
        fs::write(
            php83_dir.join("rakit-manifest.toml"),
            rakitray_lib::store::catalog::TEMPLATE_PHP_CGI,
        )
        .unwrap();

        // 2. Setup PHP 8.2 runtime hierarchy
        let php82_dir = root.join("bin/runtime/php/8.2.20");
        let php82_ext = php82_dir.join("ext");
        let php82_bin = php82_dir.join("bin");
        fs::create_dir_all(&php82_ext).unwrap();
        fs::create_dir_all(&php82_bin).unwrap();

        for ext in &["php_mbstring.dll", "php_curl.dll", "php_pdo_mysql.dll"] {
            fs::write(php82_ext.join(ext), b"").unwrap();
        }

        Self::create_stub_php_binary(&php82_bin);
        fs::write(
            php82_dir.join("rakit-manifest.toml"),
            rakitray_lib::store::catalog::TEMPLATE_PHP_CGI,
        )
        .unwrap();

        let supervisor = Arc::new(Supervisor::with_debounce(
            root.clone(),
            Duration::from_millis(10),
        ));

        // Register fake instances
        {
            let mut instances = supervisor.instances.write().await;
            instances.insert(
                "php_83".to_string(),
                ServiceInstance {
                    instance_id: "php_83".to_string(),
                    engine: "php".to_string(),
                    display_name: "PHP 8.3".to_string(),
                    category: ServiceCategory::Runtime,
                    version: "8.3.14".to_string(),
                    profile: "dev".to_string(),
                    status: ServiceStatus::Stopped,
                    pid: None,
                    port: Some(9000),
                    socket_path: None,
                    memory_bytes: 0,
                    cpu_percent: 0.0,
                    uptime_secs: 0,
                    pinned_to_rack: false,
                    last_error: None,
                },
            );
            instances.insert(
                "php_82".to_string(),
                ServiceInstance {
                    instance_id: "php_82".to_string(),
                    engine: "php".to_string(),
                    display_name: "PHP 8.2".to_string(),
                    category: ServiceCategory::Runtime,
                    version: "8.2.20".to_string(),
                    profile: "dev".to_string(),
                    status: ServiceStatus::Stopped,
                    pid: None,
                    port: Some(9001),
                    socket_path: None,
                    memory_bytes: 0,
                    cpu_percent: 0.0,
                    uptime_secs: 0,
                    pinned_to_rack: false,
                    last_error: None,
                },
            );
        }

        Self { root, supervisor }
    }

    fn create_stub_php_binary(bin_dir: &Path) {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let php_path = bin_dir.join("php");
            let script = r#"#!/bin/sh
if [ "$1" = "-n" ] && [ "$2" = "-m" ]; then
  cat << 'EOF'
[PHP Modules]
ctype
dom
fileinfo
filter
hash
json
pcre
pdo
session
tokenizer
xml
[Zend Modules]
Zend OPcache
EOF
  exit 0
fi

if [ "$1" = "-n" ] && [ "$2" = "-c" ]; then
  if grep -q "broken_ext" "$3"; then
    echo "PHP Fatal error: Unable to load dynamic library broken_ext" >&2
    exit 1
  fi
  exit 0
fi

exit 0
"#;
            fs::write(&php_path, script).unwrap();
            let mut perms = fs::metadata(&php_path).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&php_path, perms).unwrap();
        }

        #[cfg(windows)]
        {
            let php_path = bin_dir.join("php.bat");
            let script = r#"@echo off
if "%1"=="-n" if "%2"=="-m" (
  echo [PHP Modules]
  echo ctype
  echo dom
  echo fileinfo
  echo filter
  echo hash
  echo json
  echo pcre
  echo pdo
  echo session
  echo tokenizer
  echo xml
  echo [Zend Modules]
  echo Zend OPcache
  exit /b 0
)
if "%1"=="-n" if "%2"=="-c" (
  findstr "broken_ext" "%~3" >nul 2>&1
  if %errorlevel% equ 0 (
    echo PHP Fatal error: Unable to load dynamic library broken_ext >&2
    exit /b 1
  )
  exit /b 0
)
exit /b 0
"#;
            fs::write(&php_path, script).unwrap();
        }
    }
}

impl Drop for TestFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[tokio::test]
async fn test_php_matrix_e2e_scan_and_builtins() {
    let fixture = TestFixture::setup("scan_builtins").await;
    let matrix = get_php_matrix_inner(&fixture.supervisor, "php_83")
        .await
        .expect("get_php_matrix failed");

    assert_eq!(matrix.version, "8.3.14");
    assert_eq!(matrix.profile, PhpProfileMode::Dev);

    // Verify built-ins from fake php -n -m
    let ctype = matrix
        .extensions
        .iter()
        .find(|e| e.name == "ctype")
        .expect("ctype not found");
    assert_eq!(ctype.source, ExtensionSource::BuiltIn);
    assert!(!ctype.toggleable);
    assert!(ctype.enabled);
    assert_eq!(ctype.category, ExtensionCategory::LaravelCore);

    // Verify shared libraries from ext/
    let mbstring = matrix
        .extensions
        .iter()
        .find(|e| e.name == "mbstring")
        .expect("mbstring not found");
    assert_eq!(mbstring.source, ExtensionSource::SharedLib);
    assert!(mbstring.toggleable);
    assert!(!mbstring.enabled);

    let redis = matrix
        .extensions
        .iter()
        .find(|e| e.name == "redis")
        .expect("redis not found");
    assert_eq!(redis.category, ExtensionCategory::DatabaseCache);
    assert!(redis.toggleable);

    let opcache = matrix
        .extensions
        .iter()
        .find(|e| e.name == "opcache")
        .expect("opcache not found");
    assert_eq!(opcache.kind, ExtensionKind::ZendExtension);
    assert_eq!(opcache.source, ExtensionSource::SharedLib);
    assert!(opcache.toggleable);
}

#[tokio::test]
async fn test_php_matrix_e2e_profile_switch_and_topological_sort() {
    let fixture = TestFixture::setup("profile_toposort").await;

    // Enable mbstring and exif
    let _ = toggle_php_extension_inner(&fixture.supervisor, "php_83", "mbstring", true)
        .await
        .unwrap();
    let _ = toggle_php_extension_inner(&fixture.supervisor, "php_83", "exif", true)
        .await
        .unwrap();

    let limits = PhpIniLimits {
        memory_limit: "768M".to_string(),
        upload_max_filesize: "64M".to_string(),
        post_max_size: "64M".to_string(),
        max_execution_time: 120,
    };

    let updated =
        apply_php_profile_inner(&fixture.supervisor, "php_83", PhpProfileMode::Prod, limits)
            .await
            .expect("apply_php_profile failed");

    assert_eq!(updated.profile, PhpProfileMode::Prod);
    assert_eq!(updated.limits.memory_limit, "768M");

    let gen_ini_path = fixture.root.join("config/php/8.3.14-generated.ini");
    let content = fs::read_to_string(&gen_ini_path).expect("generated ini missing");

    // Assert production profile settings
    assert!(content.contains("display_errors = Off"));
    assert!(content.contains("memory_limit = 768M"));
    assert!(content.contains("opcache.validate_timestamps = 0"));

    // Assert topological order (mbstring MUST precede exif)
    let pos_mb = content.find("extension=mbstring").unwrap();
    let pos_exif = content.find("extension=exif").unwrap();
    assert!(
        pos_mb < pos_exif,
        "mbstring must precede exif in rendered ini"
    );
}

#[tokio::test]
async fn test_php_matrix_e2e_allowlist_and_hostile_inputs() {
    let fixture = TestFixture::setup("hostile_input").await;

    // 1. Path traversal attempt
    let res_traversal =
        toggle_php_extension_inner(&fixture.supervisor, "php_83", "../etc/passwd", true).await;
    assert!(matches!(res_traversal, Err(RakitrayError::InvalidInput(_))));

    // 2. Newline injection attempt
    let res_nl =
        toggle_php_extension_inner(&fixture.supervisor, "php_83", "mbstring\n[evil]", true).await;
    assert!(matches!(res_nl, Err(RakitrayError::InvalidInput(_))));

    // 3. Module not in ext/
    let res_unknown =
        toggle_php_extension_inner(&fixture.supervisor, "php_83", "malicious_rootkit", true).await;
    assert!(matches!(res_unknown, Err(RakitrayError::Extension(_))));

    // 4. Missing dependency (exif requires mbstring)
    let res_dep = toggle_php_extension_inner(&fixture.supervisor, "php_83", "exif", true).await;
    assert!(matches!(
        res_dep,
        Err(RakitrayError::Extension(msg)) if msg.contains("requires 'mbstring'")
    ));
}

#[cfg(unix)]
#[tokio::test]
async fn test_php_matrix_e2e_atomic_validation_failure_and_rollback() {
    let fixture = TestFixture::setup("rollback_e2e").await;

    // Enable valid extension first
    let initial = toggle_php_extension_inner(&fixture.supervisor, "php_83", "curl", true)
        .await
        .unwrap();
    assert!(initial
        .extensions
        .iter()
        .any(|e| e.name == "curl" && e.enabled));

    let gen_ini_path = fixture.root.join("config/php/8.3.14-generated.ini");
    let valid_ini_content = fs::read_to_string(&gen_ini_path).unwrap();
    assert!(valid_ini_content.contains("extension=curl"));

    // Now try to enable broken_ext which causes fake php -n -c <tmp> -m to fail with error
    let fail_res =
        toggle_php_extension_inner(&fixture.supervisor, "php_83", "broken_ext", true).await;
    assert!(fail_res.is_err());

    // Verify old INI was not overwritten with broken content
    let preserved_ini_content = fs::read_to_string(&gen_ini_path).unwrap();
    assert_eq!(preserved_ini_content, valid_ini_content);
    assert!(!preserved_ini_content.contains("broken_ext"));

    // State should still report curl enabled and broken_ext disabled
    let current_matrix = get_php_matrix_inner(&fixture.supervisor, "php_83")
        .await
        .unwrap();
    let broken = current_matrix
        .extensions
        .iter()
        .find(|e| e.name == "broken_ext")
        .unwrap();
    assert!(!broken.enabled);
}

#[tokio::test]
async fn test_php_matrix_e2e_laravel_preset_and_clone() {
    let fixture = TestFixture::setup("preset_and_clone").await;

    // 1. 1-Click Laravel Preset
    let outcome = apply_laravel_preset_with_diff(&fixture.supervisor, "php_83")
        .await
        .expect("preset failed");

    assert!(outcome.enabled.contains(&"mbstring".to_string()));
    assert!(outcome.enabled.contains(&"curl".to_string()));
    assert!(outcome.enabled.contains(&"pdo_mysql".to_string()));
    assert!(outcome.enabled.contains(&"redis".to_string()));

    // 2. Clone from PHP 8.3 to PHP 8.2
    let cloned = clone_php_matrix_inner(&fixture.supervisor, "php_83", "php_82")
        .await
        .expect("clone failed");

    assert_eq!(cloned.version, "8.2.20");
    // mbstring, curl, pdo_mysql exist on 8.2 and are enabled
    assert!(cloned
        .extensions
        .iter()
        .any(|e| e.name == "mbstring" && e.enabled));
    assert!(cloned
        .extensions
        .iter()
        .any(|e| e.name == "curl" && e.enabled));
    assert!(cloned
        .extensions
        .iter()
        .any(|e| e.name == "pdo_mysql" && e.enabled));

    // redis is missing on 8.2 ext dir, so it was cleanly skipped with a warning
    assert!(cloned
        .warnings
        .iter()
        .any(|w| w.contains("redis") && w.contains("skipped")));

    // Verify 8.2 generated ini has 8.2 path, not 8.3
    let gen_ini_82 = fixture.root.join("config/php/8.2.20-generated.ini");
    let ini_82 = fs::read_to_string(gen_ini_82).unwrap();
    assert!(ini_82.contains("8.2.20"));
    assert!(!ini_82.contains("8.3.14"));
}
