use super::php_fpm::{
    default_directives_for_profile, default_limits_for_profile, render_ini_with_directives,
    RenderedIni,
};
use super::DriverError;
use crate::models::{PhpIniDirective, PhpIniLimits, PhpProfileMode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::fs::{self, File};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::sync::Mutex;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhpMatrixState {
    pub profile: PhpProfileMode,
    pub limits: PhpIniLimits,
    pub enabled_exts: Vec<String>,
    #[serde(default)]
    pub directives: Vec<PhpIniDirective>,
}

#[derive(Debug, thiserror::Error)]
pub enum MatrixStoreError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Driver error: {0}")]
    Driver(#[from] DriverError),
    #[error("TOML deserialize error: {0}")]
    TomlDe(#[from] toml::de::Error),
    #[error("TOML serialize error: {0}")]
    TomlSer(#[from] toml::ser::Error),
    #[error("PHP validation failed: {0}")]
    ValidationFailed(String),
    #[error("Rollback failed: {0}")]
    RollbackFailed(String),
}

#[derive(Clone, Default)]
pub struct PhpMatrixStore {
    locks: Arc<Mutex<HashMap<String, Arc<Mutex<()>>>>>,
}

impl PhpMatrixStore {
    pub fn new() -> Self {
        Self {
            locks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn get_version_lock(&self, version: &str) -> Arc<Mutex<()>> {
        let mut map = self.locks.lock().await;
        map.entry(version.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    pub fn matrix_toml_path(config_php_dir: &Path, version: &str) -> PathBuf {
        config_php_dir.join(format!("{version}.matrix.toml"))
    }

    pub fn generated_ini_path(config_php_dir: &Path, version: &str) -> PathBuf {
        config_php_dir.join(format!("{version}-generated.ini"))
    }

    pub fn user_ini_path(config_php_dir: &Path, version: &str) -> PathBuf {
        config_php_dir.join(format!("{version}-user.ini"))
    }

    pub async fn ensure_user_ini(
        config_php_dir: &Path,
        version: &str,
    ) -> Result<PathBuf, std::io::Error> {
        let path = Self::user_ini_path(config_php_dir, version);
        if !path.exists() {
            fs::create_dir_all(config_php_dir).await?;
            let content = format!(
                "; Rakitray user overrides for PHP {version}\n; Add custom directives here; Rakitray will never overwrite this file.\n"
            );
            fs::write(&path, content).await?;
        }
        Ok(path)
    }

    pub async fn read_state(
        &self,
        config_php_dir: &Path,
        version: &str,
        default_profile: PhpProfileMode,
    ) -> Result<PhpMatrixState, MatrixStoreError> {
        let toml_path = Self::matrix_toml_path(config_php_dir, version);
        if toml_path.exists() {
            let content = fs::read_to_string(&toml_path).await?;
            let mut state: PhpMatrixState = toml::from_str(&content)?;
            if state.directives.is_empty() {
                state.directives = default_directives_for_profile(state.profile, &state.limits);
            }
            return Ok(state);
        }

        let ini_path = Self::generated_ini_path(config_php_dir, version);
        if ini_path.exists() {
            let content = fs::read_to_string(&ini_path).await?;
            let enabled = crate::discovery::php_ext::parse_enabled_from_ini(&content);
            let mut enabled_exts: Vec<String> = enabled.into_iter().collect();
            enabled_exts.sort();
            let limits = default_limits_for_profile(default_profile);
            let directives = default_directives_for_profile(default_profile, &limits);
            return Ok(PhpMatrixState {
                profile: default_profile,
                limits,
                enabled_exts,
                directives,
            });
        }

        let limits = default_limits_for_profile(default_profile);
        let directives = default_directives_for_profile(default_profile, &limits);
        Ok(PhpMatrixState {
            profile: default_profile,
            limits,
            enabled_exts: Vec::new(),
            directives,
        })
    }

    pub async fn validate_ini_file(php_bin: &Path, tmp_ini: &Path) -> Result<(), MatrixStoreError> {
        let mut cmd = Command::new(php_bin);
        cmd.args(["-n", "-c"])
            .arg(tmp_ini)
            .arg("-m")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        let timeout_res = tokio::time::timeout(Duration::from_secs(5), cmd.output()).await;
        match timeout_res {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let combined = format!("{stdout}\n{stderr}");

                let combined_lower = combined.to_lowercase();
                if combined_lower.contains("unable to load dynamic library")
                    || combined_lower.contains("php startup:")
                    || combined_lower.contains("fatal error")
                {
                    return Err(MatrixStoreError::ValidationFailed(
                        combined.trim().to_string(),
                    ));
                }

                if !output.status.success() {
                    return Err(MatrixStoreError::ValidationFailed(format!(
                        "PHP validation process exited with {}: {}",
                        output.status,
                        combined.trim()
                    )));
                }

                Ok(())
            }
            Ok(Err(e)) => Err(MatrixStoreError::ValidationFailed(format!(
                "Failed to execute PHP validation: {e}"
            ))),
            Err(_) => Err(MatrixStoreError::ValidationFailed(
                "PHP validation process timed out after 5 seconds".to_string(),
            )),
        }
    }

    pub async fn write_matrix(
        &self,
        config_php_dir: &Path,
        version: &str,
        state: &PhpMatrixState,
        php_bin: Option<&Path>,
        ext_dir: Option<&Path>,
    ) -> Result<RenderedIni, MatrixStoreError> {
        let lock = self.get_version_lock(version).await;
        let _guard = lock.lock().await;

        fs::create_dir_all(config_php_dir).await?;
        Self::ensure_user_ini(config_php_dir, version).await?;

        let rendered = render_ini_with_directives(
            version,
            state.profile,
            &state.limits,
            &state.enabled_exts,
            ext_dir,
            &state.directives,
        )?;

        let gen_ini = Self::generated_ini_path(config_php_dir, version);
        let tmp_ini = config_php_dir.join(format!("{version}-generated.ini.tmp"));
        let bak_ini = config_php_dir.join(format!("{version}-generated.ini.bak"));

        let toml_file = Self::matrix_toml_path(config_php_dir, version);
        let tmp_toml = config_php_dir.join(format!("{version}.matrix.toml.tmp"));
        let bak_toml = config_php_dir.join(format!("{version}.matrix.toml.bak"));

        {
            let mut file = File::create(&tmp_ini).await?;
            file.write_all(rendered.ini.as_bytes()).await?;
            file.sync_all().await?;
        }

        if let Some(bin) = php_bin {
            if bin.exists() {
                if let Err(e) = Self::validate_ini_file(bin, &tmp_ini).await {
                    let _ = fs::remove_file(&tmp_ini).await;
                    return Err(e);
                }
            }
        }

        if gen_ini.exists() {
            let _ = fs::copy(&gen_ini, &bak_ini).await;
        }
        if toml_file.exists() {
            let _ = fs::copy(&toml_file, &bak_toml).await;
        }

        fs::rename(&tmp_ini, &gen_ini).await?;

        let toml_str = toml::to_string_pretty(state)?;
        {
            let mut file = File::create(&tmp_toml).await?;
            file.write_all(toml_str.as_bytes()).await?;
            file.sync_all().await?;
        }
        fs::rename(&tmp_toml, &toml_file).await?;

        Ok(rendered)
    }

    pub async fn rollback_matrix(
        &self,
        config_php_dir: &Path,
        version: &str,
    ) -> Result<(), MatrixStoreError> {
        let lock = self.get_version_lock(version).await;
        let _guard = lock.lock().await;

        let gen_ini = Self::generated_ini_path(config_php_dir, version);
        let bak_ini = config_php_dir.join(format!("{version}-generated.ini.bak"));

        let toml_file = Self::matrix_toml_path(config_php_dir, version);
        let bak_toml = config_php_dir.join(format!("{version}.matrix.toml.bak"));

        if bak_ini.exists() {
            fs::copy(&bak_ini, &gen_ini).await?;
        } else {
            return Err(MatrixStoreError::RollbackFailed(format!(
                "No backup file found at '{}'",
                bak_ini.display()
            )));
        }

        if bak_toml.exists() {
            let _ = fs::copy(&bak_toml, &toml_file).await;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TestTempDir(PathBuf);
    impl TestTempDir {
        fn new(prefix: &str) -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "rakitray_matrix_test_{prefix}_{}_{}",
                std::process::id(),
                nanos
            ));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
        fn path(&self) -> &Path {
            &self.0
        }
    }
    impl Drop for TestTempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[tokio::test]
    async fn write_matrix_success_and_preserves_user_ini() {
        let dir = TestTempDir::new("success");
        let store = PhpMatrixStore::new();
        let version = "8.3.14";

        let user_ini = PhpMatrixStore::user_ini_path(dir.path(), version);
        std::fs::create_dir_all(dir.path()).unwrap();
        std::fs::write(&user_ini, "custom_user_directive = 1\n").unwrap();

        let limits1 = default_limits_for_profile(PhpProfileMode::Dev);
        let directives1 = default_directives_for_profile(PhpProfileMode::Dev, &limits1);
        let state1 = PhpMatrixState {
            profile: PhpProfileMode::Dev,
            limits: limits1,
            enabled_exts: vec!["curl".to_string(), "mbstring".to_string()],
            directives: directives1,
        };

        let rendered1 = store
            .write_matrix(dir.path(), version, &state1, None, None)
            .await
            .unwrap();
        assert!(rendered1.ini.contains("extension=curl"));
        assert!(rendered1.ini.contains("extension=mbstring"));

        let user_content = std::fs::read_to_string(&user_ini).unwrap();
        assert_eq!(user_content, "custom_user_directive = 1\n");

        let gen_ini = PhpMatrixStore::generated_ini_path(dir.path(), version);
        let gen_content = std::fs::read_to_string(&gen_ini).unwrap();
        assert_eq!(gen_content, rendered1.ini);

        let read_back = store
            .read_state(dir.path(), version, PhpProfileMode::Dev)
            .await
            .unwrap();
        assert_eq!(read_back, state1);

        let limits2 = default_limits_for_profile(PhpProfileMode::Prod);
        let directives2 = default_directives_for_profile(PhpProfileMode::Prod, &limits2);
        let state2 = PhpMatrixState {
            profile: PhpProfileMode::Prod,
            limits: limits2,
            enabled_exts: vec!["mbstring".to_string()],
            directives: directives2,
        };
        store
            .write_matrix(dir.path(), version, &state2, None, None)
            .await
            .unwrap();

        let bak_ini = dir.path().join(format!("{version}-generated.ini.bak"));
        assert!(bak_ini.exists());
        let bak_content = std::fs::read_to_string(&bak_ini).unwrap();
        assert_eq!(bak_content, rendered1.ini);

        store.rollback_matrix(dir.path(), version).await.unwrap();
        let restored = std::fs::read_to_string(&gen_ini).unwrap();
        assert_eq!(restored, rendered1.ini);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn write_matrix_validation_failure_preserves_old_ini() {
        use std::os::unix::fs::PermissionsExt;

        let dir = TestTempDir::new("validation_fail");
        let store = PhpMatrixStore::new();
        let version = "8.3.14";

        let bin_dir = dir.path().join("bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        let stub_php = bin_dir.join("php");
        std::fs::write(
            &stub_php,
            b"#!/bin/sh\nif grep -q 'fake_ext' \"$3\"; then\n  echo 'PHP Warning: PHP Startup: Unable to load dynamic library fake_ext' >&2\n  exit 1\nfi\nexit 0\n",
        )
        .unwrap();

        let mut perms = std::fs::metadata(&stub_php).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&stub_php, perms).unwrap();

        let limits_dev = default_limits_for_profile(PhpProfileMode::Dev);
        let dirs_dev = default_directives_for_profile(PhpProfileMode::Dev, &limits_dev);
        let initial_state = PhpMatrixState {
            profile: PhpProfileMode::Dev,
            limits: limits_dev.clone(),
            enabled_exts: vec!["curl".to_string()],
            directives: dirs_dev.clone(),
        };

        store
            .write_matrix(dir.path(), version, &initial_state, Some(&stub_php), None)
            .await
            .unwrap();

        let gen_ini = PhpMatrixStore::generated_ini_path(dir.path(), version);
        let initial_content = std::fs::read_to_string(&gen_ini).unwrap();
        assert!(initial_content.contains("extension=curl"));

        let bad_state = PhpMatrixState {
            profile: PhpProfileMode::Dev,
            limits: limits_dev.clone(),
            enabled_exts: vec!["fake_ext".to_string()],
            directives: dirs_dev.clone(),
        };

        let err = store
            .write_matrix(dir.path(), version, &bad_state, Some(&stub_php), None)
            .await;
        assert!(err.is_err());
        match err.unwrap_err() {
            MatrixStoreError::ValidationFailed(msg) => {
                assert!(msg.contains("Unable to load dynamic library"));
            }
            other => panic!("Expected ValidationFailed, got {other:?}"),
        }

        let preserved_content = std::fs::read_to_string(&gen_ini).unwrap();
        assert_eq!(preserved_content, initial_content);

        let preserved_state = store
            .read_state(dir.path(), version, PhpProfileMode::Dev)
            .await
            .unwrap();
        assert_eq!(preserved_state, initial_state);
    }

    #[tokio::test]
    async fn write_matrix_mid_write_crash_does_not_corrupt_old_file() {
        let dir = TestTempDir::new("crash");
        let store = PhpMatrixStore::new();
        let version = "8.3.14";

        let limits_crash = default_limits_for_profile(PhpProfileMode::Dev);
        let dirs_crash = default_directives_for_profile(PhpProfileMode::Dev, &limits_crash);
        let state = PhpMatrixState {
            profile: PhpProfileMode::Dev,
            limits: limits_crash,
            enabled_exts: vec!["curl".to_string()],
            directives: dirs_crash,
        };
        store
            .write_matrix(dir.path(), version, &state, None, None)
            .await
            .unwrap();

        let gen_ini = PhpMatrixStore::generated_ini_path(dir.path(), version);
        let orig_ini = std::fs::read_to_string(&gen_ini).unwrap();

        let tmp_ini = dir.path().join(format!("{version}-generated.ini.tmp"));
        std::fs::write(&tmp_ini, "corrupted partial content").unwrap();

        let current_ini = std::fs::read_to_string(&gen_ini).unwrap();
        assert_eq!(current_ini, orig_ini);

        let current_state = store
            .read_state(dir.path(), version, PhpProfileMode::Dev)
            .await
            .unwrap();
        assert_eq!(current_state, state);
    }
}
