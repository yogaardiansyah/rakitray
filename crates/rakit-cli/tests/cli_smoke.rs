use std::fs;
use std::process::Command;

#[test]
fn cli_version_outputs_rakit() {
    let out = Command::new(env!("CARGO_BIN_EXE_rakit"))
        .arg("--version")
        .output()
        .expect("failed to execute rakit --version");

    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("rakit"), "stdout was: {stdout}");
}

#[test]
fn cli_store_list_json_returns_catalog() {
    let out = Command::new(env!("CARGO_BIN_EXE_rakit"))
        .args(["store", "list", "--json"])
        .output()
        .expect("failed to execute rakit store list --json");

    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("store list --json must emit valid JSON");
    let arr = parsed.as_array().expect("catalog JSON must be an array");
    assert!(!arr.is_empty(), "catalog JSON must not be empty");
}

#[test]
fn cli_scan_succeeds() {
    let out = Command::new(env!("CARGO_BIN_EXE_rakit"))
        .arg("scan")
        .output()
        .expect("failed to execute rakit scan");

    assert!(out.status.success());
}

#[test]
fn cli_init_writes_manifest_in_temp_dir() {
    let tmp = std::env::temp_dir().join(format!("rakit_cli_smoke_{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).expect("create temp dir");

    let out = Command::new(env!("CARGO_BIN_EXE_rakit"))
        .current_dir(&tmp)
        .args([
            "init",
            "--name",
            "smoke-app",
            "--php",
            "8.3",
            "--web",
            "nginx",
        ])
        .output()
        .expect("failed to execute rakit init");

    assert!(out.status.success());
    let manifest_path = tmp.join(".rakitray.toml");
    assert!(manifest_path.exists(), ".rakitray.toml should be created");
    let content = fs::read_to_string(&manifest_path).expect("read .rakitray.toml");
    assert!(content.contains("smoke-app"));

    let _ = fs::remove_dir_all(&tmp);
}
