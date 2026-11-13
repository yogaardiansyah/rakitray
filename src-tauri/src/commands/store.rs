use std::path::PathBuf;
use tauri::ipc::Channel;
use tauri::State;

use crate::commands::{bridge_channel, SupervisorState};
use crate::models::{CatalogItem, DiscoveredRuntime, StackTemplate, StoreEvent};
pub use crate::store::list_builtin_stack_templates;
use crate::store::{detect_host_platform, execute_install, CatalogManager};

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

    let tx = bridge_channel(on_event);
    execute_install(root, entry, Some(tx)).await
}

#[tauri::command]
pub(crate) async fn cancel_install(
    install_id: String,
    _state: State<'_, SupervisorState>,
) -> Result<(), String> {
    let _ = install_id;
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

    let tx = bridge_channel(on_event);
    let mut installed_list = Vec::new();
    for member_id in template.members {
        if let Some(entry) = catalog
            .iter()
            .find(|e| e.id == member_id && e.platform == platform)
        {
            let res = execute_install(root.clone(), entry.clone(), Some(tx.clone())).await?;
            installed_list.push(res);
        }
    }

    Ok(installed_list)
}
