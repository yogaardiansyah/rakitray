pub mod commands;
pub use rakit_core::{blueprint, db, discovery, drivers, models, store, supervisor};

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

    #[cfg_attr(not(all(debug_assertions, feature = "mcp")), allow(unused_mut))]
    let mut builder = tauri::Builder::default().plugin(tauri_plugin_opener::init());

    #[cfg(all(debug_assertions, feature = "mcp"))]
    {
        builder = builder.plugin(tauri_plugin_mcp::init_with_config(
            tauri_plugin_mcp::PluginConfig::new("rakitray".to_string()).start_socket_server(true),
        ));
    }

    builder
        .manage(supervisor as SupervisorState)
        .setup(move |_app| {
            tauri::async_runtime::block_on(async {
                supervisor_setup.start_background_tasks();
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_app_version,
            get_system_info,
            commands::subscribe_supervisor,
            commands::scan_installed_runtimes,
            commands::start_instance,
            commands::stop_instance,
            commands::save_project_site,
            commands::delete_project_site,
            commands::update_instance_config,
            commands::store::list_catalog,
            commands::store::install_runtime,
            commands::store::cancel_install,
            commands::store::uninstall_runtime,
            commands::store::list_stack_templates,
            commands::store::install_stack,
            commands::db::db_overview,
            commands::db::db_create_database,
            commands::db::db_create_user,
            commands::db::db_set_root_password,
            commands::db::db_env_snippet,
            commands::db::cache_flush,
            commands::php_matrix::subscribe_php_matrix,
            commands::php_matrix::get_php_matrix,
            commands::php_matrix::toggle_php_extension,
            commands::php_matrix::apply_php_profile,
            commands::php_matrix::save_php_ini_directives,
            commands::php_matrix::apply_laravel_preset,
            commands::php_matrix::clone_php_matrix,
        ])
        .run(tauri::generate_context!())
        .expect("error while running rakitray application");
}
