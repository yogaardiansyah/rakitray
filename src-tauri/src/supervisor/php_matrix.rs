use crate::discovery::php_ext::scan;
use crate::drivers::php_matrix_store::{PhpMatrixState, PhpMatrixStore};
use crate::models::{PhpIniLimits, PhpMatrix, PhpMatrixEvent, PhpProfileMode, ServiceInstance};
use crate::supervisor::SupervisorError;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot, RwLock};

#[derive(Debug, Clone)]
pub enum PhpMatrixChange {
    ToggleExtension {
        ext_name: String,
        enabled: bool,
    },
    ApplyProfile {
        profile: PhpProfileMode,
        limits: PhpIniLimits,
    },
    SetExtensions {
        enabled_exts: Vec<String>,
        label: String,
        extra_warnings: Vec<String>,
    },
    ReplaceState {
        state: PhpMatrixState,
        label: String,
        extra_warnings: Vec<String>,
    },
    UpdateDirectives {
        directives: Vec<crate::models::PhpIniDirective>,
    },
}

impl PhpMatrixChange {
    pub fn description(&self) -> String {
        match self {
            Self::ToggleExtension { ext_name, enabled } => {
                if *enabled {
                    format!("enabled '{ext_name}'")
                } else {
                    format!("disabled '{ext_name}'")
                }
            }
            Self::ApplyProfile { profile, .. } => format!("profile '{profile:?}'"),
            Self::SetExtensions { label, .. } => label.clone(),
            Self::ReplaceState { label, .. } => label.clone(),
            Self::UpdateDirectives { .. } => "updated php.ini directives".to_string(),
        }
    }

    pub fn extra_warnings(&self) -> Vec<String> {
        match self {
            Self::SetExtensions { extra_warnings, .. } => extra_warnings.clone(),
            Self::ReplaceState { extra_warnings, .. } => extra_warnings.clone(),
            _ => Vec::new(),
        }
    }

    pub fn apply_to(&self, state: &mut PhpMatrixState) {
        match self {
            Self::ToggleExtension { ext_name, enabled } => {
                let lower = ext_name.trim().to_lowercase();
                if *enabled {
                    if !state.enabled_exts.contains(&lower) {
                        state.enabled_exts.push(lower);
                    }
                } else {
                    state.enabled_exts.retain(|e| e != &lower);
                }
            }
            Self::ApplyProfile { profile, limits } => {
                state.profile = *profile;
                state.limits = limits.clone();
                state.directives =
                    crate::drivers::php_fpm::default_directives_for_profile(*profile, limits);
            }
            Self::SetExtensions { enabled_exts, .. } => {
                state.enabled_exts = enabled_exts.clone();
            }
            Self::ReplaceState {
                state: new_state, ..
            } => {
                *state = new_state.clone();
            }
            Self::UpdateDirectives { directives } => {
                for d in directives {
                    if d.key == "memory_limit" && !d.value.trim().is_empty() {
                        state.limits.memory_limit = d.value.clone();
                    } else if d.key == "upload_max_filesize" && !d.value.trim().is_empty() {
                        state.limits.upload_max_filesize = d.value.clone();
                    } else if d.key == "post_max_size" && !d.value.trim().is_empty() {
                        state.limits.post_max_size = d.value.clone();
                    } else if d.key == "max_execution_time" {
                        if let Ok(v) = d.value.trim().parse::<u32>() {
                            state.limits.max_execution_time = v;
                        }
                    }
                }
                state.directives = directives.clone();
            }
        }
    }
}

