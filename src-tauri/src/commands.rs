pub mod db;
pub mod php_matrix;
pub mod store;

pub use store::list_builtin_stack_templates;

use crate::models::{DiscoveredRuntime, IpcStreamEvent, ProjectSite, ServiceInstance};
use crate::supervisor::Supervisor;
use std::sync::Arc;
use tauri::ipc::Channel;
use tauri::State;

pub type SupervisorState = Arc<Supervisor>;

#[tauri::command]
pub(crate) async fn subscribe_supervisor(
    on_event: Channel<IpcStreamEvent>,
    state: State<'_, SupervisorState>,
) -> Result<(), String> {
    state.add_subscriber(on_event).await;
    Ok(())
}

#[tauri::command]
pub(crate) async fn scan_installed_runtimes(
    state: State<'_, SupervisorState>,
) -> Result<Vec<DiscoveredRuntime>, String> {
    Ok(state.scan_runtimes())
}

#[tauri::command]
pub(crate) async fn start_instance(
    instance_id: String,
    state: State<'_, SupervisorState>,
) -> Result<ServiceInstance, String> {
    state
        .start_instance(&instance_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) async fn stop_instance(
    instance_id: String,
    state: State<'_, SupervisorState>,
) -> Result<ServiceInstance, String> {
    state
        .stop_instance(&instance_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) async fn save_project_site(
    site: ProjectSite,
    state: State<'_, SupervisorState>,
) -> Result<(), String> {
    state.save_project_site(site).await;
    Ok(())
}

#[tauri::command]
pub(crate) async fn update_instance_config(
    instance_id: String,
    patch: crate::models::ConfigPatch,
    state: State<'_, SupervisorState>,
) -> Result<ServiceInstance, String> {
    state
        .update_instance_config(&instance_id, patch)
        .await
        .map_err(|e| e.to_string())
}
