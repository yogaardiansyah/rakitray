use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tauri::ipc::Channel;
use tauri::State;

use crate::commands::SupervisorState;
use crate::models::{CatalogEntry, CatalogItem, DiscoveredRuntime, StackTemplate, StoreEvent};
use crate::store::catalog::InstallReceipt;
use crate::store::{
    detect_host_platform, CatalogManager, ChunkDownloader, DownloadOptions, ExtractOptions,
    LeanExtractor,
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

#[tauri::command]
pub(crate) async fn list_catalog(
    _refresh: bool,
    state: State<'_, SupervisorState>,
) -> Result<Vec<CatalogItem>, String> {
    let platform = detect_host_platform().map_err(|e| e.to_string())?;
    CatalogManager::list_catalog_items(&state.paths.root, platform).map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) async fn list_stack_templates() -> Result<Vec<StackTemplate>, String> {
    Ok(list_builtin_stack_templates())
}

#[tauri::command]
pub(crate) async fn install_runtime(
    catalog_id: String,
    on_event: Channel<StoreEvent>,
    state: State<'_, SupervisorState>,
) -> Result<DiscoveredRuntime, String> {
    let root = state.paths.root.clone();
    let platform = detect_host_platform().map_err(|e| e.to_string())?;
    let catalog = CatalogManager::load_catalog(&root).map_err(|e| e.to_string())?;

    let entry = catalog
        .into_iter()
        .find(|e| e.id == catalog_id && e.platform == platform)
        .ok_or_else(|| format!("Unknown catalog ID: {}", catalog_id))?;

    execute_install(root, entry, on_event).await
}

pub async fn execute_install(
    root: PathBuf,
    entry: CatalogEntry,
    on_event: Channel<StoreEvent>,
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
                let _ = on_event_down.send(StoreEvent::Progress(p));
            },
        )
        .await
        .map_err(|e| e.to_string())?;

    // Verify SHA-256
    crate::store::verify_and_clean_file_sha256(&archive_path, &entry.sha256)
        .map_err(|e| e.to_string())?;

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
            let _ = on_event_ext.send(StoreEvent::Progress(p));
        },
    )
    .map_err(|e| e.to_string())?;

    // Write rakit-manifest.toml
    CatalogManager::write_blueprint_to_bin(&dest_dir, &entry.blueprint_id)
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

    let _ = on_event.send(StoreEvent::InstalledRuntime(discovered.clone()));
    Ok(discovered)
}

#[tauri::command]
pub(crate) async fn cancel_install(
    _install_id: String,
    _state: State<'_, SupervisorState>,
) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub(crate) async fn uninstall_runtime(
    catalog_id: String,
    state: State<'_, SupervisorState>,
) -> Result<(), String> {
    let root = &state.paths.root;
    let receipts = CatalogManager::load_installed_receipts(root);

    if let Some(receipt) = receipts.get(&catalog_id) {
        let install_path = PathBuf::from(&receipt.install_path);
        if install_path.exists() {
            let _ = std::fs::remove_dir_all(&install_path);
        }
        CatalogManager::record_uninstall(root, &catalog_id).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub(crate) async fn install_stack(
    template_id: String,
    on_event: Channel<StoreEvent>,
    state: State<'_, SupervisorState>,
) -> Result<Vec<DiscoveredRuntime>, String> {
    let templates = list_builtin_stack_templates();
    let template = templates
        .into_iter()
        .find(|t| t.id == template_id)
        .ok_or_else(|| format!("Unknown stack template: {}", template_id))?;

    let root = state.paths.root.clone();
    let platform = detect_host_platform().map_err(|e| e.to_string())?;
    let catalog = CatalogManager::load_catalog(&root).map_err(|e| e.to_string())?;

    let mut installed_list = Vec::new();
    for member_id in template.members {
        if let Some(entry) = catalog
            .iter()
            .find(|e| e.id == member_id && e.platform == platform)
        {
            let res = execute_install(root.clone(), entry.clone(), on_event.clone()).await?;
            installed_list.push(res);
        }
    }

    Ok(installed_list)
}
