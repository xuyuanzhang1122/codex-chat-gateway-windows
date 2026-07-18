mod gateway;
mod models;
mod paths;

use gateway::{
    open_logs_folder, project_root_string, read_version, run_project_script, set_autostart,
    ActionResult, GatewayManager, GatewayStatus, ENDPOINT_V1, GITHUB_REPO,
};
use models::{
    add_profile, delete_profile, fetch_remote_models, read_store, set_default, update_profile,
    ModelInput, ModelStore,
};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

struct AppState {
    gateway: Arc<GatewayManager>,
}

#[tauri::command]
fn get_status(state: State<'_, AppState>) -> GatewayStatus {
    state.gateway.refresh_light()
}

#[tauri::command]
fn get_project_info() -> serde_json::Value {
    serde_json::json!({
        "root": project_root_string(),
        "version": read_version(),
        "endpoint": ENDPOINT_V1,
        "autostart": gateway::autostart_enabled(),
        "github": GITHUB_REPO,
        "credits": {
            "project": "Codex Chat Gateway",
            "repository": GITHUB_REPO,
            "owner": "xuyuanzhang1122",
            "ui_kit": "https://ui.lobehub.com",
            "ui_kit_name": "LobeHub UI",
        }
    })
}

#[tauri::command]
fn list_models(state: State<'_, AppState>) -> Result<ModelStore, String> {
    let store = read_store()?;
    state.gateway.invalidate_models(&store);
    Ok(store)
}

#[tauri::command]
fn create_model(state: State<'_, AppState>, input: ModelInput) -> Result<ModelStore, String> {
    let store = add_profile(input)?;
    state.gateway.invalidate_models(&store);
    Ok(store)
}

#[tauri::command]
fn edit_model(
    state: State<'_, AppState>,
    id: String,
    input: ModelInput,
) -> Result<ModelStore, String> {
    let store = update_profile(&id, input)?;
    state.gateway.invalidate_models(&store);
    Ok(store)
}

#[tauri::command]
fn remove_model(state: State<'_, AppState>, id: String) -> Result<ModelStore, String> {
    let store = delete_profile(&id)?;
    state.gateway.invalidate_models(&store);
    Ok(store)
}

#[tauri::command]
fn make_default(state: State<'_, AppState>, id: String) -> Result<ModelStore, String> {
    let store = set_default(&id)?;
    state.gateway.invalidate_models(&store);
    Ok(store)
}

#[tauri::command]
fn fetch_models(base_url: String, api_key: String) -> Result<Vec<String>, String> {
    fetch_remote_models(&base_url, &api_key)
}

/// Non-blocking: work runs on a native worker thread; UI listens to gateway://* events.
#[tauri::command]
fn gateway_start(app: AppHandle, state: State<'_, AppState>) {
    state.gateway.start_background(app);
}

#[tauri::command]
fn gateway_stop(app: AppHandle, state: State<'_, AppState>) {
    state.gateway.stop_background(app);
}

#[tauri::command]
fn gateway_restart(app: AppHandle, state: State<'_, AppState>) {
    state.gateway.restart_background(app);
}

#[tauri::command]
fn gateway_check(app: AppHandle, state: State<'_, AppState>) -> ActionResult {
    state.gateway.check_now(&app)
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
fn run_script(app: AppHandle, state: State<'_, AppState>, name: String) -> Result<ActionResult, String> {
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
    let mut result = run_project_script(&name)?;
    result.status = state.gateway.snapshot();
    // surface script logs through the same event bus
    for line in &result.logs {
        let _ = app.emit(
            "gateway://log",
            gateway::LogEvent {
                level: if result.ok { "DIM".into() } else { "ERR".into() },
                message: line.clone(),
            },
        );
    }
    let _ = app.emit(
        "gateway://log",
        gateway::LogEvent {
            level: if result.ok { "OK".into() } else { "ERR".into() },
            message: result.message.clone(),
        },
    );
    Ok(result)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let gateway = Arc::new(GatewayManager::new());

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            gateway: Arc::clone(&gateway),
        })
        .setup(move |app| {
            let handle = app.handle().clone();
            // Initial light probe + start watcher
            let _ = gateway.refresh_light();
            gateway.start_watcher(handle);
            Ok(())
        })
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
