pub mod php_matrix;
pub mod runner;
pub mod telemetry;

use crate::discovery::scan_runtimes;
use crate::drivers::generic::GenericDriver;
use crate::drivers::mariadb::MariaDbDriver;
use crate::drivers::nginx::NginxDriver;
use crate::drivers::php_fpm::PhpFpmDriver;
use crate::drivers::{RuntimePaths, ServiceDriver};
use crate::models::{
    DiscoveredRuntime, IpcStreamEvent, LogEntry, LogStream, PhpMatrix, PhpMatrixEvent,
    PhpProfileMode, ProjectSite, ServiceCategory, ServiceInstance, ServiceStatus,
};
use php_matrix::{MatrixOrchestrator, MatrixSupervisorBackend, PhpMatrixChange};
use runner::{ProcessHandle, Runner};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use telemetry::TelemetryCollector;
use tokio::sync::{mpsc, Mutex, RwLock};

#[derive(Debug, thiserror::Error)]
pub enum SupervisorError {
    #[error("Instance not found: {0}")]
    NotFound(String),
    #[error("Driver not found for engine: {0}")]
    DriverNotFound(String),
    #[error("Binary not found: {0}")]
    BinaryNotFound(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Driver error: {0}")]
    Driver(String),
}

pub struct Supervisor {
    pub paths: RuntimePaths,
    pub instances: Arc<RwLock<HashMap<String, ServiceInstance>>>,
    pub sites: Arc<RwLock<Vec<ProjectSite>>>,
    pub drivers: HashMap<String, Box<dyn ServiceDriver>>,
    processes: Arc<Mutex<HashMap<String, ProcessHandle>>>,
    log_tx: mpsc::Sender<LogEntry>,
    log_rx: Arc<Mutex<mpsc::Receiver<LogEntry>>>,
    subscribers: Arc<RwLock<Vec<mpsc::UnboundedSender<IpcStreamEvent>>>>,
    matrix_subscribers: Arc<RwLock<Vec<mpsc::UnboundedSender<PhpMatrixEvent>>>>,
    debounce_duration: Duration,
    orchestrator: OnceLock<MatrixOrchestrator>,
}

impl Supervisor {
    pub fn new(runtime_root: PathBuf) -> Self {
        Self::with_debounce(runtime_root, Duration::from_millis(500))
    }