#[async_trait::async_trait]
pub trait MatrixSupervisorBackend: Send + Sync {
    async fn resolve_version(
        &self,
        instance_id: &str,
    ) -> Result<(String, PhpProfileMode), SupervisorError>;
    async fn find_instances_for_version(&self, version: &str) -> Vec<ServiceInstance>;
    async fn restart_instance(&self, instance_id: &str) -> Result<(), SupervisorError>;
    async fn emit_log(&self, instance_id: &str, line: String);
    async fn emit_event(&self, event: PhpMatrixEvent);
    async fn sync_version_profile(&self, version: &str, profile: PhpProfileMode);
    fn config_php_dir(&self) -> PathBuf;
    fn resolve_php_root(&self, version: &str) -> Option<PathBuf>;
    fn resolve_php_binary(&self, version: &str) -> Option<PathBuf>;
    fn resolve_ext_dir(&self, version: &str) -> Option<PathBuf>;
}

struct QueuedRequest {
    instance_id: String,
    change: PhpMatrixChange,
    responder: oneshot::Sender<Result<PhpMatrix, SupervisorError>>,
}

struct VersionBatcher {
    tx: mpsc::Sender<QueuedRequest>,
}

pub struct MatrixOrchestrator {
    backend: Arc<dyn MatrixSupervisorBackend>,
    store: PhpMatrixStore,
    debounce_duration: Duration,
    batchers: Arc<RwLock<HashMap<String, VersionBatcher>>>,
}

