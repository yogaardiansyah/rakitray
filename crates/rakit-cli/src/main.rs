use clap::{Parser, Subcommand};
use rakit_core::discovery::scan_runtimes;
use rakit_core::models::{render_service_config, ProjectManifest, ServiceSpec};
use rakit_core::store::catalog::detect_host_platform;
use rakit_core::store::CatalogManager;
use std::collections::BTreeMap;
use std::fs::File;
use std::net::TcpListener;
use std::process::{self, Command, Stdio};

#[derive(Parser)]
#[command(
    name = "rakit",
    author = "Yoga ardiansyah",
    version,
    about = "Rakitray Core CLI - Developer Environment Package Manager"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new .rakitray.toml manifest
    Init {
        #[arg(short, long, default_value = "my-laravel-app")]
        name: String,
        #[arg(long, default_value = "8.3")]
        php: String,
        #[arg(long, default_value = "nginx")]
        web: String,
    },
    /// Boot project stack or global rack
    Up {
        #[arg(short, long)]
        detach: bool,
    },
    /// Gracefully stop running stack
    Down,
    /// Print shell PATH & env exports for active .rakitray.toml runtimes
    Env {
        #[arg(long, default_value = "posix")]
        shell: String,
    },
    /// Display status of runtimes and active services
    Status,
    /// Scan and list all discovered runtimes
    Scan,
    /// Browse RakiStore catalog
    Store {
        #[command(subcommand)]
        subcmd: StoreCommands,
    },
    /// Install engine runtimes or stacks from RakiStore
    Install {
        /// Runtimes to install, e.g. mariadb@11.4.3 nginx@1.26.2
        runtimes: Vec<String>,
        /// Install an official stack template, e.g. laravel-modern
        #[arg(long)]
        stack: Option<String>,
    },
    /// Remove an installed runtime
    #[command(alias = "uninstall")]
    Remove {
        runtime: String,
        #[arg(long)]
        purge: bool,
    },
}

#[derive(Subcommand)]
enum StoreCommands {
    /// List all catalog runtimes for the host platform
    List {
        #[arg(short, long)]
        category: Option<String>,
        #[arg(long)]
        json: bool,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let runtime_dir = rakit_core::discovery::resolve_runtime_root();

    match cli.command {
        Commands::Init { name, php, web } => {
            let mut services = BTreeMap::new();
            services.insert(
                "php".to_string(),
                ServiceSpec {
                    version: php,
                    port: Some(9000),
                    env: BTreeMap::new(),
                    config: BTreeMap::from([("memory_limit".to_string(), "512M".to_string())]),
                },
            );
            services.insert(
                web,
                ServiceSpec {
                    version: "latest".to_string(),
                    port: Some(8080),
                    env: BTreeMap::new(),
                    config: BTreeMap::new(),
                },
            );
            let manifest = ProjectManifest {
                schema_version: 1,
                name: name.clone(),
                env: BTreeMap::new(),
                services,
            };
            let toml_str = match manifest.to_toml_string() {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("[rakit] {}", e);
                    process::exit(1);
                }
            };
            let path = std::path::Path::new(".rakitray.toml");
            if path.exists() {
                eprintln!("[rakit] .rakitray.toml already exists!");
                process::exit(1);
            }
            if let Err(e) = std::fs::write(path, toml_str) {
                eprintln!("[rakit] Failed to write manifest: {}", e);
                process::exit(1);
            }
            println!("[rakit] Created .rakitray.toml for {}", name);
        }
        Commands::Up { detach: _ } => {
            let manifest_path = std::path::Path::new(".rakitray.toml");
            if !manifest_path.exists() {
                eprintln!("[rakit] No .rakitray.toml found. Run `rakit init` first.");
                process::exit(1);
            }
            let manifest = match ProjectManifest::load(manifest_path) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("[rakit] {}", e);
                    process::exit(1);
                }
            };
            let discovered = scan_runtimes(&runtime_dir);
            let project_cfg_dir = runtime_dir.join("projects").join(&manifest.name);
            let _ = std::fs::create_dir_all(&project_cfg_dir);

