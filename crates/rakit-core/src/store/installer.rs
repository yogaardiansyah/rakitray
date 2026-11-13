use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::models::{CatalogEntry, DiscoveredRuntime, StackTemplate, StoreEvent};
use crate::store::catalog::InstallReceipt;
use crate::store::{
    CatalogManager, ChunkDownloader, DownloadOptions, ExtractOptions, LeanExtractor,
};

pub fn list_builtin_stack_templates() -> Vec<StackTemplate> {
    vec![
        StackTemplate {
            id: "laravel-modern".to_string(),
            name: "Laravel Modern Stack".to_string(),
            description: "Nginx 1.26 + PHP 8.3 + MariaDB 11.4 + Redis 7.2".to_string(),
            members: vec![
                "nginx@1.26.2".to_string(),
                "php@8.3.12".to_string(),
                "mariadb@11.4.3".to_string(),
                "redis@7.2.5".to_string(),
            ],
        },
        StackTemplate {
            id: "legacy-php".to_string(),
            name: "Legacy PHP Stack".to_string(),
            description: "Nginx 1.26 + PHP 8.2 + MariaDB 11.4".to_string(),
            members: vec![
                "nginx@1.26.2".to_string(),
                "php@8.2.24".to_string(),
                "mariadb@11.4.3".to_string(),
            ],
        },
        StackTemplate {
            id: "data-only".to_string(),
            name: "Data & Cache Stack".to_string(),
            description: "MariaDB 11.4 + Redis 7.2".to_string(),
            members: vec!["mariadb@11.4.3".to_string(), "redis@7.2.5".to_string()],
        },
    ]
}

pub async fn execute_install(
    root: PathBuf,
    entry: CatalogEntry,
    on_event: Option<mpsc::UnboundedSender<StoreEvent>>,
) -> Result<DiscoveredRuntime, String> {
    let install_id = format!("inst_{}", entry.id.replace('@', "_"));
    let downloads_dir = root.join("store/downloads");
    let staging_dir = root.join("store/staging").join(&install_id);

    let ext = match entry.archive_format {
        crate::models::ArchiveFormat::TarGz => "tar.gz",
        crate::models::ArchiveFormat::TarXz => "tar.xz",
        crate::models::ArchiveFormat::Zip => "zip",
    };
    let dest_archive = downloads_dir.join(format!("{}.{}", install_id, ext));

    let downloader = ChunkDownloader::new();
    let cancel_token = Arc::new(AtomicBool::new(false));

    // Progress dispatch closure
    let on_event_down = on_event.clone();
    let archive_path = downloader
        .download(
            DownloadOptions {
                url: entry.download_url.clone(),
                destination_file: dest_archive,
                install_id: install_id.clone(),
                catalog_id: entry.id.clone(),
            },
            cancel_token,
            move |p| {
                if let Some(tx) = &on_event_down {
                    let _ = tx.send(StoreEvent::Progress(p));
                }
            },
        )
        .await
        .map_err(|e| e.to_string())?;

    // Verify SHA-256 when provided by upstream source
    if !entry.sha256.trim().is_empty() {
        crate::store::verify_and_clean_file_sha256(&archive_path, &entry.sha256)
            .map_err(|e| e.to_string())?;
    }

    // Extraction destination: runtime/bin/<category>/<engine>/<version>/
    let category_str = match entry.category {
        crate::models::ServiceCategory::WebServer => "web_server",
        crate::models::ServiceCategory::Runtime => "runtime",
        crate::models::ServiceCategory::Database => "database",
        crate::models::ServiceCategory::Cache => "cache",
        crate::models::ServiceCategory::Tool => "tool",
    };
    let dest_dir = root
        .join("bin")
        .join(category_str)
        .join(&entry.engine)
        .join(&entry.version);

    let on_event_ext = on_event.clone();
    let summary = LeanExtractor::extract(
        ExtractOptions {
            archive_path: &archive_path,
            format: entry.archive_format,
            staging_dir: &staging_dir,
            dest_dir: &dest_dir,
            strip_comp: entry.strip_components,
            strip_profile: &entry.strip_profile,
            install_id: &install_id,
            catalog_id: &entry.id,
        },
        move |p| {
            if let Some(tx) = &on_event_ext {
                let _ = tx.send(StoreEvent::Progress(p));
            }
        },
    )
    .map_err(|e| e.to_string())?;

    // Write rakit-manifest.toml
    CatalogManager::write_blueprint_to_bin(&dest_dir, &entry.blueprint_id, &entry.version)
        .map_err(|e| e.to_string())?;

    // Record install receipt
    let receipt = InstallReceipt {
        catalog_id: entry.id.clone(),
        engine: entry.engine.clone(),
        version: entry.version.clone(),
        sha256: entry.sha256.clone(),
        installed_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        stripped_bytes: summary.stripped_bytes,
        install_path: dest_dir.to_string_lossy().to_string(),
    };
    CatalogManager::record_install(&root, receipt).map_err(|e| e.to_string())?;

    let discovered = DiscoveredRuntime {
        category: entry.category,
        engine: entry.engine,
        version: entry.version,
        binary_path: dest_dir,
        is_valid: true,
    };

    if let Some(tx) = &on_event {
        let _ = tx.send(StoreEvent::InstalledRuntime(discovered.clone()));
    }
    Ok(discovered)
}
