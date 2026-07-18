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
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, State, WindowEvent,
};

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

#[tauri::command]
fn show_main_window(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
    Ok(())
}

#[tauri::command]
fn hide_main_window(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
    Ok(())
}

/// Quit the desktop console only. Gateway process is left running unless the user
/// explicitly stops it first (or uses uninstall scripts).
#[tauri::command]
fn quit_console(app: AppHandle) {
    app.exit(0);
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
            let _ = gateway.refresh_light();
            gateway.start_watcher(handle.clone());

            // System tray: close/minimize-to-tray must not kill the gateway process.
            let show_i = MenuItem::with_id(app, "show", "显示控制台", true, None::<&str>)?;
            let hide_i = MenuItem::with_id(app, "hide", "隐藏到托盘", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "退出控制台（网关继续运行）", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &hide_i, &quit_i])?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().cloned().expect("missing window icon"))
                .menu(&menu)
                .tooltip("Codex Chat Gateway")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.unminimize();
                            let _ = w.set_focus();
                        }
                    }
                    "hide" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.hide();
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.unminimize();
                            let _ = w.set_focus();
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                // X button / Alt+F4 → hide to tray; do not stop gateway.
                api.prevent_close();
                let _ = window.hide();
            }
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
            show_main_window,
            hide_main_window,
            quit_console,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
