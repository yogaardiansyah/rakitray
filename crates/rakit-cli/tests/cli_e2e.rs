use std::fs;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Command;

fn rakit_bin() -> &'static str {
    env!("CARGO_BIN_EXE_rakit")
}

fn setup_temp_workspace(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rakit_cli_e2e_{}_{}", name, std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("project")).unwrap();
    fs::create_dir_all(dir.join("runtime/bin")).unwrap();
    dir
}

fn create_mock_runtime(
    runtime_dir: &Path,
    category: &str,
    engine: &str,
    version: &str,
    bin_rel: &str,
) {
    let rt_dir = runtime_dir
        .join("bin")
        .join(category)
        .join(engine)
        .join(version);
    let bin_path = rt_dir.join(bin_rel);
    fs::create_dir_all(bin_path.parent().unwrap()).unwrap();
    // Executable no-op script on Unix (or file on Windows)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::write(&bin_path, "#!/usr/bin/env sh\nexit 0\n").unwrap();
        let mut perms = fs::metadata(&bin_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&bin_path, perms).unwrap();
    }
    #[cfg(not(unix))]
    fs::write(&bin_path, "").unwrap();

    rakit_core::store::CatalogManager::write_blueprint_to_bin(&rt_dir, engine, version).unwrap();
}

#[test]
fn cli_init_creates_manifest_and_prevents_overwrite() {
    let ws = setup_temp_workspace("init");
    let proj = ws.join("project");

    let out = Command::new(rakit_bin())
        .current_dir(&proj)
        .env("RAKITRAY_RUNTIME_DIR", ws.join("runtime"))
        .args([
            "init", "--name", "demo-app", "--php", "8.3.9", "--web", "nginx",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(proj.join(".rakitray.toml").exists());

    let out2 = Command::new(rakit_bin())
        .current_dir(&proj)
        .env("RAKITRAY_RUNTIME_DIR", ws.join("runtime"))
        .args(["init"])
        .output()
        .unwrap();
    assert!(!out2.status.success());

    let _ = fs::remove_dir_all(&ws);
}

#[test]
fn cli_up_fails_without_manifest() {
    let ws = setup_temp_workspace("no_manifest");
    let out = Command::new(rakit_bin())
        .current_dir(ws.join("project"))
        .env("RAKITRAY_RUNTIME_DIR", ws.join("runtime"))
        .arg("up")
        .output()
        .unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains(".rakitray.toml"));
    let _ = fs::remove_dir_all(&ws);
}

#[test]
fn cli_up_renders_configs_detects_port_conflict_and_env_isolates_path() {
    let ws = setup_temp_workspace("up_env");
    let proj = ws.join("project");
    let rt = ws.join("runtime");

    create_mock_runtime(&rt, "runtime", "php", "8.3.9", "php");
    create_mock_runtime(&rt, "cache", "redis", "7.2.5", "bin/redis-server");

    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let busy_port = listener.local_addr().unwrap().port();
    let free_listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let free_port = free_listener.local_addr().unwrap().port();
    drop(free_listener);

    fs::write(
        proj.join(".rakitray.toml"),
        format!(
            r#"schema_version = 1
name = "sample-stack"

[env]
APP_ENV = "local"

[services.php]
version = "8.3.9"
port = {}

[services.php.env]
PHP_MEMORY = "512M"

[services.php.config]
memory_limit = "512M"

[services.redis]
version = "7.2.5"

[services.redis.config]
maxmemory = "128mb"
"#,
            free_port
        ),
    )
    .unwrap();

    // Test rakit env output (posix + powershell)
    let env_out = Command::new(rakit_bin())
        .current_dir(&proj)
        .env("RAKITRAY_RUNTIME_DIR", &rt)
        .arg("env")
        .output()
        .unwrap();
    assert!(env_out.status.success());
    let env_stdout = String::from_utf8_lossy(&env_out.stdout);
    assert!(
        env_stdout.contains("export PATH=\""),
        "stdout={:?}, stderr={:?}",
        env_stdout,
        String::from_utf8_lossy(&env_out.stderr)
    );
    assert!(env_stdout.contains("php/8.3.9"));
    assert!(env_stdout.contains("export APP_ENV=\"local\""));
    assert!(env_stdout.contains("export PHP_MEMORY=\"512M\""));

    let ps_out = Command::new(rakit_bin())
        .current_dir(&proj)
        .env("RAKITRAY_RUNTIME_DIR", &rt)
        .args(["env", "--shell", "powershell"])
        .output()
        .unwrap();
    assert!(ps_out.status.success());
    let ps_stdout = String::from_utf8_lossy(&ps_out.stdout);
    assert!(ps_stdout.contains("$env:PATH = \""));
    assert!(ps_stdout.contains("$env:APP_ENV = \"local\""));

    // Test rakit up with free port -> renders engine-specific configs
    let up_out = Command::new(rakit_bin())
        .current_dir(&proj)
        .env("RAKITRAY_RUNTIME_DIR", &rt)
        .arg("up")
        .output()
        .unwrap();
    assert!(
        up_out.status.success(),
        "{}",
        String::from_utf8_lossy(&up_out.stderr)
    );

    let php_conf = fs::read_to_string(rt.join("projects/sample-stack/php.conf")).unwrap();
    let redis_conf = fs::read_to_string(rt.join("projects/sample-stack/redis.conf")).unwrap();
    assert_eq!(php_conf, "memory_limit = 512M");
    assert_eq!(redis_conf, "maxmemory 128mb");

    // Test rakit up with busy port -> refuses cleanly
    fs::write(
        proj.join(".rakitray.toml"),
        format!(
            "schema_version = 1\nname = \"sample-stack\"\n\n[services.php]\nversion = \"8.3.9\"\nport = {}\n",
            busy_port
        ),
    )
    .unwrap();
    let conflict_out = Command::new(rakit_bin())
        .current_dir(&proj)
        .env("RAKITRAY_RUNTIME_DIR", &rt)
        .arg("up")
        .output()
        .unwrap();
    assert!(!conflict_out.status.success());
    assert!(String::from_utf8_lossy(&conflict_out.stderr).contains("already in use"));

    drop(listener);
    let _ = fs::remove_dir_all(&ws);
}

#[test]
fn cli_uninstall_rejects_ambiguous_and_removes_valid_runtime() {
    let ws = setup_temp_workspace("uninstall");
    let rt = ws.join("runtime");

    create_mock_runtime(&rt, "runtime", "python", "3.11.9", "bin/python3");
    create_mock_runtime(&rt, "runtime", "python", "3.12.16", "bin/python3");

    // Ambiguous prefix -> must fail without deleting either
    let ambig = Command::new(rakit_bin())
        .env("RAKITRAY_RUNTIME_DIR", &rt)
        .args(["uninstall", "python@3"])
        .output()
        .unwrap();
    assert!(!ambig.status.success());
    assert!(String::from_utf8_lossy(&ambig.stderr).contains("Ambiguous target"));
    assert!(rt.join("bin/runtime/python/3.11.9").exists());
    assert!(rt.join("bin/runtime/python/3.12.16").exists());

    // Exact match -> removes only 3.11.9
    let ok = Command::new(rakit_bin())
        .env("RAKITRAY_RUNTIME_DIR", &rt)
        .args(["uninstall", "python@3.11.9"])
        .output()
        .unwrap();
    assert!(
        ok.status.success(),
        "{}",
        String::from_utf8_lossy(&ok.stderr)
    );
    assert!(!rt.join("bin/runtime/python/3.11.9").exists());
    assert!(rt.join("bin/runtime/python/3.12.16").exists());

    let _ = fs::remove_dir_all(&ws);
}
