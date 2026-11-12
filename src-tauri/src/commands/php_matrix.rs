use crate::commands::SupervisorState;
use crate::models::{
    ExtensionSource, PhpIniDirective, PhpIniLimits, PhpMatrix, PhpMatrixEvent, PhpProfileMode,
};
use crate::supervisor::php_matrix::PhpMatrixChange;
use crate::supervisor::{Supervisor, SupervisorError};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use tauri::ipc::Channel;
use tauri::State;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RakitrayError {
    #[error("Instance not found: {0}")]
    NotFound(String),
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    #[error("Extension error: {0}")]
    Extension(String),
    #[error("Supervisor error: {0}")]
    Supervisor(String),
}

impl serde::Serialize for RakitrayError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl From<SupervisorError> for RakitrayError {
    fn from(err: SupervisorError) -> Self {
        match err {
            SupervisorError::NotFound(id) => Self::NotFound(id),
            other => Self::Supervisor(other.to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaravelPresetOutcome {
    pub matrix: PhpMatrix,
    pub enabled: Vec<String>,
    pub already_present: Vec<String>,
    pub missing: Vec<String>,
}

pub const LARAVEL_REQUIRED: &[&str] = &[
    "ctype",
    "curl",
    "dom",
    "fileinfo",
    "filter",
    "hash",
    "mbstring",
    "openssl",
    "pcre",
    "pdo",
    "session",
    "tokenizer",
    "xml",
];

pub const LARAVEL_RECOMMENDED: &[&str] = &["pdo_mysql", "zip", "bcmath", "intl", "gd", "exif"];

pub const LARAVEL_OPTIONAL: &[&str] = &["redis"];

fn validate_ext_name_format(name: &str) -> Result<String, RakitrayError> {
    let s = name.trim();
    if s.is_empty() {
        return Err(RakitrayError::InvalidInput(
            "Extension name cannot be empty".to_string(),
        ));
    }
    if !s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(RakitrayError::InvalidInput(format!(
            "Extension name '{s}' contains invalid characters (allowed: [a-zA-Z0-9_])"
        )));
    }
    Ok(s.to_lowercase())
}

fn check_php_version_for_laravel(version: &str) -> Vec<String> {
    let mut warnings = Vec::new();
    let parts: Vec<&str> = version.split('.').collect();
    if let (Some(maj_str), Some(min_str)) = (parts.first(), parts.get(1)) {
        if let (Ok(maj), Ok(min)) = (maj_str.parse::<u32>(), min_str.parse::<u32>()) {
            if maj < 8 || (maj == 8 && min < 2) {
                warnings.push(format!(
                    "Laravel 12 requires PHP >= 8.2; current version is {version}"
                ));
            }
            if maj < 8 || (maj == 8 && min < 3) {
                warnings.push(format!(
                    "Laravel 13 requires PHP >= 8.3; current version is {version}"
                ));
            }
        }
    }
    warnings
}

pub async fn get_php_matrix_inner(
    supervisor: &Arc<Supervisor>,
    instance_id: &str,
) -> Result<PhpMatrix, RakitrayError> {
    supervisor
        .get_php_matrix(instance_id)
        .await
        .map_err(Into::into)
}

pub async fn toggle_php_extension_inner(
    supervisor: &Arc<Supervisor>,
    instance_id: &str,
    ext_name: &str,
    enabled: bool,
) -> Result<PhpMatrix, RakitrayError> {
    let norm = validate_ext_name_format(ext_name)?;

    let current = supervisor.get_php_matrix(instance_id).await?;

    let ext_def = current
        .extensions
        .iter()
        .find(|e| e.name == norm)
        .ok_or_else(|| {
            RakitrayError::Extension(format!(
                "Extension '{norm}' is not available for this PHP installation"
            ))
        })?;

    if !ext_def.toggleable {
        return Err(RakitrayError::Extension(format!(
            "Extension '{norm}' is compiled directly into PHP and cannot be toggled"
        )));
    }

    if enabled {
        for req in &ext_def.requires {
            let req_enabled = current
                .extensions
                .iter()
                .any(|e| e.name == req.as_str() && e.enabled);
            if !req_enabled {
                return Err(RakitrayError::Extension(format!(
                    "Extension '{norm}' requires '{req}' to be enabled first"
                )));
            }
        }
    } else {
        for other in &current.extensions {
            if other.enabled && other.requires.contains(&norm) {
                return Err(RakitrayError::Extension(format!(
                    "Cannot disable '{norm}' because enabled extension '{}' depends on it",
                    other.name
                )));
            }
        }
    }

    let change = PhpMatrixChange::ToggleExtension {
        ext_name: norm,
        enabled,
    };
    supervisor
        .apply_php_matrix(instance_id, change)
        .await
        .map_err(Into::into)
}

pub async fn apply_php_profile_inner(
    supervisor: &Arc<Supervisor>,
    instance_id: &str,
    profile: PhpProfileMode,
    limits: PhpIniLimits,
) -> Result<PhpMatrix, RakitrayError> {
    let change = PhpMatrixChange::ApplyProfile { profile, limits };
    supervisor
        .apply_php_matrix(instance_id, change)
        .await
        .map_err(Into::into)
}

pub async fn apply_laravel_preset_with_diff(
    supervisor: &Arc<Supervisor>,
    instance_id: &str,
) -> Result<LaravelPresetOutcome, RakitrayError> {
    let current = supervisor.get_php_matrix(instance_id).await?;

    let mut enabled_set: HashSet<String> = current
        .extensions
        .iter()
        .filter(|e| e.enabled)
        .map(|e| e.name.clone())
        .collect();

    let mut newly_enabled = Vec::new();
    let mut already_present = Vec::new();
    let mut missing = Vec::new();
    let mut extra_warnings = check_php_version_for_laravel(&current.version);

    let all_preset_tiers = [
        ("required", LARAVEL_REQUIRED),
        ("recommended", LARAVEL_RECOMMENDED),
        ("optional", LARAVEL_OPTIONAL),
    ];

    for (_tier, tier_exts) in all_preset_tiers {
        for &raw_name in tier_exts {
            let name = raw_name.to_lowercase();
            if let Some(ext) = current.extensions.iter().find(|e| e.name == name) {
                if ext.enabled || ext.source == ExtensionSource::BuiltIn {
                    already_present.push(name);
                } else {
                    newly_enabled.push(name.clone());
                    enabled_set.insert(name);
                }
            } else {
                missing.push(name.clone());
                extra_warnings.push(format!(
                    "{name} not found: place php_{name}.dll in ext/ (or install {name}.so)"
                ));
            }
        }
    }

    let mut final_exts: Vec<String> = enabled_set.into_iter().collect();
    final_exts.sort();

    let change = PhpMatrixChange::SetExtensions {
        enabled_exts: final_exts,
        label: "applied 1-Click Laravel Preset".to_string(),
        extra_warnings,
    };

    let updated_matrix = supervisor.apply_php_matrix(instance_id, change).await?;

    Ok(LaravelPresetOutcome {
        matrix: updated_matrix,
        enabled: newly_enabled,
        already_present,
        missing,
    })
}

pub async fn clone_php_matrix_inner(
    supervisor: &Arc<Supervisor>,
    source_instance_id: &str,
    target_instance_id: &str,
) -> Result<PhpMatrix, RakitrayError> {
    if source_instance_id == target_instance_id {
        return Err(RakitrayError::InvalidInput(
            "Source and target instance must be different".to_string(),
        ));
    }

    let source = supervisor.get_php_matrix(source_instance_id).await?;
    let target = supervisor.get_php_matrix(target_instance_id).await?;

    if source.version == target.version {
        return Err(RakitrayError::InvalidInput(format!(
            "Cannot clone between instances of the same PHP version ({})",
            source.version
        )));
    }

    let target_toggleable: HashSet<String> = target
        .extensions
        .iter()
        .filter(|e| e.toggleable)
        .map(|e| e.name.clone())
        .collect();

    let target_builtin: HashSet<String> = target
        .extensions
        .iter()
        .filter(|e| e.source == ExtensionSource::BuiltIn)
        .map(|e| e.name.clone())
        .collect();

    let target_requires: std::collections::HashMap<String, Vec<String>> = target
        .extensions
        .iter()
        .map(|e| (e.name.clone(), e.requires.clone()))
        .collect();

    let source_enabled: Vec<String> = source
        .extensions
        .iter()
        .filter(|e| e.enabled)
        .map(|e| e.name.clone())
        .collect();

    let mut candidate_enabled = HashSet::new();
    let mut missing_on_target = Vec::new();

    for ext in source_enabled {
        if target_toggleable.contains(&ext) {
            candidate_enabled.insert(ext);
        } else if target_builtin.contains(&ext) {
            // Satisfied as built-in on target; no ini extension needed
        } else {
            missing_on_target.push(ext);
        }
    }

    let mut extra_warnings = Vec::new();
    for missing in missing_on_target {
        extra_warnings.push(format!(
            "Extension '{missing}' from PHP {} is not available in PHP {} (skipped)",
            source.version, target.version
        ));
    }

    let mut cloned_enabled = Vec::new();
    for cand in candidate_enabled.iter() {
        if let Some(reqs) = target_requires.get(cand) {
            let missing_req = reqs
                .iter()
                .find(|r| !candidate_enabled.contains(*r) && !target_builtin.contains(*r));
            if let Some(req) = missing_req {
                extra_warnings.push(format!(
                    "Extension '{cand}' skipped on PHP {} because prerequisite '{req}' is not available",
                    target.version
                ));
                continue;
            }
        }
        cloned_enabled.push(cand.clone());
    }
    cloned_enabled.sort();

    let new_state = crate::drivers::php_matrix_store::PhpMatrixState {
        profile: source.profile,
        limits: source.limits,
        enabled_exts: cloned_enabled,
        directives: source.directives,
    };

    let change = PhpMatrixChange::ReplaceState {
        state: new_state,
        label: format!("cloned profile from PHP {}", source.version),
        extra_warnings,
    };

    supervisor
        .apply_php_matrix(target_instance_id, change)
        .await
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) async fn subscribe_php_matrix(
    on_event: Channel<PhpMatrixEvent>,
    state: State<'_, SupervisorState>,
) -> Result<(), RakitrayError> {
    state.add_matrix_subscriber(on_event).await;
    Ok(())
}

#[tauri::command]
pub(crate) async fn get_php_matrix(
    instance_id: String,
    state: State<'_, SupervisorState>,
) -> Result<PhpMatrix, RakitrayError> {
    get_php_matrix_inner(&state, &instance_id).await
}

#[tauri::command]
pub(crate) async fn toggle_php_extension(
    instance_id: String,
    ext_name: String,
    enabled: bool,
    state: State<'_, SupervisorState>,
) -> Result<PhpMatrix, RakitrayError> {
    toggle_php_extension_inner(&state, &instance_id, &ext_name, enabled).await
}

#[tauri::command]
pub(crate) async fn apply_php_profile(
    instance_id: String,
    profile: PhpProfileMode,
    limits: PhpIniLimits,
    state: State<'_, SupervisorState>,
) -> Result<PhpMatrix, RakitrayError> {
    apply_php_profile_inner(&state, &instance_id, profile, limits).await
}

#[tauri::command]
pub(crate) async fn apply_laravel_preset(
    instance_id: String,
    state: State<'_, SupervisorState>,
) -> Result<PhpMatrix, RakitrayError> {
    let outcome = apply_laravel_preset_with_diff(&state, &instance_id).await?;
    Ok(outcome.matrix)
}

pub async fn save_php_ini_directives_inner(
    supervisor: &Arc<Supervisor>,
    instance_id: &str,
    directives: Vec<PhpIniDirective>,
) -> Result<PhpMatrix, RakitrayError> {
    for d in &directives {
        crate::drivers::php_fpm::validate_directive_key(&d.key)
            .map_err(|e| RakitrayError::InvalidInput(e.to_string()))?;
        crate::drivers::php_fpm::validate_directive_value(&d.value)
            .map_err(|e| RakitrayError::InvalidInput(e.to_string()))?;
    }
    let change = PhpMatrixChange::UpdateDirectives { directives };
    supervisor
        .apply_php_matrix(instance_id, change)
        .await
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) async fn save_php_ini_directives(
    instance_id: String,
    directives: Vec<PhpIniDirective>,
    state: State<'_, SupervisorState>,
) -> Result<PhpMatrix, RakitrayError> {
    save_php_ini_directives_inner(&state, &instance_id, directives).await
}

#[tauri::command]
pub(crate) async fn clone_php_matrix(
    source_instance_id: String,
    target_instance_id: String,
    state: State<'_, SupervisorState>,
) -> Result<PhpMatrix, RakitrayError> {
    clone_php_matrix_inner(&state, &source_instance_id, &target_instance_id).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::time::Duration;

    fn setup_test_fixture(name: &str) -> (PathBuf, Arc<Supervisor>) {
        let root =
            std::env::temp_dir().join(format!("rakit_cmd_test_{name}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();

        let php83_ext = root.join("bin/runtime/php/8.3.14/ext");
        fs::create_dir_all(&php83_ext).unwrap();
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
            "php_custom.dll",
        ] {
            fs::write(php83_ext.join(ext), b"").unwrap();
        }

        let php82_ext = root.join("bin/runtime/php/8.2.20/ext");
        fs::create_dir_all(&php82_ext).unwrap();
        for ext in &["php_mbstring.dll", "php_curl.dll"] {
            fs::write(php82_ext.join(ext), b"").unwrap();
        }

        let supervisor = Arc::new(Supervisor::with_debounce(
            root.clone(),
            Duration::from_millis(10),
        ));
        (root, supervisor)
    }

    #[tokio::test]
    async fn command_get_and_toggle_extension_allowlist_and_hostile_inputs() {
        let (root, supervisor) = setup_test_fixture("toggle_sec");

        let matrix = get_php_matrix_inner(&supervisor, "php_83").await.unwrap();
        assert_eq!(matrix.version, "8.3.14");
        assert!(matrix.extensions.iter().any(|e| e.name == "mbstring"));

        let bad_dotdot = toggle_php_extension_inner(&supervisor, "php_83", "../bad", true).await;
        assert!(matches!(bad_dotdot, Err(RakitrayError::InvalidInput(_))));

        let bad_nl = toggle_php_extension_inner(&supervisor, "php_83", "a\nb", true).await;
        assert!(matches!(bad_nl, Err(RakitrayError::InvalidInput(_))));

        let bad_unknown =
            toggle_php_extension_inner(&supervisor, "php_83", "non_existent_module", true).await;
        assert!(matches!(bad_unknown, Err(RakitrayError::Extension(_))));

        let exif_without_mb = toggle_php_extension_inner(&supervisor, "php_83", "exif", true).await;
        assert!(
            matches!(exif_without_mb, Err(RakitrayError::Extension(msg)) if msg.contains("requires 'mbstring'"))
        );

        let res_mb = toggle_php_extension_inner(&supervisor, "php_83", "mbstring", true)
            .await
            .unwrap();
        assert!(res_mb
            .extensions
            .iter()
            .any(|e| e.name == "mbstring" && e.enabled));

        let res_exif = toggle_php_extension_inner(&supervisor, "php_83", "exif", true)
            .await
            .unwrap();
        assert!(res_exif
            .extensions
            .iter()
            .any(|e| e.name == "exif" && e.enabled));

        let disable_mb = toggle_php_extension_inner(&supervisor, "php_83", "mbstring", false).await;
        assert!(matches!(disable_mb, Err(RakitrayError::Extension(msg)) if msg.contains("exif")));

        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn command_apply_php_profile() {
        let (root, supervisor) = setup_test_fixture("apply_profile");

        let limits = PhpIniLimits {
            memory_limit: "2048M".to_string(),
            upload_max_filesize: "128M".to_string(),
            post_max_size: "128M".to_string(),
            max_execution_time: 300,
        };

        let updated = apply_php_profile_inner(&supervisor, "php_83", PhpProfileMode::Prod, limits)
            .await
            .unwrap();

        assert_eq!(updated.profile, PhpProfileMode::Prod);
        assert_eq!(updated.limits.memory_limit, "2048M");
        assert_eq!(updated.limits.max_execution_time, 300);

        let gen_ini_path = root.join("config/php/8.3.14-generated.ini");
        let ini_content = fs::read_to_string(gen_ini_path).unwrap();
        assert!(ini_content.contains("memory_limit = 2048M"));
        assert!(ini_content.contains("display_errors = Off"));

        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn laravel_preset_full_matrix() {
        let (root, supervisor) = setup_test_fixture("preset_full");

        let _ = toggle_php_extension_inner(&supervisor, "php_83", "custom", true)
            .await
            .unwrap();

        let outcome = apply_laravel_preset_with_diff(&supervisor, "php_83")
            .await
            .unwrap();

        assert!(outcome.enabled.contains(&"mbstring".to_string()));
        assert!(outcome.enabled.contains(&"curl".to_string()));
        assert!(outcome.enabled.contains(&"redis".to_string()));
        assert!(outcome
            .matrix
            .extensions
            .iter()
            .any(|e| e.name == "custom" && e.enabled));

        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn laravel_preset_without_redis() {
        let root =
            std::env::temp_dir().join(format!("rakit_cmd_test_no_redis_{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();

        let php83_ext = root.join("bin/runtime/php/8.3.14/ext");
        fs::create_dir_all(&php83_ext).unwrap();
        for ext in &["php_mbstring.dll", "php_curl.dll"] {
            fs::write(php83_ext.join(ext), b"").unwrap();
        }

        let supervisor = Arc::new(Supervisor::with_debounce(
            root.clone(),
            Duration::from_millis(10),
        ));

        let outcome = apply_laravel_preset_with_diff(&supervisor, "php_83")
            .await
            .unwrap();

        assert!(outcome.missing.contains(&"redis".to_string()));
        assert!(outcome
            .matrix
            .warnings
            .iter()
            .any(|w| w.contains("redis not found")));

        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn laravel_preset_php_81_version_warning() {
        let root =
            std::env::temp_dir().join(format!("rakit_cmd_test_php81_{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();

        let php81_ext = root.join("bin/runtime/php/8.1.29/ext");
        fs::create_dir_all(&php81_ext).unwrap();
        fs::write(php81_ext.join("php_mbstring.dll"), b"").unwrap();

        let supervisor = Arc::new(Supervisor::with_debounce(
            root.clone(),
            Duration::from_millis(10),
        ));
        {
            let mut instances = supervisor.instances.write().await;
            instances.insert(
                "php_81".to_string(),
                crate::models::ServiceInstance {
                    instance_id: "php_81".to_string(),
                    engine: "php".to_string(),
                    display_name: "PHP 8.1".to_string(),
                    category: crate::models::ServiceCategory::Runtime,
                    version: "8.1.29".to_string(),
                    profile: "dev".to_string(),
                    status: crate::models::ServiceStatus::Stopped,
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

        let outcome = apply_laravel_preset_with_diff(&supervisor, "php_81")
            .await
            .unwrap();

        assert!(outcome
            .matrix
            .warnings
            .iter()
            .any(|w| w.contains("Laravel 12 requires PHP >= 8.2")));
        assert!(outcome
            .matrix
            .warnings
            .iter()
            .any(|w| w.contains("Laravel 13 requires PHP >= 8.3")));

        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn command_clone_php_matrix() {
        let (root, supervisor) = setup_test_fixture("clone_matrix");

        let _ = toggle_php_extension_inner(&supervisor, "php_83", "mbstring", true)
            .await
            .unwrap();
        let _ = toggle_php_extension_inner(&supervisor, "php_83", "redis", true)
            .await
            .unwrap();

        let cloned = clone_php_matrix_inner(&supervisor, "php_83", "php_82")
            .await
            .unwrap();

        assert_eq!(cloned.version, "8.2.20");
        assert!(cloned
            .extensions
            .iter()
            .any(|e| e.name == "mbstring" && e.enabled));
        assert!(cloned
            .warnings
            .iter()
            .any(|w| w.contains("redis") && w.contains("skipped")));

        let same_inst = clone_php_matrix_inner(&supervisor, "php_83", "php_83").await;
        assert!(matches!(same_inst, Err(RakitrayError::InvalidInput(_))));

        let not_found = clone_php_matrix_inner(&supervisor, "php_83", "nonexistent").await;
        assert!(matches!(not_found, Err(RakitrayError::NotFound(_))));

        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn clone_82_to_83_with_missing_ext_and_isolated_ext_dir() {
        let (root, supervisor) = setup_test_fixture("clone_82_83");

        // Add a legacy-only extension to 8.2 ext dir
        let php82_ext = root.join("bin/runtime/php/8.2.20/ext");
        fs::write(php82_ext.join("php_legacy_only.dll"), b"").unwrap();

        // Configure 8.2 with Prod profile, custom limits, and enabled exts
        let limits_82 = PhpIniLimits {
            memory_limit: "1024M".to_string(),
            upload_max_filesize: "128M".to_string(),
            post_max_size: "128M".to_string(),
            max_execution_time: 180,
        };
        apply_php_profile_inner(&supervisor, "php_82", PhpProfileMode::Prod, limits_82)
            .await
            .unwrap();
        toggle_php_extension_inner(&supervisor, "php_82", "mbstring", true)
            .await
            .unwrap();
        toggle_php_extension_inner(&supervisor, "php_82", "curl", true)
            .await
            .unwrap();
        toggle_php_extension_inner(&supervisor, "php_82", "legacy_only", true)
            .await
            .unwrap();

        // Clone 8.2 -> 8.3
        let cloned = clone_php_matrix_inner(&supervisor, "php_82", "php_83")
            .await
            .unwrap();

        assert_eq!(cloned.version, "8.3.14");
        assert_eq!(cloned.profile, PhpProfileMode::Prod);
        assert_eq!(cloned.limits.memory_limit, "1024M");
        assert_eq!(cloned.limits.upload_max_filesize, "128M");
        assert!(cloned
            .extensions
            .iter()
            .any(|e| e.name == "mbstring" && e.enabled));
        assert!(cloned
            .extensions
            .iter()
            .any(|e| e.name == "curl" && e.enabled));
        assert!(cloned
            .warnings
            .iter()
            .any(|w| w.contains("legacy_only") && w.contains("skipped")));

        // Verify target generated ini has target's own ext dir, not 8.2's ext dir
        let gen_ini_83 = root.join("config/php/8.3.14-generated.ini");
        let ini_content = fs::read_to_string(gen_ini_83).unwrap();
        assert!(!ini_content.contains("8.2.20"));
        assert!(ini_content.contains("8.3.14"));

        // Verify instance profile in supervisor was synchronized
        let inst_map = supervisor.instances.read().await;
        let inst_83 = inst_map.get("php_83").unwrap();
        assert_eq!(inst_83.profile, "prod");

        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn clone_same_version_different_instances_rejected() {
        let (root, supervisor) = setup_test_fixture("clone_same_ver");

        // Add a second instance that uses the same 8.3.14 version
        {
            let mut inst_map = supervisor.instances.write().await;
            inst_map.insert(
                "php_83_worker".to_string(),
                crate::models::ServiceInstance {
                    instance_id: "php_83_worker".to_string(),
                    engine: "php".to_string(),
                    display_name: "PHP 8.3 Worker".to_string(),
                    category: crate::models::ServiceCategory::Runtime,
                    version: "8.3.14".to_string(),
                    profile: "dev".to_string(),
                    status: crate::models::ServiceStatus::Stopped,
                    pid: None,
                    port: Some(9005),
                    socket_path: None,
                    memory_bytes: 0,
                    cpu_percent: 0.0,
                    uptime_secs: 0,
                    pinned_to_rack: false,
                    last_error: None,
                },
            );
        }

        let res = clone_php_matrix_inner(&supervisor, "php_83", "php_83_worker").await;
        assert!(
            matches!(res, Err(RakitrayError::InvalidInput(msg)) if msg.contains("same PHP version"))
        );

        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn save_php_ini_directives_validation_and_persistence() {
        let (root, supervisor) = setup_test_fixture("save_directives");

        let bad_key = vec![PhpIniDirective {
            key: "bad\nkey".to_string(),
            value: "1".to_string(),
            enabled: true,
            section: "Core".to_string(),
            custom: false,
        }];
        assert!(matches!(
            save_php_ini_directives_inner(&supervisor, "php_83", bad_key).await,
            Err(RakitrayError::InvalidInput(_))
        ));

        let bad_val = vec![PhpIniDirective {
            key: "memory_limit".to_string(),
            value: "512M\n[evil]".to_string(),
            enabled: true,
            section: "Limits".to_string(),
            custom: false,
        }];
        assert!(matches!(
            save_php_ini_directives_inner(&supervisor, "php_83", bad_val).await,
            Err(RakitrayError::InvalidInput(_))
        ));

        let good_directives = vec![
            PhpIniDirective {
                key: "memory_limit".to_string(),
                value: "1024M".to_string(),
                enabled: true,
                section: "Limits & Uploads".to_string(),
                custom: false,
            },
            PhpIniDirective {
                key: "custom_flag".to_string(),
                value: "hello".to_string(),
                enabled: false,
                section: "Custom".to_string(),
                custom: true,
            },
        ];

        let updated = save_php_ini_directives_inner(&supervisor, "php_83", good_directives)
            .await
            .unwrap();

        assert_eq!(updated.limits.memory_limit, "1024M");
        assert!(updated
            .directives
            .iter()
            .any(|d| d.key == "memory_limit" && d.value == "1024M" && d.enabled));
        assert!(updated
            .directives
            .iter()
            .any(|d| d.key == "custom_flag" && !d.enabled));

        let gen_ini = root.join("config/php/8.3.14-generated.ini");
        let ini_content = fs::read_to_string(gen_ini).unwrap();
        assert!(ini_content.contains("memory_limit = 1024M"));
        assert!(ini_content.contains("; custom_flag = hello"));

        let _ = fs::remove_dir_all(root);
    }
}
