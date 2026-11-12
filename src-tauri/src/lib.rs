pub mod commands;
pub mod discovery;
pub mod drivers;
pub mod models;
pub mod supervisor;

use commands::SupervisorState;
use discovery::resolve_runtime_root;
use std::sync::Arc;
use supervisor::Supervisor;

#[tauri::command]
fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
fn get_system_info() -> String {
    format!(
        "Rakitray Core Engine running on {} ({})",
        std::env::consts::OS,
        std::env::consts::ARCH
    )
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let runtime_dir = resolve_runtime_root();
    let supervisor = Arc::new(Supervisor::new(runtime_dir));
    let supervisor_setup = supervisor.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(supervisor as SupervisorState)
        .setup(move |_app| {
            supervisor_setup.start_background_tasks();
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_app_version,
            get_system_info,
            commands::subscribe_supervisor,
            commands::scan_installed_runtimes,
            commands::start_instance,
            commands::stop_instance,
            commands::save_project_site
        ])
        .run(tauri::generate_context!())
        .expect("error while running rakitray application");
}