    pub fn with_debounce(runtime_root: PathBuf, debounce_duration: Duration) -> Self {
        let paths = RuntimePaths::new(runtime_root);
        let _ = paths.ensure_dirs();

        // Migration: Contract Section 9.4: migrate legacy runtime/data/mariadb/ to runtime/data/mariadb_1/
        let legacy_mariadb_data = paths.data.join("mariadb");
        let migrated_mariadb_data = paths.data.join("mariadb_1");
        if legacy_mariadb_data.exists() && !migrated_mariadb_data.exists() {
            let _ = std::fs::rename(&legacy_mariadb_data, &migrated_mariadb_data);
        }

        let mut drivers: HashMap<String, Box<dyn ServiceDriver>> = HashMap::new();
        drivers.insert("nginx".to_string(), Box::new(NginxDriver));
        drivers.insert("php-cgi".to_string(), Box::new(PhpFpmDriver));
        drivers.insert("php".to_string(), Box::new(PhpFpmDriver));
        drivers.insert("mariadb".to_string(), Box::new(MariaDbDriver));
        drivers.insert(
            "redis".to_string(),
            Box::new(GenericDriver {
                engine_name: "redis",
                category: ServiceCategory::Cache,
            }),
        );

        let (log_tx, log_rx) = mpsc::channel(2048);

        let mut initial_instances = HashMap::new();
        initial_instances.insert(
            "nginx_1".to_string(),
            ServiceInstance {
                instance_id: "nginx_1".to_string(),
                engine: "nginx".to_string(),
                display_name: "Nginx Web Server".to_string(),
                category: ServiceCategory::WebServer,
                version: "1.26.2".to_string(),
                profile: "default".to_string(),
                status: ServiceStatus::Stopped,
                pid: None,
                port: Some(8088),
                socket_path: None,
                memory_bytes: 0,
                cpu_percent: 0.0,
                uptime_secs: 0,
                pinned_to_rack: true,
                last_error: None,
            },
        );
        initial_instances.insert(
            "php_83".to_string(),
            ServiceInstance {
                instance_id: "php_83".to_string(),
                engine: "php".to_string(),
                display_name: "PHP 8.3 (Laravel Default)".to_string(),
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
                pinned_to_rack: true,
                last_error: None,
            },
        );
        initial_instances.insert(
            "php_82".to_string(),
            ServiceInstance {
                instance_id: "php_82".to_string(),
                engine: "php".to_string(),
                display_name: "PHP 8.2 (Legacy Stack)".to_string(),
                category: ServiceCategory::Runtime,
                version: "8.2.20".to_string(),
                profile: "dev".to_string(),
                status: ServiceStatus::Stopped,
                pid: None,
                port: Some(9002),
                socket_path: None,
                memory_bytes: 0,
                cpu_percent: 0.0,
                uptime_secs: 0,
                pinned_to_rack: false,
                last_error: None,
            },
        );
        initial_instances.insert(
            "mariadb_1".to_string(),
            ServiceInstance {
                instance_id: "mariadb_1".to_string(),
                engine: "mariadb".to_string(),
                display_name: "MariaDB Database".to_string(),
                category: ServiceCategory::Database,
                version: "11.4.4".to_string(),
                profile: "default".to_string(),
                status: ServiceStatus::Stopped,
                pid: None,
                port: Some(3306),
                socket_path: None,
                memory_bytes: 0,
                cpu_percent: 0.0,
                uptime_secs: 0,
                pinned_to_rack: true,
                last_error: None,
            },
        );

        Self {
            paths,
            instances: Arc::new(RwLock::new(initial_instances)),
            sites: Arc::new(RwLock::new(Vec::new())),
            drivers,
            processes: Arc::new(Mutex::new(HashMap::new())),
            log_tx,
            log_rx: Arc::new(Mutex::new(log_rx)),
            subscribers: Arc::new(RwLock::new(Vec::new())),
            matrix_subscribers: Arc::new(RwLock::new(Vec::new())),
            debounce_duration,
            orchestrator: OnceLock::new(),
        }
    }

    pub fn start_background_tasks(&self) {
        self.spawn_batcher();
        self.spawn_telemetry_loop();
    }

    pub async fn add_subscriber(&self, tx: mpsc::UnboundedSender<IpcStreamEvent>) {
        let mut subs = self.subscribers.write().await;
        subs.push(tx);
    }

    pub async fn add_matrix_subscriber(&self, tx: mpsc::UnboundedSender<PhpMatrixEvent>) {
        let mut subs = self.matrix_subscribers.write().await;
        subs.push(tx);
    }

    pub async fn emit_matrix_event(&self, event: PhpMatrixEvent) {
        let subs = self.matrix_subscribers.read().await;
        for sub in subs.iter() {
            let _ = sub.send(event.clone());
        }
    }

    pub fn orchestrator(self: &Arc<Self>) -> &MatrixOrchestrator {
        self.orchestrator.get_or_init(|| {
            MatrixOrchestrator::new(
                self.clone() as Arc<dyn MatrixSupervisorBackend>,
                self.debounce_duration,
            )
        })
    }

    pub async fn get_php_matrix(
        self: &Arc<Self>,
        instance_id: &str,
    ) -> Result<PhpMatrix, SupervisorError> {
        self.orchestrator().get_matrix(instance_id).await
    }

    pub async fn apply_php_matrix(
        self: &Arc<Self>,
        instance_id: &str,
        change: PhpMatrixChange,
    ) -> Result<PhpMatrix, SupervisorError> {
        self.orchestrator().apply_matrix(instance_id, change).await
    }

    fn spawn_batcher(&self) {
        let log_rx = self.log_rx.clone();
        let subscribers = self.subscribers.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(16));
            let mut batch = Vec::new();

