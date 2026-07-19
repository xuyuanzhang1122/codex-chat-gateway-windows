mod gateway;
mod models;
mod paths;
mod updater;

use gateway::{
    open_logs_folder, project_root_string, read_version, run_project_script, set_autostart,
    ActionResult, GatewayManager, GatewayStatus, ENDPOINT_V1, GITHUB_REPO,
};
use models::{
    add_profile, delete_profile, fetch_remote_models, import_profiles, parse_api_text, read_store,
    set_default, set_model_routing, set_profile_routing, update_profile, ModelInput, ModelStore,
    ParsedApiText,
};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, State, WindowEvent,
};

struct AppState {
    gateway: Arc<GatewayManager>,
    tray: Mutex<Option<TrayIcon>>,
    /// Last known main-window visibility for tray menu labels.
    window_visible: Mutex<bool>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RoutingTrafficRoute {
    model_id: String,
    profile_id: String,
    profile_name: String,
    upstream_host: String,
    hit_count: u64,
    first_seen_at: String,
    last_seen_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RoutingTrafficStore {
    version: u32,
    #[serde(default)]
    routes: Vec<RoutingTrafficRoute>,
}

impl Default for RoutingTrafficStore {
    fn default() -> Self {
        Self {
            version: 1,
            routes: Vec::new(),
        }
    }
}

#[tauri::command]
fn get_routing_traffic() -> Result<RoutingTrafficStore, String> {
    let path = paths::project_root()
        .join(".gateway")
        .join("routing-traffic.json");
    if !path.is_file() {
        return Ok(RoutingTrafficStore::default());
    }
    let text = std::fs::read_to_string(&path).map_err(|e| format!("读取分流轨迹失败: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("解析分流轨迹失败: {e}"))
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
fn configure_model_routing(
    state: State<'_, AppState>,
    model_id: String,
    enabled: bool,
) -> Result<ModelStore, String> {
    let store = set_model_routing(&model_id, enabled)?;
    state.gateway.invalidate_models(&store);
    Ok(store)
}

#[tauri::command]
fn configure_profile_routing(
    state: State<'_, AppState>,
    id: String,
    enabled: bool,
) -> Result<ModelStore, String> {
    let store = set_profile_routing(&id, enabled)?;
    state.gateway.invalidate_models(&store);
    Ok(store)
}

#[tauri::command]
fn fetch_models(base_url: String, api_key: String) -> Result<Vec<String>, String> {
    fetch_remote_models(&base_url, &api_key)
}

#[tauri::command]
fn parse_model_text(text: String) -> Result<ParsedApiText, String> {
    parse_api_text(&text)
}

#[tauri::command]
fn parse_model_file(path: String) -> Result<ParsedApiText, String> {
    let text = std::fs::read_to_string(&path).map_err(|e| format!("读取文件失败: {e}"))?;
    parse_api_text(&text)
}

#[tauri::command]
fn import_model_profiles(
    state: State<'_, AppState>,
    base_url: String,
    api_key: String,
    model_ids: Vec<String>,
    name_hint: Option<String>,
) -> Result<ModelStore, String> {
    let store = import_profiles(&base_url, &api_key, &model_ids, name_hint.as_deref())?;
    state.gateway.invalidate_models(&store);
    Ok(store)
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
fn run_script(
    app: AppHandle,
    state: State<'_, AppState>,
    name: String,
) -> Result<ActionResult, String> {
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
                level: if result.ok {
                    "DIM".into()
                } else {
                    "ERR".into()
                },
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
fn show_main_window(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
        *state.window_visible.lock() = true;
        sync_tray(&app, &state.gateway.snapshot());
    }
    Ok(())
}

#[tauri::command]
fn hide_main_window(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
        *state.window_visible.lock() = false;
        sync_tray(&app, &state.gateway.snapshot());
    }
    Ok(())
}

/// Quit the desktop console only. Gateway process is left running unless the user
/// explicitly stops it first (or uses uninstall scripts).
#[tauri::command]
fn quit_console(app: AppHandle) {
    app.exit(0);
}

/// Rebuild tray tooltip + menu so labels track gateway phase and window visibility.
pub fn sync_tray(app: &AppHandle, status: &GatewayStatus) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let visible = *state.window_visible.lock();
    let (phase_label, tip) = tray_status_labels(status);

    let tray_guard = state.tray.lock();
    let Some(tray) = tray_guard.as_ref() else {
        return;
    };

    let _ = tray.set_tooltip(Some(&tip));

    // Rebuild menu so Chinese labels stay in sync with live state.
    if let Ok(menu) = build_tray_menu(app, &phase_label, visible) {
        let _ = tray.set_menu(Some(menu));
    }
}

fn tray_status_labels(status: &GatewayStatus) -> (String, String) {
    let phase = match status.phase {
        gateway::GatewayPhase::Running if status.healthy => {
            if status.is_our_gateway {
                "运行中"
            } else {
                "端口占用"
            }
        }
        gateway::GatewayPhase::Running => "进程在线",
        gateway::GatewayPhase::Starting => "启动中…",
        gateway::GatewayPhase::Stopping => "停止中…",
        gateway::GatewayPhase::Error => "异常",
        gateway::GatewayPhase::Stopped => "已停止",
    };
    let pid = status
        .pid
        .map(|p| format!(" · PID {p}"))
        .unwrap_or_default();
    let model = status
        .default_model_name
        .as_ref()
        .or(status.model.as_ref())
        .map(|m| format!(" · {m}"))
        .unwrap_or_default();
    let tip = format!("Codex Chat Gateway · {phase}{pid}{model}");
    let menu_status = format!("网关：{phase}");
    (menu_status, tip)
}

fn build_tray_menu(
    app: &AppHandle,
    phase_label: &str,
    window_visible: bool,
) -> tauri::Result<Menu<tauri::Wry>> {
    let status_i = MenuItem::with_id(app, "status", phase_label, false, None::<&str>)?;
    let show_label = if window_visible {
        "显示控制台（当前已打开）"
    } else {
        "显示控制台"
    };
    let hide_label = if window_visible {
        "隐藏到托盘"
    } else {
        "隐藏到托盘（已隐藏）"
    };
    let show_i = MenuItem::with_id(app, "show", show_label, true, None::<&str>)?;
    let hide_i = MenuItem::with_id(app, "hide", hide_label, window_visible, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit_i = MenuItem::with_id(
        app,
        "quit",
        "退出控制台（网关继续运行）",
        true,
        None::<&str>,
    )?;
    Menu::with_items(app, &[&status_i, &sep, &show_i, &hide_i, &sep, &quit_i])
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let gateway = Arc::new(GatewayManager::new());

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .manage(AppState {
            gateway: Arc::clone(&gateway),
            tray: Mutex::new(None),
            window_visible: Mutex::new(true),
        })
        .setup(move |app| {
            // HTTPS GitHub Release updater (signature verified with public key in tauri.conf.json).
            // Private signing key must never be committed — use TAURI_SIGNING_PRIVATE_KEY(_PATH).
            app.handle()
                .plugin(tauri_plugin_updater::Builder::new().build())?;

            let handle = app.handle().clone();
            let st0 = gateway.refresh_light();
            gateway.start_watcher(handle.clone());

            // System tray: close/minimize-to-tray must not kill the gateway process.
            let (phase_label, tip) = tray_status_labels(&st0);
            let menu = build_tray_menu(app.handle(), &phase_label, true)?;

            let tray = TrayIconBuilder::new()
                .icon(
                    app.default_window_icon()
                        .cloned()
                        .expect("missing window icon"),
                )
                .menu(&menu)
                .tooltip(&tip)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.unminimize();
                            let _ = w.set_focus();
                        }
                        if let Some(state) = app.try_state::<AppState>() {
                            *state.window_visible.lock() = true;
                            sync_tray(app, &state.gateway.snapshot());
                        }
                    }
                    "hide" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.hide();
                        }
                        if let Some(state) = app.try_state::<AppState>() {
                            *state.window_visible.lock() = false;
                            sync_tray(app, &state.gateway.snapshot());
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
                        if let Some(state) = app.try_state::<AppState>() {
                            *state.window_visible.lock() = true;
                            sync_tray(app, &state.gateway.snapshot());
                        }
                    }
                })
                .build(app)?;

            if let Some(state) = app.try_state::<AppState>() {
                *state.tray.lock() = Some(tray);
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                // X button / Alt+F4 → hide to tray; do not stop gateway.
                api.prevent_close();
                let _ = window.hide();
                let app = window.app_handle();
                if let Some(state) = app.try_state::<AppState>() {
                    *state.window_visible.lock() = false;
                    sync_tray(app, &state.gateway.snapshot());
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_status,
            get_project_info,
            get_routing_traffic,
            list_models,
            create_model,
            edit_model,
            remove_model,
            make_default,
            configure_model_routing,
            configure_profile_routing,
            fetch_models,
            parse_model_text,
            parse_model_file,
            import_model_profiles,
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
            updater::download_studio_installer,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
