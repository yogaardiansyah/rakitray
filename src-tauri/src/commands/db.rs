use tauri::State;

use crate::commands::SupervisorState;
use crate::db::{generate_db_env_snippet, DbProvisioner};
use crate::models::{DbCreateRequest, DbOverview, DbUserRequest};

#[tauri::command]
pub(crate) async fn db_overview(
    instance_id: String,
    state: State<'_, SupervisorState>,
) -> Result<DbOverview, String> {
    let sock = state.paths.tmp.join(format!("{}.sock", instance_id));
    DbProvisioner::get_overview(&sock, None)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) async fn db_create_database(
    instance_id: String,
    req: DbCreateRequest,
    state: State<'_, SupervisorState>,
) -> Result<(), String> {
    let sock = state.paths.tmp.join(format!("{}.sock", instance_id));
    DbProvisioner::create_database(&sock, None, &req)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) async fn db_create_user(
    instance_id: String,
    req: DbUserRequest,
    state: State<'_, SupervisorState>,
) -> Result<(), String> {
    let sock = state.paths.tmp.join(format!("{}.sock", instance_id));
    DbProvisioner::create_user(&sock, None, &req)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) async fn db_set_root_password(
    instance_id: String,
    password: String,
    state: State<'_, SupervisorState>,
) -> Result<(), String> {
    let sock = state.paths.tmp.join(format!("{}.sock", instance_id));
    DbProvisioner::set_root_password(&sock, None, &password)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) async fn db_env_snippet(
    instance_id: String,
    database: String,
    state: State<'_, SupervisorState>,
) -> Result<String, String> {
    let map = state.instances.read().await;
    let inst = map.get(&instance_id);
    let port = inst.and_then(|i| i.port).unwrap_or(3306);
    let conn = if instance_id.starts_with("postgres") {
        "pgsql"
    } else {
        "mysql"
    };

    Ok(generate_db_env_snippet(
        conn,
        "127.0.0.1",
        port,
        &database,
        "root",
        "",
    ))
}

#[tauri::command]
pub(crate) async fn cache_flush(
    _instance_id: String,
    _state: State<'_, SupervisorState>,
) -> Result<(), String> {
    Ok(())
}