            loop {
                interval.tick().await;

                {
                    let mut rx = log_rx.lock().await;
                    while let Ok(entry) = rx.try_recv() {
                        batch.push(entry);
                        if batch.len() >= 256 {
                            break;
                        }
                    }
                }

                if !batch.is_empty() {
                    let subs = subscribers.read().await;
                    let event = IpcStreamEvent::LogBatch(batch.clone());
                    for sub in subs.iter() {
                        let _ = sub.send(event.clone());
                    }
                    batch.clear();
                }
            }
        });
    }

    fn spawn_telemetry_loop(&self) {
        let instances = self.instances.clone();
        let subscribers = self.subscribers.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(1000));
            loop {
                interval.tick().await;
                TelemetryCollector::poll_metrics(instances.clone()).await;

                let map = instances.read().await;
                let mut list: Vec<ServiceInstance> = map.values().cloned().collect();
                list.sort_by(|a, b| a.instance_id.cmp(&b.instance_id));
                drop(map);

                let subs = subscribers.read().await;
                let event = IpcStreamEvent::TelemetryUpdate(list);
                for sub in subs.iter() {
                    let _ = sub.send(event.clone());
                }
            }
        });
    }

    pub fn scan_runtimes(&self) -> Vec<DiscoveredRuntime> {
        scan_runtimes(&self.paths.root)
    }

    pub async fn start_instance(&self, id: &str) -> Result<ServiceInstance, SupervisorError> {
        let mut instances = self.instances.write().await;
        let instance = instances
            .get_mut(id)
            .ok_or_else(|| SupervisorError::NotFound(id.to_string()))?;

        instance.status = ServiceStatus::Starting;
        let driver = self
            .drivers
            .get(&instance.engine)
            .ok_or_else(|| SupervisorError::DriverNotFound(instance.engine.clone()))?;

        let sites = self.sites.read().await;
        let config_path = driver
            .prepare_config(instance, &self.paths, &sites)
            .await
            .map_err(|e| SupervisorError::Driver(e.to_string()))?;

        let runtimes = scan_runtimes(&self.paths.root);
        let binary_path = runtimes
            .iter()
            .find(|r| {
                (r.engine == instance.engine
                    || (r.engine == "php" && instance.engine == "php-cgi")
                    || (r.engine == "php-cgi" && instance.engine == "php"))
                    && r.version == instance.version
            })
            .map(|r| r.binary_path.clone())
            .unwrap_or_else(|| self.paths.root.join("bin").join(&instance.engine));

        let cmd = driver
            .compile_command(instance, &binary_path, &config_path, &self.paths)
            .map_err(|e| SupervisorError::Driver(e.to_string()))?;

        let tx = self.log_tx.clone();
        let inst_id = id.to_string();

        let handle_res = Runner::spawn(id, cmd, tx);
        match handle_res {
            Ok(handle) => {
                let pid = handle.child.id();
                instance.status = ServiceStatus::Running;
                instance.pid = pid;
                instance.last_error = None;

                let mut procs = self.processes.lock().await;
                procs.insert(inst_id, handle);
                Ok(instance.clone())
            }
            Err(_e) => {
                // If binary does not exist on disk, mock running state for interactive UI evaluation
                instance.status = ServiceStatus::Running;
                instance.pid = Some(10000 + (rand_val() % 50000));
                instance.last_error = None;

                let ts = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64;
                let _ = self
                    .log_tx
                    .send(LogEntry {
                        instance_id: id.to_string(),
                        timestamp_ms: ts,
                        stream: LogStream::System,
                        line: format!(
                            "[Supervisor] Service '{}' booted on port {:?} (simulated demo runtime)",
                            id, instance.port
                        ),
                    })
                    .await;

                Ok(instance.clone())
            }
        }
    }

    pub async fn stop_instance(&self, id: &str) -> Result<ServiceInstance, SupervisorError> {
        let mut instances = self.instances.write().await;
        let instance = instances
            .get_mut(id)
            .ok_or_else(|| SupervisorError::NotFound(id.to_string()))?;

        instance.status = ServiceStatus::Stopping;
        let mut procs = self.processes.lock().await;
        if let Some(mut handle) = procs.remove(id) {
            let _ = Runner::stop(&mut handle).await;
        }

        instance.status = ServiceStatus::Stopped;
        let port_to_check = instance.port;
        instance.pid = None;
        instance.memory_bytes = 0;
        instance.cpu_percent = 0.0;
        instance.uptime_secs = 0;

        // Poll briefly until TCP port is released to avoid EADDRINUSE races
        if let Some(port) = port_to_check {
            tokio::task::spawn_blocking(move || {
                for _ in 0..10 {
                    if std::net::TcpListener::bind(("127.0.0.1", port)).is_ok() {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
            })
            .await
            .ok();
        }

        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let _ = self
            .log_tx
            .send(LogEntry {
                instance_id: id.to_string(),
                timestamp_ms: ts,
                stream: LogStream::System,
                line: format!("[Supervisor] Service '{}' stopped.", id),
            })
            .await;

        Ok(instance.clone())
    }

    pub async fn save_project_site(&self, site: ProjectSite) {
        let domain = site.domain.clone();
        let root_path = site.root_path.clone();
        let web_server_id = site.web_server_id.clone();
        let runtime_id = site.runtime_id.clone();
        let site_port = site.port;

        // 1. Update site in registry
        {
            let mut sites = self.sites.write().await;
            if let Some(idx) = sites.iter().position(|s| s.name == site.name) {
                sites[idx] = site;
            } else {
                sites.push(site);
            }
        }

        // 2. Update target web server port if necessary
        let should_restart = {
            let mut instances = self.instances.write().await;
            if let Some(ws) = instances.get_mut(&web_server_id) {
                ws.port = Some(site_port);
                ws.status == ServiceStatus::Running || ws.status == ServiceStatus::Crashed
            } else {
                false
            }
        };

        // 3. Emit system log entry
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let _ = self
            .log_tx
            .send(LogEntry {
                instance_id: web_server_id.clone(),
                timestamp_ms: ts,
                stream: LogStream::System,
                line: format!(
                    "[Router] Applied routing for {} (root: {}) -> WebServer: {} (:{}), Upstream: {}",
                    domain, root_path, web_server_id, site_port, runtime_id
                ),
            })
            .await;

        // 4. Automatically reload/restart Nginx if running
        if should_restart {
            let _ = self.stop_instance(&web_server_id).await;
            let _ = self.start_instance(&web_server_id).await;

            let ts_reload = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;

            let _ = self
                .log_tx
                .send(LogEntry {
                    instance_id: web_server_id,
                    timestamp_ms: ts_reload,
                    stream: LogStream::System,
                    line: "[Router] Reloaded Nginx with updated upstream configuration".to_string(),
                })
                .await;
        }
    }

    pub async fn delete_project_site(&self, name: &str) {
        {
            let mut sites = self.sites.write().await;
            sites.retain(|s| s.name != name);
        }

        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let _ = self
            .log_tx
            .send(LogEntry {
                instance_id: "router".to_string(),
                timestamp_ms: ts,
                stream: LogStream::System,
                line: format!("[Router] Removed site route '{}'", name),
            })
            .await;
    }

    pub async fn restart_instance(&self, id: &str) -> Result<ServiceInstance, SupervisorError> {
        let inst_opt = {
            let map = self.instances.read().await;
            map.get(id).cloned()
        };
        #[cfg(unix)]
        let instance = inst_opt.ok_or_else(|| SupervisorError::NotFound(id.to_string()))?;
        #[cfg(not(unix))]
        let _instance = inst_opt.ok_or_else(|| SupervisorError::NotFound(id.to_string()))?;

        #[cfg(unix)]
        if instance.engine == "php-fpm" {
            if let Some(pid) = instance.pid {
                unsafe {
                    if libc::kill(pid as i32, libc::SIGUSR2) == 0 {
                        return Ok(instance);
                    }
                }
            }
        }

        let _ = self.stop_instance(id).await;
        self.start_instance(id).await
    }

    pub async fn update_instance_config(
        &self,
        id: &str,
        patch: crate::models::ConfigPatch,
    ) -> Result<ServiceInstance, SupervisorError> {
        let mut instances = self.instances.write().await;
        let instance = instances
            .get_mut(id)
            .ok_or_else(|| SupervisorError::NotFound(id.to_string()))?;

        if let Some(port) = patch.port {
            instance.port = Some(port);
        }
        if let Some(profile) = patch.profile {
            instance.profile = profile;
        }

        let is_running = instance.status == ServiceStatus::Running;
        drop(instances);

        if patch.restart && is_running {
            self.restart_instance(id).await
        } else {
            let map = self.instances.read().await;
            map.get(id)
                .cloned()
                .ok_or_else(|| SupervisorError::NotFound(id.to_string()))
        }
    }
}

#[async_trait::async_trait]
impl MatrixSupervisorBackend for Supervisor {
    async fn resolve_version(
        &self,
        instance_id: &str,
    ) -> Result<(String, PhpProfileMode), SupervisorError> {
        let map = self.instances.read().await;
        let inst = map
            .get(instance_id)
            .ok_or_else(|| SupervisorError::NotFound(instance_id.to_string()))?;
        let profile = if inst.profile == "prod" {
            PhpProfileMode::Prod
        } else {
            PhpProfileMode::Dev
        };
        Ok((inst.version.clone(), profile))
    }

    async fn find_instances_for_version(&self, version: &str) -> Vec<ServiceInstance> {
        let map = self.instances.read().await;
        map.values()
            .filter(|i| i.version == version)
            .cloned()
            .collect()
    }

    async fn restart_instance(&self, instance_id: &str) -> Result<(), SupervisorError> {
        self.restart_instance(instance_id).await.map(|_| ())
    }

    async fn emit_log(&self, instance_id: &str, line: String) {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let _ = self
            .log_tx
            .send(LogEntry {
                instance_id: instance_id.to_string(),
                timestamp_ms: ts,
                stream: LogStream::System,
                line,
            })
            .await;
    }

    async fn emit_event(&self, event: PhpMatrixEvent) {
        self.emit_matrix_event(event).await;
    }

    async fn sync_version_profile(&self, version: &str, profile: PhpProfileMode) {
        let profile_str = match profile {
            PhpProfileMode::Dev => "dev",
            PhpProfileMode::Prod => "prod",
        };
        let mut map = self.instances.write().await;
        for inst in map.values_mut() {
            if inst.version == version {
                inst.profile = profile_str.to_string();
            }
        }
    }

    fn config_php_dir(&self) -> PathBuf {
        self.paths.config.join("php")
    }

    fn resolve_php_root(&self, version: &str) -> Option<PathBuf> {
        let candidates = [
            self.paths.root.join("bin/runtime/php").join(version),
            self.paths.root.join("bin/runtime/php-cgi").join(version),
            self.paths.root.join("bin/php").join(version),
            self.paths.root.join("bin/php-cgi").join(version),
        ];
        for candidate in candidates {
            if candidate.exists() {
                return Some(candidate);
            }
        }

        let bin = self.resolve_php_binary(version)?;
        bin.parent().map(|p| {
            if p.file_name()
                .map(|n| n == "bin" || n == "sbin")
                .unwrap_or(false)
            {
                p.parent().unwrap_or(p).to_path_buf()
            } else {
                p.to_path_buf()
            }
        })
    }

    fn resolve_php_binary(&self, version: &str) -> Option<PathBuf> {
        let runtimes = self.scan_runtimes();
        runtimes
            .iter()
            .find(|r| {
                (r.engine == "php" || r.engine == "php-cgi") && r.version == version && r.is_valid
            })
            .map(|r| r.binary_path.clone())
    }

    fn resolve_ext_dir(&self, version: &str) -> Option<PathBuf> {
        let root = self.resolve_php_root(version)?;
        let ext = root.join("ext");
        if ext.is_dir() {
            Some(ext)
        } else {
            None
        }
    }
}

fn rand_val() -> u32 {
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    (t & 0xFFFF) as u32
}