pub fn default_simulated_extensions(
    enabled_set: &HashSet<String>,
) -> Vec<crate::models::PhpExtension> {
    use crate::models::{ExtensionCategory, ExtensionKind, ExtensionSource, PhpExtension};
    let list = vec![
        (
            "ctype",
            ExtensionKind::Extension,
            ExtensionSource::BuiltIn,
            ExtensionCategory::LaravelCore,
            true,
            false,
            vec![],
        ),
        (
            "curl",
            ExtensionKind::Extension,
            ExtensionSource::SharedLib,
            ExtensionCategory::LaravelCore,
            true,
            true,
            vec![],
        ),
        (
            "dom",
            ExtensionKind::Extension,
            ExtensionSource::BuiltIn,
            ExtensionCategory::LaravelCore,
            true,
            false,
            vec![],
        ),
        (
            "fileinfo",
            ExtensionKind::Extension,
            ExtensionSource::BuiltIn,
            ExtensionCategory::LaravelCore,
            true,
            false,
            vec![],
        ),
        (
            "filter",
            ExtensionKind::Extension,
            ExtensionSource::BuiltIn,
            ExtensionCategory::LaravelCore,
            true,
            false,
            vec![],
        ),
        (
            "hash",
            ExtensionKind::Extension,
            ExtensionSource::BuiltIn,
            ExtensionCategory::LaravelCore,
            true,
            false,
            vec![],
        ),
        (
            "mbstring",
            ExtensionKind::Extension,
            ExtensionSource::SharedLib,
            ExtensionCategory::LaravelCore,
            true,
            true,
            vec![],
        ),
        (
            "openssl",
            ExtensionKind::Extension,
            ExtensionSource::SharedLib,
            ExtensionCategory::LaravelCore,
            true,
            true,
            vec![],
        ),
        (
            "pcre",
            ExtensionKind::Extension,
            ExtensionSource::BuiltIn,
            ExtensionCategory::LaravelCore,
            true,
            false,
            vec![],
        ),
        (
            "pdo",
            ExtensionKind::Extension,
            ExtensionSource::BuiltIn,
            ExtensionCategory::LaravelCore,
            true,
            false,
            vec![],
        ),
        (
            "session",
            ExtensionKind::Extension,
            ExtensionSource::BuiltIn,
            ExtensionCategory::LaravelCore,
            true,
            false,
            vec![],
        ),
        (
            "tokenizer",
            ExtensionKind::Extension,
            ExtensionSource::BuiltIn,
            ExtensionCategory::LaravelCore,
            true,
            false,
            vec![],
        ),
        (
            "xml",
            ExtensionKind::Extension,
            ExtensionSource::BuiltIn,
            ExtensionCategory::LaravelCore,
            true,
            false,
            vec![],
        ),
        (
            "zip",
            ExtensionKind::Extension,
            ExtensionSource::SharedLib,
            ExtensionCategory::LaravelCore,
            true,
            true,
            vec![],
        ),
        (
            "bcmath",
            ExtensionKind::Extension,
            ExtensionSource::SharedLib,
            ExtensionCategory::LaravelCore,
            true,
            true,
            vec![],
        ),
        (
            "pdo_mysql",
            ExtensionKind::Extension,
            ExtensionSource::SharedLib,
            ExtensionCategory::DatabaseCache,
            true,
            true,
            vec![],
        ),
        (
            "pdo_pgsql",
            ExtensionKind::Extension,
            ExtensionSource::SharedLib,
            ExtensionCategory::DatabaseCache,
            false,
            true,
            vec![],
        ),
        (
            "pdo_sqlite",
            ExtensionKind::Extension,
            ExtensionSource::SharedLib,
            ExtensionCategory::DatabaseCache,
            false,
            true,
            vec![],
        ),
        (
            "mysqli",
            ExtensionKind::Extension,
            ExtensionSource::SharedLib,
            ExtensionCategory::DatabaseCache,
            false,
            true,
            vec![],
        ),
        (
            "pgsql",
            ExtensionKind::Extension,
            ExtensionSource::SharedLib,
            ExtensionCategory::DatabaseCache,
            false,
            true,
            vec![],
        ),
        (
            "sqlite3",
            ExtensionKind::Extension,
            ExtensionSource::SharedLib,
            ExtensionCategory::DatabaseCache,
            false,
            true,
            vec![],
        ),
        (
            "redis",
            ExtensionKind::Extension,
            ExtensionSource::SharedLib,
            ExtensionCategory::DatabaseCache,
            true,
            true,
            vec![],
        ),
        (
            "memcached",
            ExtensionKind::Extension,
            ExtensionSource::SharedLib,
            ExtensionCategory::DatabaseCache,
            false,
            true,
            vec![],
        ),
        (
            "opcache",
            ExtensionKind::ZendExtension,
            ExtensionSource::BuiltIn,
            ExtensionCategory::PerformanceDebug,
            true,
            false,
            vec![],
        ),
        (
            "xdebug",
            ExtensionKind::ZendExtension,
            ExtensionSource::SharedLib,
            ExtensionCategory::PerformanceDebug,
            false,
            true,
            vec![],
        ),
        (
            "pcov",
            ExtensionKind::ZendExtension,
            ExtensionSource::SharedLib,
            ExtensionCategory::PerformanceDebug,
            false,
            true,
            vec![],
        ),
        (
            "exif",
            ExtensionKind::Extension,
            ExtensionSource::SharedLib,
            ExtensionCategory::MediaFormat,
            true,
            true,
            vec!["mbstring".to_string()],
        ),
        (
            "gd",
            ExtensionKind::Extension,
            ExtensionSource::SharedLib,
            ExtensionCategory::MediaFormat,
            true,
            true,
            vec![],
        ),
        (
            "intl",
            ExtensionKind::Extension,
            ExtensionSource::SharedLib,
            ExtensionCategory::MediaFormat,
            true,
            true,
            vec![],
        ),
        (
            "iconv",
            ExtensionKind::Extension,
            ExtensionSource::SharedLib,
            ExtensionCategory::MediaFormat,
            false,
            true,
            vec![],
        ),
        (
            "gettext",
            ExtensionKind::Extension,
            ExtensionSource::SharedLib,
            ExtensionCategory::Other,
            false,
            true,
            vec![],
        ),
        (
            "sockets",
            ExtensionKind::Extension,
            ExtensionSource::SharedLib,
            ExtensionCategory::Other,
            false,
            true,
            vec![],
        ),
        (
            "sodium",
            ExtensionKind::Extension,
            ExtensionSource::SharedLib,
            ExtensionCategory::Other,
            false,
            true,
            vec![],
        ),
    ];

    list.into_iter()
        .map(
            |(name, kind, source, category, default_enabled, toggleable, requires)| {
                let enabled = if source == ExtensionSource::BuiltIn {
                    true
                } else if enabled_set.is_empty() {
                    default_enabled
                } else {
                    enabled_set.contains(name)
                };
                PhpExtension {
                    name: name.to_string(),
                    kind,
                    source,
                    category,
                    enabled,
                    toggleable,
                    requires,
                }
            },
        )
        .collect()
}

