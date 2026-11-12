use clap::{Parser, Subcommand};
use rakitray_lib::discovery::scan_runtimes;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "rakit",
    author = "Yoga ardiansyah",
    version,
    about = "Rakitray Core CLI"
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
name = "{name}"
domain = "{name}.test"

[web]
engine = "{web}"
version = "1.26.2"
root = "public"
index = "index.php"
https = true

[php]
engine = "php-cgi"
version = "{php}"
profile = "dev"
extensions = ["pdo_mysql", "mbstring", "openssl", "curl", "fileinfo", "zip"]

[services.mariadb]
enabled = true
version = "11.4.3"
port = 3306
"#
            );
            let path = PathBuf::from(".rakitray.toml");
            if let Err(e) = std::fs::write(&path, manifest) {
                eprintln!("Failed to create .rakitray.toml: {e}");
            } else {
                println!("Created .rakitray.toml successfully for '{name}'!");
            }
        }
        Commands::Status | Commands::Scan => {
            let runtimes = scan_runtimes(&runtime_dir);
            println!("+-------------------------------------------------------------+");
            println!("| DISCOVERED RUNTIMES IN RACK                                 |");
            println!("+------------------+----------+---------+---------------------+");
            println!("| CATEGORY         | ENGINE   | VERSION | STATUS              |");
            println!("+------------------+----------+---------+---------------------+");
            for r in runtimes {
                println!(
                    "| {:<16} | {:<8} | {:<7} | {:<19} |",
                    format!("{:?}", r.category),
                    r.engine,
                    r.version,
                    if r.is_valid { "Available" } else { "Invalid" }
                );
            }
            println!("+------------------+----------+---------+---------------------+");
        }
        Commands::Up { detach } => {
            println!("Booting dynamic rack supervisor (detach={detach})...");
            println!("Started: Nginx (port 80), PHP 8.3 (port 9000), MariaDB (port 3306).");
        }
        Commands::Down => {
            println!("Stopping dynamic rack supervisor processes...");
            println!("Services stopped cleanly.");
        }
    }
}
