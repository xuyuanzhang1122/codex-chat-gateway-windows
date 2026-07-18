mod gateway;
mod models;
mod paths;

use gateway::{
    check_gateway, open_logs_folder, project_root_string, read_version, restart_gateway,
    run_project_script, set_autostart, start_gateway, status, stop_gateway, ActionResult,
    GatewayStatus, ENDPOINT_V1,
};
use models::{
    add_profile, delete_profile, fetch_remote_models, read_store, set_default, update_profile,
    ModelInput, ModelStore,
};

#[tauri::command]
fn get_status() -> GatewayStatus {
    status()
}

#[tauri::command]
fn get_project_info() -> serde_json::Value {
    serde_json::json!({
        "root": project_root_string(),
        "version": read_version(),
        "endpoint": ENDPOINT_V1,
        "autostart": gateway::autostart_enabled(),
    })
}

#[tauri::command]
fn list_models() -> Result<ModelStore, String> {
    read_store()
}

#[tauri::command]
fn create_model(input: ModelInput) -> Result<ModelStore, String> {
    add_profile(input)
}

#[tauri::command]
fn edit_model(id: String, input: ModelInput) -> Result<ModelStore, String> {
    update_profile(&id, input)
}

#[tauri::command]
fn remove_model(id: String) -> Result<ModelStore, String> {
    delete_profile(&id)
}

#[tauri::command]
fn make_default(id: String) -> Result<ModelStore, String> {
    set_default(&id)
}

#[tauri::command]
fn fetch_models(base_url: String, api_key: String) -> Result<Vec<String>, String> {
    fetch_remote_models(&base_url, &api_key)
}

#[tauri::command]
fn gateway_start() -> ActionResult {
    start_gateway()
}

#[tauri::command]
fn gateway_stop() -> ActionResult {
    stop_gateway()
}

#[tauri::command]
fn gateway_restart() -> ActionResult {
    restart_gateway()
}

#[tauri::command]
fn gateway_check() -> ActionResult {
    check_gateway()
}

#[tauri::command]
fn get_logs_dir() -> Result<String, String> {
    open_logs_folder()
}

#[tauri::command]
fn toggle_autostart(enable: bool) -> Result<String, String> {
    set_autostart(enable)
}

#[tauri::command]
fn run_script(name: String) -> Result<ActionResult, String> {
    // only allow known scripts
    const ALLOWED: &[&str] = &[
        "configure-codex.ps1",
        "restore-codex.ps1",
        "configure-claude-desktop.ps1",
        "restore-claude-desktop.ps1",
        "enable-autostart.ps1",
        "disable-autostart.ps1",
        "check.ps1",
    ];
    if !ALLOWED.contains(&name.as_str()) {
        return Err(format!("不允许执行脚本: {name}"));
    }
    run_project_script(&name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            get_status,
            get_project_info,
            list_models,
            create_model,
            edit_model,
            remove_model,
            make_default,
            fetch_models,
            gateway_start,
            gateway_stop,
            gateway_restart,
            gateway_check,
            get_logs_dir,
            toggle_autostart,
            run_script,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