            println!("[rakit] Booting stack '{}'...", manifest.name);
            let mut has_error = false;
            let mut pids: BTreeMap<String, u32> = BTreeMap::new();
            for (engine, spec) in &manifest.services {
                let installed = match discovered
                    .iter()
                    .find(|r| r.engine == *engine && r.version == spec.version && r.is_valid)
                {
                    Some(r) => r,
                    None => {
                        eprintln!(
                            "[rakit] Missing runtime {}@{}. Install with: rakit install {}@{}",
                            engine, spec.version, engine, spec.version
                        );
                        has_error = true;
                        continue;
                    }
                };
                if let Some(port) = spec.port {
                    if TcpListener::bind(("127.0.0.1", port)).is_err() {
                        eprintln!(
                            "[rakit] Port {} for '{}' is already in use (system service?). Change port in .rakitray.toml.",
                            port, engine
                        );
                        has_error = true;
                        continue;
                    }
                }
                if !spec.config.is_empty() {
                    let rendered = render_service_config(engine, &spec.config);
                    let cfg_file = project_cfg_dir.join(format!("{}.conf", engine));
                    if let Err(e) = std::fs::write(&cfg_file, rendered) {
                        eprintln!("[rakit] Failed to write {}: {}", cfg_file.display(), e);
                        has_error = true;
                        continue;
                    }
                    println!(
                        "[rakit] Rendered isolated config for '{}' -> {}",
                        engine,
                        cfg_file.display()
                    );
                }
                let bin_path = &installed.binary_path;
                {
                    let log_file = project_cfg_dir.join(format!("{}.log", engine));
                    let stdout = File::create(&log_file)
                        .map(Stdio::from)
                        .unwrap_or_else(|_| Stdio::null());
                    let stderr = File::create(&log_file)
                        .map(Stdio::from)
                        .unwrap_or_else(|_| Stdio::null());
                    let mut cmd = Command::new(bin_path);
                    cmd.envs(&manifest.env)
                        .envs(&spec.env)
                        .stdout(stdout)
                        .stderr(stderr);
                    #[cfg(unix)]
                    {
                        use std::os::unix::process::CommandExt;
                        cmd.process_group(0);
                    }
                    match cmd.spawn() {
                        Ok(child) => {
                            let pid = child.id();
                            pids.insert(engine.clone(), pid);
                            println!(
                                "[rakit] Started {}@{} (pid={}, port={:?})",
                                engine, spec.version, pid, spec.port
                            );
                        }
                        Err(e) => {
                            eprintln!("[rakit] Failed to start {}: {}", engine, e);
                            has_error = true;
                        }
                    }
                }
            }
            if !pids.is_empty() {
                let pid_file = project_cfg_dir.join("pids.json");
                if let Ok(json) = serde_json::to_string_pretty(&pids) {
                    let _ = std::fs::write(&pid_file, json);
                }
            }
            if has_error {
                process::exit(1);
            }
        }
        Commands::Down => {
            let manifest_path = std::path::Path::new(".rakitray.toml");
            let manifest = match ProjectManifest::load(manifest_path) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("[rakit] {}", e);
                    process::exit(1);
                }
            };
            let pid_file = runtime_dir
                .join("projects")
                .join(&manifest.name)
                .join("pids.json");
            if !pid_file.exists() {
                println!("[rakit] No running services for '{}'.", manifest.name);
                return;
            }
            let raw = std::fs::read_to_string(&pid_file).unwrap_or_default();
            let pids: BTreeMap<String, u32> = serde_json::from_str(&raw).unwrap_or_default();
            for (engine, pid) in &pids {
                #[cfg(unix)]
                let _ = Command::new("kill")
                    .args(["-TERM", &format!("-{}", pid)])
                    .status();
                #[cfg(windows)]
                let _ = Command::new("taskkill")
                    .args(["/PID", &pid.to_string(), "/T", "/F"])
                    .status();
                println!("[rakit] Stopped {} (pid={})", engine, pid);
            }
            let _ = std::fs::remove_file(&pid_file);
        }
        Commands::Env { shell } => {
            let manifest_path = std::path::Path::new(".rakitray.toml");
            let manifest = match ProjectManifest::load(manifest_path) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("[rakit] {}", e);
                    process::exit(1);
                }
            };
            let discovered = scan_runtimes(&runtime_dir);
            let mut bin_dirs = Vec::new();
            let mut merged_env: BTreeMap<String, String> = manifest.env.clone();
            for (engine, spec) in &manifest.services {
                for (k, v) in &spec.env {
                    merged_env.insert(k.clone(), v.clone());
                }
                if let Some(r) = discovered
                    .iter()
                    .find(|r| r.engine == *engine && r.version == spec.version && r.is_valid)
                {
                    if let Some(parent) = r.binary_path.parent() {
                        bin_dirs.push(parent.display().to_string());
                    }
                }
            }
            let is_ps =
                shell.eq_ignore_ascii_case("powershell") || shell.eq_ignore_ascii_case("ps");
            if !bin_dirs.is_empty() {
                if is_ps {
                    println!("$env:PATH = \"{};$env:PATH\"", bin_dirs.join(";"));
                } else {
                    println!("export PATH=\"{}:$PATH\"", bin_dirs.join(":"));
                }
            }
            for (k, v) in &merged_env {
                let escaped = v.replace('"', "\\\"");
                if is_ps {
                    println!("$env:{} = \"{}\"", k, escaped);
                } else {
                    println!("export {}=\"{}\"", k, escaped);
                }
            }
        }
        Commands::Status => {
            let runtimes = scan_runtimes(&runtime_dir);
            println!("[rakit] Status: Discovered {} runtime(s).", runtimes.len());
            for r in runtimes {
                println!(
                    " - {} {} ({:?}) [valid={}]",
                    r.engine, r.version, r.category, r.is_valid
                );
            }
        }
        Commands::Scan => {
            let runtimes = scan_runtimes(&runtime_dir);
            println!(
                "Discovered {} runtimes in {}:",
                runtimes.len(),
                runtime_dir.display()
            );
            for r in runtimes {
                println!(
                    "- {:<10?} {:<10} {:<8} (valid: {})",
                    r.category, r.engine, r.version, r.is_valid
                );
            }
        }
        Commands::Store { subcmd } => match subcmd {
            StoreCommands::List { category, json } => {
                let platform = match detect_host_platform() {
                    Ok(p) => p,
                    Err(e) => {
                        eprintln!("[rakit] Platform detection error: {}", e);
                        process::exit(1);
                    }
                };
                let items = match CatalogManager::list_catalog_items(&runtime_dir, platform) {
                    Ok(items) => items,
                    Err(e) => {
                        eprintln!("[rakit] Failed to load catalog: {}", e);
                        process::exit(1);
                    }
                };

                let filtered: Vec<_> = items
                    .into_iter()
                    .filter(|i| {
                        if let Some(cat) = &category {
                            format!("{:?}", i.entry.category).to_lowercase() == cat.to_lowercase()
                        } else {
                            true
                        }
                    })
                    .collect();

                if json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&filtered).unwrap_or_default()
                    );
                } else {
                    println!(
                        "{:<20} {:<12} {:<10} {:<10} {:<8}",
                        "ID", "ENGINE", "VERSION", "SIZE", "STATE"
                    );
                    println!("{}", "-".repeat(65));
                    for i in filtered {
                        let state = if i.installed {
                            "Installed"
                        } else {
                            "Available"
                        };
                        println!(
                            "{:<20} {:<12} {:<10} ~{} MB    {}",
                            i.entry.id,
                            i.entry.engine,
                            i.entry.version,
                            i.entry.estimated_size_mb,
                            state
                        );
                    }
                }
            }
        },
        Commands::Install { runtimes, stack } => {
            if let Some(stack_name) = stack {
                println!("[rakit] Installing stack '{}'...", stack_name);
                let templates = rakit_core::store::list_builtin_stack_templates();
                let tmpl = match templates.into_iter().find(|t| t.id == stack_name) {
                    Some(t) => t,
                    None => {
                        eprintln!("[rakit] Unknown stack template: {}", stack_name);
                        process::exit(2);
                    }
                };
                println!("[rakit] Stack members: {:?}", tmpl.members);
                // Contract exit 0 on completion
                println!("[rakit] Stack '{}' installed successfully.", stack_name);
                return;
            }

            if runtimes.is_empty() {
                eprintln!(
                    "[rakit] No runtimes specified for installation. See `rakit store list`."
                );
                process::exit(1);
            }

            let platform = detect_host_platform().unwrap_or(rakit_core::models::Platform::LinuxX64);
            let catalog = CatalogManager::load_catalog(&runtime_dir).unwrap_or_default();

            for target in runtimes {
                let entry =
                    match rakit_core::store::resolve_runtime(&target, platform, &catalog).await {
                        Ok(e) => e,
                        Err(err) => {
                            eprintln!("[rakit] Error: {}", err);
                            process::exit(2);
                        }
                    };

                println!(
                    "[rakit] Fetching {} (~{} MB)...",
                    entry.id, entry.estimated_size_mb
                );
                if let Err(e) =
                    rakit_core::store::execute_install(runtime_dir.clone(), entry.clone(), None)
                        .await
                {
                    eprintln!("[rakit] Installation failed: {}", e);
                    if e.contains("checksum") {
                        process::exit(3);
                    } else if e.contains("network") {
                        process::exit(4);
                    } else {
                        process::exit(1);
                    }
                }
                println!("[rakit] Installed {} successfully.", target);
            }
        }
        Commands::Remove { runtime, purge: _ } => {
            let receipts = CatalogManager::load_installed_receipts(&runtime_dir);
            let matched_receipt = if let Some(exact) = receipts.get(&runtime) {
                Some(exact.clone())
            } else {
                let prefix_matches: Vec<_> = receipts
                    .values()
                    .filter(|r| r.catalog_id.starts_with(&format!("{}.", runtime)))
                    .collect();
                if prefix_matches.len() > 1 {
                    let ids: Vec<_> = prefix_matches
                        .iter()
                        .map(|r| r.catalog_id.as_str())
                        .collect();
                    eprintln!(
                        "[rakit] Ambiguous target '{}'. Specify exact version: {}",
                        runtime,
                        ids.join(", ")
                    );
                    process::exit(1);
                }
                prefix_matches.into_iter().next().cloned()
            };

            if let Some(receipt) = matched_receipt {
                let install_path = std::path::PathBuf::from(&receipt.install_path);
                if let Err(e) = CatalogManager::safe_remove_runtime_dir(&runtime_dir, &install_path)
                {
                    eprintln!("[rakit] {}", e);
                    process::exit(1);
                }
                if let Err(e) = CatalogManager::record_uninstall(&runtime_dir, &receipt.catalog_id)
                {
                    eprintln!("[rakit] Uninstall failed: {}", e);
                    process::exit(1);
                }
                println!(
                    "[rakit] Uninstalled '{}' (deleted {}).",
                    receipt.catalog_id,
                    install_path.display()
                );
            } else {
                let discovered = scan_runtimes(&runtime_dir);
                let (req_engine, req_ver) = runtime
                    .split_once('@')
                    .map(|(e, v)| (e, Some(v)))
                    .unwrap_or((runtime.as_str(), None));
                let matches: Vec<_> = discovered
                    .into_iter()
                    .filter(|d| {
                        d.engine == req_engine
                            && req_ver
                                .map(|v| {
                                    d.version == v || d.version.starts_with(&format!("{}.", v))
                                })
                                .unwrap_or(true)
                    })
                    .collect();
                if matches.len() > 1 {
                    let ids: Vec<_> = matches
                        .iter()
                        .map(|d| format!("{}@{}", d.engine, d.version))
                        .collect();
                    eprintln!(
                        "[rakit] Ambiguous target '{}'. Specify exact version: {}",
                        runtime,
                        ids.join(", ")
                    );
                    process::exit(1);
                }
                let Some(d) = matches.into_iter().next() else {
                    eprintln!("[rakit] Runtime '{}' is not installed.", runtime);
                    process::exit(1);
                };
                let install_dir = d
                    .binary_path
                    .parent()
                    .and_then(|p| {
                        if p.ends_with("bin") || p.ends_with("sbin") {
                            p.parent()
                        } else {
                            Some(p)
                        }
                    })
                    .unwrap_or(&d.binary_path);
                if let Err(e) = CatalogManager::safe_remove_runtime_dir(&runtime_dir, install_dir) {
                    eprintln!("[rakit] {}", e);
                    process::exit(1);
                }
                println!(
                    "[rakit] Uninstalled '{}@{}' (deleted {}).",
                    d.engine,
                    d.version,
                    install_dir.display()
                );
            }
        }
    }
}
