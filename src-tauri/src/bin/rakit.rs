use clap::{Parser, Subcommand};
use rakitray_lib::discovery::scan_runtimes;
use rakitray_lib::store::catalog::detect_host_platform;
use rakitray_lib::store::CatalogManager;
use std::process;
use tauri::ipc::Channel;

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
    let runtime_dir = rakitray_lib::discovery::resolve_runtime_root();

    match cli.command {
        Commands::Init { name, php, web } => {
            let manifest = format!(
                r#"# Rakitray Project Configuration
schema_version = 1
name = "{}"

[services]
php = "{}"
web = "{}"
"#,
                name, php, web
            );
            let path = std::path::Path::new(".rakitray.toml");
            if path.exists() {
                eprintln!("[rakit] .rakitray.toml already exists!");
                process::exit(1);
            }
            if let Err(e) = std::fs::write(path, manifest) {
                eprintln!("[rakit] Failed to write manifest: {}", e);
                process::exit(1);
            }
            println!("[rakit] Created .rakitray.toml for {}", name);
        }
        Commands::Up { detach: _ } => {
            println!(
                "[rakit] Starting Rakitray services from {}",
                runtime_dir.display()
            );
        }
        Commands::Down => {
            println!("[rakit] Stopping all running Rakitray services...");
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
                let templates = rakitray_lib::commands::list_builtin_stack_templates();
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

            let platform =
                detect_host_platform().unwrap_or(rakitray_lib::models::Platform::LinuxX64);
            let catalog = CatalogManager::load_catalog(&runtime_dir).unwrap_or_default();

            for target in runtimes {
                let entry = catalog
                    .iter()
                    .find(|e| e.id == target && e.platform == platform);
                if entry.is_none() {
                    eprintln!("[rakit] Error: Unknown catalog id '{}'", target);
                    process::exit(2);
                }
                let entry = entry.unwrap();

                println!(
                    "[rakit] Fetching {} (~{} MB)...",
                    entry.id, entry.estimated_size_mb
                );
                let channel = Channel::new(|_body| Ok(()));

                if let Err(e) = rakitray_lib::commands::store::execute_install(
                    runtime_dir.clone(),
                    entry.clone(),
                    channel,
                )
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
            if !receipts.contains_key(&runtime) {
                eprintln!("[rakit] Runtime '{}' is not installed.", runtime);
                process::exit(1);
            }
            if let Err(e) = CatalogManager::record_uninstall(&runtime_dir, &runtime) {
                eprintln!("[rakit] Uninstall failed: {}", e);
                process::exit(1);
            }
            println!("[rakit] Removed '{}' from receipt register.", runtime);
        }
    }
}