impl MatrixOrchestrator {
    pub fn new(backend: Arc<dyn MatrixSupervisorBackend>, debounce_duration: Duration) -> Self {
        Self {
            backend,
            store: PhpMatrixStore::new(),
            debounce_duration,
            batchers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn get_matrix(&self, instance_id: &str) -> Result<PhpMatrix, SupervisorError> {
        let (version, default_profile) = self.backend.resolve_version(instance_id).await?;
        let config_dir = self.backend.config_php_dir();
        let state = self
            .store
            .read_state(&config_dir, &version, default_profile)
            .await
            .map_err(|e| SupervisorError::Driver(e.to_string()))?;

        let php_root = self
            .backend
            .resolve_php_root(&version)
            .unwrap_or_else(|| PathBuf::from("."));
        let enabled_set: HashSet<String> = state.enabled_exts.iter().cloned().collect();
        let scan_outcome = scan(&php_root, &enabled_set).await;

        let ext_dir = self.backend.resolve_ext_dir(&version);
        let mut all_warnings = Vec::new();
        if let Ok(rendered) = crate::drivers::render_ini(
            &version,
            state.profile,
            &state.limits,
            &state.enabled_exts,
            ext_dir.as_deref(),
        ) {
            all_warnings.extend(rendered.warnings);
        }
        all_warnings.extend(scan_outcome.warnings);

        let extensions = if scan_outcome.extensions.is_empty() {
            default_simulated_extensions(&enabled_set)
        } else {
            scan_outcome.extensions
        };

        Ok(PhpMatrix {
            instance_id: instance_id.to_string(),
            version,
            profile: state.profile,
            limits: state.limits,
            directives: state.directives,
            extensions,
            warnings: all_warnings,
        })
    }

    pub async fn apply_matrix(
        &self,
        instance_id: &str,
        change: PhpMatrixChange,
    ) -> Result<PhpMatrix, SupervisorError> {
        let (version, _) = self.backend.resolve_version(instance_id).await?;

        let (tx, rx) = oneshot::channel();
        let mut queued = QueuedRequest {
            instance_id: instance_id.to_string(),
            change,
            responder: tx,
        };

        {
            let mut map = self.batchers.write().await;
            if let Some(batcher) = map.get(&version) {
                match batcher.tx.send(queued).await {
                    Ok(_) => {
                        drop(map);
                        return rx.await.unwrap_or_else(|_| {
                            Err(SupervisorError::Driver(
                                "Batch worker dropped channel".to_string(),
                            ))
                        });
                    }
                    Err(send_err) => {
                        queued = send_err.0;
                    }
                }
            }

            let (batch_tx, batch_rx) = mpsc::channel(64);
            let _ = batch_tx.send(queued).await;

            let worker_backend = self.backend.clone();
            let worker_store = self.store.clone();
            let worker_version = version.clone();
            let debounce = self.debounce_duration;

            tokio::spawn(async move {
                Self::run_version_worker(
                    worker_version,
                    batch_rx,
                    worker_backend,
                    worker_store,
                    debounce,
                )
                .await;
            });

            map.insert(version, VersionBatcher { tx: batch_tx });
        }

        rx.await.unwrap_or_else(|_| {
            Err(SupervisorError::Driver(
                "Batch worker dropped response".to_string(),
            ))
        })
    }

    async fn run_version_worker(
        version: String,
        mut rx: mpsc::Receiver<QueuedRequest>,
        backend: Arc<dyn MatrixSupervisorBackend>,
        store: PhpMatrixStore,
        debounce: Duration,
    ) {
        while let Some(first_req) = rx.recv().await {
            let mut requests = vec![first_req];

            let mut timer = tokio::time::interval(debounce);
            timer.tick().await;

            loop {
                tokio::select! {
                    Some(req) = rx.recv() => {
                        requests.push(req);
                    }
                    _ = timer.tick() => {
                        break;
                    }
                }
            }

            let last_instance_id = requests
                .last()
                .map(|r| r.instance_id.clone())
                .unwrap_or_default();
            let mut change_descriptions = Vec::new();

            let config_dir = backend.config_php_dir();
            let initial_state_res = store
                .read_state(&config_dir, &version, PhpProfileMode::Dev)
                .await;

            let mut current_state = match initial_state_res {
                Ok(s) => s,
                Err(e) => {
                    let err_msg = e.to_string();
                    for req in requests {
                        let _ = req
                            .responder
                            .send(Err(SupervisorError::Driver(err_msg.clone())));
                    }
                    continue;
                }
            };

            let mut extra_warnings_accum = Vec::new();

            for req in &requests {
                change_descriptions.push(req.change.description());
                extra_warnings_accum.extend(req.change.extra_warnings());
                req.change.apply_to(&mut current_state);
            }

            backend
                .emit_event(PhpMatrixEvent::Applying {
                    instance_id: last_instance_id.clone(),
                    version: version.clone(),
                })
                .await;

            let php_bin = backend.resolve_php_binary(&version);
            let ext_dir = backend.resolve_ext_dir(&version);

            let write_res = store
                .write_matrix(
                    &config_dir,
                    &version,
                    &current_state,
                    php_bin.as_deref(),
                    ext_dir.as_deref(),
                )
                .await;

            let rendered_warnings = match write_res {
                Ok(rendered) => {
                    backend
                        .sync_version_profile(&version, current_state.profile)
                        .await;
                    rendered.warnings
                }
                Err(e) => {
                    let err_msg = format!("Validation or write failed: {e}");
                    backend
                        .emit_event(PhpMatrixEvent::RolledBack {
                            instance_id: last_instance_id.clone(),
                            version: version.clone(),
                            reason: e.to_string(),
                        })
                        .await;
                    for req in requests {
                        let _ = req
                            .responder
                            .send(Err(SupervisorError::Driver(err_msg.clone())));
                    }
                    continue;
                }
            };

            let instances = backend.find_instances_for_version(&version).await;
            let running_instances: Vec<String> = instances
                .iter()
                .filter(|i| i.status == crate::models::ServiceStatus::Running)
                .map(|i| i.instance_id.clone())
                .collect();

            let mut restart_failed = false;
            let mut restart_err = String::new();

            for inst_id in &running_instances {
                if let Err(e) = backend.restart_instance(inst_id).await {
                    restart_failed = true;
                    restart_err = e.to_string();
                    break;
                }
            }

            if restart_failed {
                let _ = store.rollback_matrix(&config_dir, &version).await;

                for inst_id in &running_instances {
                    let _ = backend.restart_instance(inst_id).await;
                }

                backend
                    .emit_event(PhpMatrixEvent::RolledBack {
                        instance_id: last_instance_id.clone(),
                        version: version.clone(),
                        reason: format!("Restart failed: {restart_err}"),
                    })
                    .await;

                let err_msg = format!("Worker restart failed, rolled back: {restart_err}");
                for req in requests {
                    let _ = req
                        .responder
                        .send(Err(SupervisorError::Driver(err_msg.clone())));
                }
                continue;
            }

            let what = change_descriptions.join(", ");
            backend
                .emit_log(
                    &last_instance_id,
                    format!("[PHP Matrix] Applied {what} to php_{version} & reloaded worker"),
                )
                .await;

            backend
                .emit_event(PhpMatrixEvent::Reloaded {
                    instance_id: last_instance_id.clone(),
                    version: version.clone(),
                })
                .await;

            let php_root = backend
                .resolve_php_root(&version)
                .unwrap_or_else(|| PathBuf::from("."));
            let enabled_set: HashSet<String> = current_state.enabled_exts.iter().cloned().collect();
            let scan_outcome = scan(&php_root, &enabled_set).await;

            let mut all_warnings = rendered_warnings;
            all_warnings.extend(extra_warnings_accum);
            all_warnings.extend(scan_outcome.warnings);

            let extensions = if scan_outcome.extensions.is_empty() {
                default_simulated_extensions(&enabled_set)
            } else {
                scan_outcome.extensions
            };

            let matrix = PhpMatrix {
                instance_id: last_instance_id,
                version: version.clone(),
                profile: current_state.profile,
                limits: current_state.limits,
                directives: current_state.directives,
                extensions,
                warnings: all_warnings,
            };

            for req in requests {
                let _ = req.responder.send(Ok(matrix.clone()));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ServiceCategory, ServiceStatus};
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use tokio::sync::Mutex;

    struct MockBackend {
        version: String,
        instances: Mutex<Vec<ServiceInstance>>,
        fail_restart: AtomicBool,
        restart_count: AtomicUsize,
        logs: Mutex<Vec<String>>,
        events: Mutex<Vec<PhpMatrixEvent>>,
        temp_dir: PathBuf,
    }

    impl MockBackend {
        fn new(temp_dir: PathBuf) -> Self {
            Self {
                version: "8.3.14".to_string(),
                instances: Mutex::new(vec![ServiceInstance {
                    instance_id: "php_83".to_string(),
                    engine: "php".to_string(),
                    display_name: "PHP 8.3".to_string(),
                    category: ServiceCategory::Runtime,
                    version: "8.3.14".to_string(),
                    profile: "dev".to_string(),
                    status: ServiceStatus::Running,
                    pid: Some(1234),
                    port: Some(9000),
                    socket_path: None,
                    memory_bytes: 0,
                    cpu_percent: 0.0,
                    uptime_secs: 0,
                    pinned_to_rack: true,
                    last_error: None,
                }]),
                fail_restart: AtomicBool::new(false),
                restart_count: AtomicUsize::new(0),
                logs: Mutex::new(Vec::new()),
                events: Mutex::new(Vec::new()),
                temp_dir,
            }
        }
    }

    #[async_trait::async_trait]
    impl MatrixSupervisorBackend for MockBackend {
        async fn resolve_version(
            &self,
            _instance_id: &str,
        ) -> Result<(String, PhpProfileMode), SupervisorError> {
            Ok((self.version.clone(), PhpProfileMode::Dev))
        }

        async fn find_instances_for_version(&self, version: &str) -> Vec<ServiceInstance> {
            let list = self.instances.lock().await;
            list.iter()
                .filter(|i| i.version == version)
                .cloned()
                .collect()
        }

        async fn restart_instance(&self, _instance_id: &str) -> Result<(), SupervisorError> {
            self.restart_count.fetch_add(1, Ordering::SeqCst);
            if self.fail_restart.load(Ordering::SeqCst) {
                return Err(SupervisorError::Driver(
                    "simulated process crash on boot".to_string(),
                ));
            }
            Ok(())
        }

        async fn emit_log(&self, _instance_id: &str, line: String) {
            self.logs.lock().await.push(line);
        }

        async fn emit_event(&self, event: PhpMatrixEvent) {
            self.events.lock().await.push(event);
        }

        async fn sync_version_profile(&self, version: &str, profile: PhpProfileMode) {
            let mut list = self.instances.lock().await;
            for inst in list.iter_mut() {
                if inst.version == version {
                    inst.profile = match profile {
                        PhpProfileMode::Dev => "dev".to_string(),
                        PhpProfileMode::Prod => "prod".to_string(),
                    };
                }
            }
        }

        fn config_php_dir(&self) -> PathBuf {
            self.temp_dir.join("config/php")
        }

        fn resolve_php_root(&self, _version: &str) -> Option<PathBuf> {
            Some(self.temp_dir.clone())
        }

        fn resolve_php_binary(&self, _version: &str) -> Option<PathBuf> {
            None
        }

        fn resolve_ext_dir(&self, _version: &str) -> Option<PathBuf> {
            None
        }
    }

    #[tokio::test]
    async fn apply_matrix_operation_order_and_debounce() {
        let temp = std::env::temp_dir().join(format!("test_orch_debounce_{}", std::process::id()));
        std::fs::create_dir_all(&temp).unwrap();

        let backend = Arc::new(MockBackend::new(temp.clone()));
        let orchestrator = MatrixOrchestrator::new(backend.clone(), Duration::from_millis(50));

        let f1 = orchestrator.apply_matrix(
            "php_83",
            PhpMatrixChange::ToggleExtension {
                ext_name: "curl".to_string(),
                enabled: true,
            },
        );
        let f2 = orchestrator.apply_matrix(
            "php_83",
            PhpMatrixChange::ToggleExtension {
                ext_name: "mbstring".to_string(),
                enabled: true,
            },
        );

        let (res1, res2) = tokio::join!(f1, f2);
        let matrix1 = res1.unwrap();
        let matrix2 = res2.unwrap();

        assert_eq!(matrix1.version, "8.3.14");
        assert_eq!(matrix2.version, "8.3.14");

        assert_eq!(backend.restart_count.load(Ordering::SeqCst), 1);

        let logs = backend.logs.lock().await;
        assert_eq!(logs.len(), 1);
        assert!(logs[0].contains("[PHP Matrix] Applied"));
        assert!(logs[0].contains("reloaded worker"));

        let events = backend.events.lock().await;
        assert_eq!(events.len(), 2);
        match &events[0] {
            PhpMatrixEvent::Applying { version, .. } => assert_eq!(version, "8.3.14"),
            _ => panic!("Expected Applying"),
        }
        match &events[1] {
            PhpMatrixEvent::Reloaded { version, .. } => assert_eq!(version, "8.3.14"),
            _ => panic!("Expected Reloaded"),
        }

        let _ = std::fs::remove_dir_all(temp);
    }

    #[tokio::test]
    async fn apply_matrix_rollback_on_restart_failure() {
        let temp = std::env::temp_dir().join(format!("test_orch_rollback_{}", std::process::id()));
        std::fs::create_dir_all(&temp).unwrap();

        let backend = Arc::new(MockBackend::new(temp.clone()));
        let orchestrator = MatrixOrchestrator::new(backend.clone(), Duration::from_millis(10));

        let _ = orchestrator
            .apply_matrix(
                "php_83",
                PhpMatrixChange::ToggleExtension {
                    ext_name: "curl".to_string(),
                    enabled: true,
                },
            )
            .await
            .unwrap();

        backend.fail_restart.store(true, Ordering::SeqCst);

        let fail_res = orchestrator
            .apply_matrix(
                "php_83",
                PhpMatrixChange::ToggleExtension {
                    ext_name: "bad_ext".to_string(),
                    enabled: true,
                },
            )
            .await;

        assert!(fail_res.is_err());

        let events = backend.events.lock().await;
        let last_event = events.last().unwrap();
        match last_event {
            PhpMatrixEvent::RolledBack { reason, .. } => {
                assert!(reason.contains("simulated process crash"));
            }
            _ => panic!("Expected RolledBack event, got {last_event:?}"),
        }

        let _ = std::fs::remove_dir_all(temp);
    }
}
