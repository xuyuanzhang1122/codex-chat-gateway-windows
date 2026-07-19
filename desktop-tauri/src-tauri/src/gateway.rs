//! Native gateway process manager.
//!
//! Protocol conversion still runs in the bundled LiteLLM process (`run_gateway.py`).
//! Lifecycle, health, state persistence and UI snapshots are owned by this module —
//! no PowerShell involved for start/stop/status.

use crate::models::{claude_litellm_model, default_profile, read_store, ModelStore};
use crate::paths::{
    config_yaml, logs_dir, normalize_path_text, normalize_text, project_root,
    project_root_display, python_runtime, run_gateway_py, state_path, strip_extended_prefix,
};
use chrono::{DateTime, Local, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use sysinfo::{ProcessesToUpdate, System};
use tauri::{AppHandle, Emitter};

pub const GATEWAY_HOST: &str = "127.0.0.1";
pub const GATEWAY_PORT: u16 = 4000;
pub const ENDPOINT: &str = "http://127.0.0.1:4000";
pub const ENDPOINT_V1: &str = "http://127.0.0.1:4000/v1";
pub const GITHUB_REPO: &str = "https://github.com/xuyuanzhang1122/codex-chat-gateway-windows";

const REQUIRED_ROUTES: &[&str] = &[
    "codex-chat",
    "claude-sonnet-5",
    "claude-opus-4-8",
    "claude-haiku-4-5",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayStateFile {
    pub pid: u32,
    pub executable: String,
    pub runner: String,
    pub endpoint: String,
    pub model: String,
    pub started_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GatewayPhase {
    Stopped,
    Starting,
    Running,
    Stopping,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayStatus {
    pub phase: GatewayPhase,
    pub running: bool,
    pub healthy: bool,
    pub is_our_gateway: bool,
    pub endpoint: String,
    pub pid: Option<u32>,
    pub model: Option<String>,
    pub started_at: Option<String>,
    pub uptime: Option<String>,
    pub default_model_name: Option<String>,
    pub message: String,
    pub routes: Vec<String>,
    pub busy: bool,
    pub startup_progress: Option<u8>,
    pub startup_stage: Option<String>,
}

impl Default for GatewayStatus {
    fn default() -> Self {
        Self {
            phase: GatewayPhase::Stopped,
            running: false,
            healthy: false,
            is_our_gateway: false,
            endpoint: ENDPOINT_V1.into(),
            pid: None,
            model: None,
            started_at: None,
            uptime: None,
            default_model_name: None,
            message: "未启动".into(),
            routes: Vec::new(),
            busy: false,
            startup_progress: None,
            startup_stage: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    pub ok: bool,
    pub message: String,
    pub logs: Vec<String>,
    pub status: GatewayStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEvent {
    pub level: String,
    pub message: String,
}

struct Inner {
    status: GatewayStatus,
    /// Cached default model display name (invalidated on model edits).
    default_name: Option<String>,
    /// Owned child when we started the process from this session (optional).
    child: Option<Child>,
    last_health_ok: bool,
    last_probe: Instant,
}

pub struct GatewayManager {
    inner: RwLock<Inner>,
    op_lock: parking_lot::Mutex<()>,
    watcher_started: AtomicBool,
}

impl GatewayManager {
    pub fn new() -> Self {
        let default_name = read_store()
            .ok()
            .and_then(|s| default_profile(&s).map(|p| p.name.clone()));
        let mut status = GatewayStatus::default();
        status.default_model_name = default_name.clone();
        Self {
            inner: RwLock::new(Inner {
                status,
                default_name,
                child: None,
                last_health_ok: false,
                last_probe: Instant::now() - Duration::from_secs(60),
            }),
            op_lock: parking_lot::Mutex::new(()),
            watcher_started: AtomicBool::new(false),
        }
    }

    pub fn snapshot(&self) -> GatewayStatus {
        let mut st = self.inner.read().status.clone();
        // Refresh uptime string cheaply without I/O
        if let Some(ref started) = st.started_at {
            st.uptime = format_uptime(started);
        }
        st
    }

    pub fn set_default_name(&self, name: Option<String>) {
        let mut g = self.inner.write();
        g.default_name = name.clone();
        g.status.default_model_name = name;
    }

    pub fn invalidate_models(&self, store: &ModelStore) {
        let name = default_profile(store).map(|p| p.name.clone());
        self.set_default_name(name);
    }

    /// Instant status for UI — uses cache; optionally does a cheap health probe if stale.
    pub fn refresh_light(&self) -> GatewayStatus {
        {
            let g = self.inner.read();
            if g.last_probe.elapsed() < Duration::from_millis(1500) {
                let mut st = g.status.clone();
                if let Some(ref started) = st.started_at {
                    st.uptime = format_uptime(started);
                }
                return st;
            }
        }

        let healthy = health_probe(350);
        let mut g = self.inner.write();
        g.last_probe = Instant::now();
        g.last_health_ok = healthy;

        if g.status.busy {
            // Don't fight an in-flight start/stop
            return g.status.clone();
        }

        if healthy {
            g.status.healthy = true;
            g.status.running = true;
            // Light probe cannot list routes; keep previous is_our_gateway unless unknown.
            if g.status.phase == GatewayPhase::Stopped || g.status.phase == GatewayPhase::Error {
                g.status.phase = GatewayPhase::Running;
                // Assume ours when we have a state/pid match; full check refines later.
                if g.status.pid.is_none() {
                    g.status.is_our_gateway = read_state_file(&project_root())
                        .map(|f| pid_alive(f.pid) && process_is_our_gateway(&project_root(), f.pid))
                        .unwrap_or(true);
                }
            }
            if g.status.phase == GatewayPhase::Starting {
                g.status.phase = GatewayPhase::Running;
                g.status.startup_progress = None;
                g.status.startup_stage = None;
            }
            if g.status.message.is_empty()
                || g.status.message == "网关未在运行"
                || g.status.message == "检测中…"
            {
                g.status.message = "运行中".into();
            }
            // Fill pid from state file once if missing
            if g.status.pid.is_none() {
                if let Some(file) = read_state_file(&project_root()) {
                    if pid_alive(file.pid) {
                        g.status.pid = Some(file.pid);
                        g.status.model = Some(file.model);
                        g.status.started_at = Some(file.started_at);
                        g.status.is_our_gateway =
                            process_is_our_gateway(&project_root(), file.pid);
                    }
                }
            } else if let Some(pid) = g.status.pid {
                if !pid_alive(pid) {
                    g.status.pid = None;
                }
            }
        } else {
            g.status.healthy = false;
            // If we think we're running but health failed, verify pid
            if let Some(pid) = g.status.pid {
                if !pid_alive(pid) {
                    g.status.pid = None;
                    g.status.started_at = None;
                    g.status.uptime = None;
                    g.status.phase = GatewayPhase::Stopped;
                    g.status.running = false;
                    g.status.is_our_gateway = false;
                    g.status.message = "网关未在运行".into();
                    g.status.routes.clear();
                    g.status.startup_progress = None;
                    g.status.startup_stage = None;
                } else {
                    g.status.running = true;
                    g.status.message = "进程在线，健康检查未通过".into();
                }
            } else {
                g.status.running = false;
                g.status.is_our_gateway = false;
                g.status.phase = GatewayPhase::Stopped;
                g.status.message = "网关未在运行".into();
                g.status.routes.clear();
                g.status.startup_progress = None;
                g.status.startup_stage = None;
            }
        }

        g.status.default_model_name = g.default_name.clone();
        if let Some(ref started) = g.status.started_at {
            g.status.uptime = format_uptime(started);
        }
        g.status.clone()
    }

    pub fn refresh_full(&self) -> GatewayStatus {
        let healthy = health_probe(1000);
        let routes = if healthy {
            list_routes().unwrap_or_default()
        } else {
            Vec::new()
        };
        let is_our = healthy && is_our_gateway(&routes);
        let root = project_root();
        let file = read_state_file(&root);

        let mut g = self.inner.write();
        g.last_probe = Instant::now();
        g.last_health_ok = healthy;
        g.status.healthy = healthy;
        g.status.is_our_gateway = is_our;
        g.status.routes = routes;
        g.status.default_model_name = g.default_name.clone();

        if let Some(ref st) = file {
            if pid_alive(st.pid) {
                g.status.pid = Some(st.pid);
                g.status.model = Some(st.model.clone());
                g.status.started_at = Some(st.started_at.clone());
                g.status.uptime = format_uptime(&st.started_at);
            } else if !healthy {
                let _ = fs::remove_file(state_path(&root));
                g.status.pid = None;
                g.status.model = None;
                g.status.started_at = None;
                g.status.uptime = None;
            }
        }

        g.status.running = healthy || g.status.pid.is_some();
        if !g.status.busy {
            g.status.phase = if g.status.running {
                if is_our || healthy {
                    GatewayPhase::Running
                } else {
                    GatewayPhase::Error
                }
            } else {
                GatewayPhase::Stopped
            };
            g.status.startup_progress = None;
            g.status.startup_stage = None;
        }

        g.status.message = if !g.status.running {
            "网关未在运行".into()
        } else if healthy && is_our {
            "运行中 · 路由校验通过".into()
        } else if healthy {
            "端口有响应，但缺少 codex-chat 路由".into()
        } else {
            "进程可能残留，健康检查失败".into()
        };

        g.status.clone()
    }

    pub fn start_background(self: &Arc<Self>, app: AppHandle) {
        let mgr = Arc::clone(self);
        thread::Builder::new()
            .name("gw-start".into())
            .spawn(move || {
                let _guard = mgr.op_lock.lock();
                mgr.run_start(&app);
            })
            .ok();
    }

    pub fn stop_background(self: &Arc<Self>, app: AppHandle) {
        let mgr = Arc::clone(self);
        thread::Builder::new()
            .name("gw-stop".into())
            .spawn(move || {
                let _guard = mgr.op_lock.lock();
                mgr.run_stop(&app);
            })
            .ok();
    }

    pub fn restart_background(self: &Arc<Self>, app: AppHandle) {
        let mgr = Arc::clone(self);
        thread::Builder::new()
            .name("gw-restart".into())
            .spawn(move || {
                let _guard = mgr.op_lock.lock();
                mgr.run_stop(&app);
                thread::sleep(Duration::from_millis(350));
                mgr.run_start(&app);
            })
            .ok();
    }

    pub fn check_now(&self, app: &AppHandle) -> ActionResult {
        emit_log(app, "INFO", "▶ 接口检查");
        let st = self.refresh_full();
        let mut logs: Vec<String> = Vec::new();
        if !st.healthy {
            let msg = "健康检查失败：本地网关不可达".to_string();
            emit_log(app, "ERR", &msg);
            logs.push(msg);
            emit_status(app, &st);
            return ActionResult {
                ok: false,
                message: "not reachable".to_string(),
                logs,
                status: st,
            };
        }
        let missing: Vec<&str> = REQUIRED_ROUTES
            .iter()
            .copied()
            .filter(|r| !st.routes.iter().any(|x| x == *r))
            .collect();
        if !missing.is_empty() {
            let msg = format!("缺少路由: {}", missing.join(", "));
            emit_log(app, "ERR", &msg);
            logs.push(msg);
            emit_status(app, &st);
            return ActionResult {
                ok: false,
                message: "missing routes".to_string(),
                logs,
                status: st,
            };
        }
        logs.push(format!("Gateway OK: {ENDPOINT}"));
        logs.push(format!("路由: {}", st.routes.join(", ")));
        for l in &logs {
            emit_log(app, "OK", l);
        }
        emit_status(app, &st);
        ActionResult {
            ok: true,
            message: "ok".to_string(),
            logs,
            status: st,
        }
    }

    fn run_start(&self, app: &AppHandle) {
        {
            let mut g = self.inner.write();
            g.status.busy = true;
            g.status.phase = GatewayPhase::Starting;
            g.status.message = "正在启动…".into();
            g.status.startup_progress = Some(4);
            g.status.startup_stage = Some("准备本地运行环境".into());
        }
        emit_status(app, &self.snapshot());
        emit_log(app, "INFO", "▶ 启动网关");

        let root = project_root();
        emit_log(app, "DIM", &format!("项目目录: {}", project_root_display()));

        // Already healthy?
        if health_probe(400) {
            let routes = list_routes().unwrap_or_default();
            if is_our_gateway(&routes) {
                let _ = ensure_state_for_running(&root);
                {
                    let mut g = self.inner.write();
                    g.status.busy = false;
                    g.status.phase = GatewayPhase::Running;
                    g.status.healthy = true;
                    g.status.running = true;
                    g.status.is_our_gateway = true;
                    g.status.routes = routes;
                    g.status.message = "网关已在运行".into();
                    g.status.startup_progress = None;
                    g.status.startup_stage = None;
                    if let Some(file) = read_state_file(&root) {
                        g.status.pid = Some(file.pid);
                        g.status.model = Some(file.model);
                        g.status.started_at = Some(file.started_at);
                    }
                }
                emit_log(app, "OK", "网关已在运行，已同步状态");
                emit_status(app, &self.snapshot());
                emit_action(app, true, "网关已在运行");
                return;
            }
            {
                let mut g = self.inner.write();
                g.status.busy = false;
                g.status.phase = GatewayPhase::Error;
                g.status.healthy = true;
                g.status.running = true;
                g.status.is_our_gateway = false;
                g.status.message = "端口 4000 被其他服务占用".into();
                g.status.startup_progress = None;
                g.status.startup_stage = None;
            }
            emit_log(app, "ERR", "端口 4000 被其他服务占用，拒绝启动");
            emit_status(app, &self.snapshot());
            emit_action(app, false, "端口 4000 被其他服务占用");
            return;
        }

        let store = match read_store() {
            Ok(s) => s,
            Err(e) => {
                self.fail_start(app, &e);
                return;
            }
        };
        let Some(profile) = default_profile(&store).cloned() else {
            self.fail_start(app, "尚未配置默认模型，请先添加模型");
            return;
        };
        let Some(python) = python_runtime(&root) else {
            self.fail_start(app, "缺少 Python 运行时（runtime/ 或 .venv）");
            return;
        };
        let runner = run_gateway_py(&root);
        let config = config_yaml(&root);
        if !runner.is_file() || !config.is_file() {
            self.fail_start(app, "缺少 run_gateway.py 或 config.yaml");
            return;
        }

        {
            let mut g = self.inner.write();
            g.status.startup_progress = Some(16);
            g.status.startup_stage = Some("检查模型配置与 Python 运行时".into());
            g.status.message = "正在检查启动环境…".into();
        }
        emit_status(app, &self.snapshot());

        let log_dir = logs_dir(&root);
        let _ = fs::create_dir_all(&log_dir);
        let _ = fs::create_dir_all(root.join(".gateway"));
        let stdout_path = log_dir.join("gateway.stdout.log");
        let stderr_path = log_dir.join("gateway.stderr.log");
        let stdout_file = match fs::File::create(&stdout_path) {
            Ok(f) => f,
            Err(e) => {
                self.fail_start(app, &format!("无法创建日志: {e}"));
                return;
            }
        };
        let stderr_file = match fs::File::create(&stderr_path) {
            Ok(f) => f,
            Err(e) => {
                self.fail_start(app, &format!("无法创建日志: {e}"));
                return;
            }
        };

        let mut cmd = Command::new(&python);
        cmd.arg(&runner)
            .arg("--config")
            .arg(&config)
            .arg("--host")
            .arg(GATEWAY_HOST)
            .arg("--port")
            .arg(GATEWAY_PORT.to_string())
            .current_dir(&root)
            .stdout(Stdio::from(stdout_file))
            .stderr(Stdio::from(stderr_file))
            .env("UPSTREAM_MODEL", &profile.litellm_model)
            .env(
                "CLAUDE_UPSTREAM_MODEL",
                claude_litellm_model(&profile.litellm_model),
            )
            .env("UPSTREAM_BASE_URL", &profile.base_url)
            .env("UPSTREAM_API_KEY", &profile.api_key)
            .env("GATEWAY_HOST", GATEWAY_HOST)
            .env("GATEWAY_PORT", GATEWAY_PORT.to_string());

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        let child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                self.fail_start(app, &format!("启动失败: {e}"));
                return;
            }
        };

        let pid = child.id();
        let started_at = Utc::now().to_rfc3339();
        emit_log(app, "DIM", &format!("已启动进程 PID {pid}"));

        let state = GatewayStateFile {
            pid,
            executable: strip_extended_prefix(python.clone())
                .to_string_lossy()
                .into_owned(),
            runner: strip_extended_prefix(runner.clone())
                .to_string_lossy()
                .into_owned(),
            endpoint: ENDPOINT.into(),
            model: profile.litellm_model.clone(),
            started_at: started_at.clone(),
        };
        if let Err(e) = write_state_file(&root, &state) {
            emit_log(app, "ERR", &format!("写入 state 失败: {e}"));
        }

        {
            let mut g = self.inner.write();
            g.child = Some(child);
            g.status.pid = Some(pid);
            g.status.model = Some(profile.litellm_model.clone());
            g.status.started_at = Some(started_at);
            g.status.default_model_name = Some(profile.name.clone());
            g.default_name = Some(profile.name.clone());
            g.status.startup_progress = Some(28);
            g.status.startup_stage = Some("加载 LiteLLM 与路由配置".into());
            g.status.message = "网关进程已创建，正在等待接口就绪…".into();
        }
        emit_status(app, &self.snapshot());

        // Wait for readiness without blocking UI (we're already on worker thread)
        let mut ready = false;
        for attempt in 0..50 {
            thread::sleep(Duration::from_millis(200));
            if !pid_alive(pid) {
                emit_log(app, "ERR", &format!("进程在就绪前退出（attempt {attempt}）"));
                break;
            }
            if attempt % 3 == 0 {
                let progress = 30 + ((attempt as u8).saturating_mul(62) / 50);
                {
                    let mut g = self.inner.write();
                    g.status.startup_progress = Some(progress.min(92));
                    g.status.startup_stage = Some("等待本地接口和路由就绪".into());
                    g.status.message = format!("首次加载可能较慢 · 启动检查 {}/50", attempt + 1);
                }
                emit_status(app, &self.snapshot());
            }
            if health_probe(300) {
                let routes = list_routes().unwrap_or_default();
                if is_our_gateway(&routes) {
                    ready = true;
                    let mut g = self.inner.write();
                    g.status.routes = routes;
                    break;
                }
            }
        }

        if !ready {
            // Only kill the process we just spawned (session child / known pid+runner).
            let _ = kill_verified(
                &root,
                pid,
                Some(python.to_string_lossy().as_ref()),
                Some(runner.to_string_lossy().as_ref()),
            );
            let _ = fs::remove_file(state_path(&root));
            {
                let mut g = self.inner.write();
                g.child = None;
                g.status.busy = false;
                g.status.phase = GatewayPhase::Error;
                g.status.running = false;
                g.status.healthy = false;
                g.status.pid = None;
                g.status.message = "启动失败，见 logs/gateway.stderr.log".into();
                g.status.startup_progress = None;
                g.status.startup_stage = None;
            }
            emit_log(app, "ERR", "网关未能就绪，已回滚");
            emit_status(app, &self.snapshot());
            emit_action(app, false, "启动失败，见日志");
            return;
        }

        {
            let mut g = self.inner.write();
            g.status.busy = false;
            g.status.phase = GatewayPhase::Running;
            g.status.running = true;
            g.status.healthy = true;
            g.status.is_our_gateway = true;
            g.status.startup_progress = None;
            g.status.startup_stage = None;
            g.status.message = format!(
                "运行中 · {} ({})",
                profile.name, profile.litellm_model
            );
        }
        emit_log(
            app,
            "OK",
            &format!(
                "网关已启动 {ENDPOINT_V1} · {} ({})",
                profile.name, profile.litellm_model
            ),
        );
        emit_status(app, &self.snapshot());
        emit_action(app, true, "网关已启动");
    }

    fn fail_start(&self, app: &AppHandle, msg: &str) {
        {
            let mut g = self.inner.write();
            g.status.busy = false;
            g.status.phase = GatewayPhase::Error;
            g.status.message = msg.into();
            g.status.startup_progress = None;
            g.status.startup_stage = None;
        }
        emit_log(app, "ERR", msg);
        emit_status(app, &self.snapshot());
        emit_action(app, false, msg);
    }

    fn run_stop(&self, app: &AppHandle) {
        {
            let mut g = self.inner.write();
            g.status.busy = true;
            g.status.phase = GatewayPhase::Stopping;
            g.status.message = "正在停止…".into();
            g.status.startup_progress = None;
            g.status.startup_stage = None;
        }
        emit_status(app, &self.snapshot());
        emit_log(app, "INFO", "▶ 停止网关");

        let root = project_root();
        let mut killed = Vec::new();

        // Drop owned child first (we spawned it this session).
        {
            let mut g = self.inner.write();
            if let Some(mut child) = g.child.take() {
                let pid = child.id();
                // Still verify identity before kill — PID reuse is rare mid-session but cheap to check.
                if process_is_our_gateway(&root, pid) {
                    if kill_pid_tree(pid) {
                        killed.push(pid);
                        emit_log(app, "DIM", &format!("已停止会话子进程 PID {pid}"));
                    }
                    let _ = child.wait();
                } else {
                    emit_log(
                        app,
                        "DIM",
                        &format!("会话 PID {pid} 已不是本网关进程，跳过 kill"),
                    );
                    let _ = child.wait();
                }
            }
        }

        // Multiple start paths (console / bat / autostart) can leave several run_gateway.py
        // instances; stop must sweep all of them, not only the state-file PID.
        for round in 0..4 {
            if let Some(st) = read_state_file(&root) {
                if !killed.contains(&st.pid) {
                    if kill_verified(&root, st.pid, Some(&st.executable), Some(&st.runner)) {
                        emit_log(app, "DIM", &format!("已停止 state PID {}", st.pid));
                        killed.push(st.pid);
                    } else if process_is_our_gateway(&root, st.pid) && kill_pid_tree(st.pid) {
                        emit_log(
                            app,
                            "DIM",
                            &format!("已停止 state PID {}（cmdline 身份）", st.pid),
                        );
                        killed.push(st.pid);
                    } else if pid_alive(st.pid) && round == 0 {
                        emit_log(
                            app,
                            "DIM",
                            &format!(
                                "state PID {} 未通过严格身份匹配，继续扫描发现路径",
                                st.pid
                            ),
                        );
                    }
                }
            }

            for pid in find_all_gateway_pids(&root) {
                if killed.contains(&pid) {
                    continue;
                }
                if kill_verified(&root, pid, None, None) {
                    emit_log(app, "DIM", &format!("已停止发现的网关进程 PID {pid}"));
                    killed.push(pid);
                }
            }


            // Fallback: whatever listens on our gateway port and runs run_gateway.py
            // IS a gateway instance (covers leftovers spawned by other install dirs).
            if let Some(pid) = find_port_listener_gateway_pid(GATEWAY_PORT) {
                if !killed.contains(&pid) && kill_pid_tree(pid) {
                    emit_log(
                        app,
                        "DIM",
                        &format!("已停止占用 {GATEWAY_PORT} 端口的网关进程 PID {pid}"),
                    );
                    killed.push(pid);
                }
            }
            let _ = fs::remove_file(state_path(&root));
            thread::sleep(Duration::from_millis(300 + round * 150));

            if !health_probe(350) {
                break;
            }
            if round < 3 {
                emit_log(
                    app,
                    "DIM",
                    &format!("端口仍有响应，第 {} 轮补杀…", round + 2),
                );
            }
        }

        let still = health_probe(500);
        let leftover = find_all_gateway_pids(&root);
        {
            let mut g = self.inner.write();
            g.status.busy = false;
            g.status.pid = None;
            g.status.started_at = None;
            g.status.uptime = None;
            g.status.routes.clear();
            if still {
                g.status.phase = GatewayPhase::Error;
                g.status.running = true;
                g.status.healthy = true;
                // Only mark as ours when we still see our runner; otherwise port is foreign.
                g.status.is_our_gateway = !leftover.is_empty()
                    || list_routes()
                        .map(|r| is_our_gateway(&r))
                        .unwrap_or(false);
                g.status.message = if leftover.is_empty() {
                    "停止后端口仍有响应（可能非本网关占用 4000）".into()
                } else {
                    format!(
                        "停止未完成，仍有 {} 个网关相关进程",
                        leftover.len()
                    )
                };
            } else {
                g.status.phase = GatewayPhase::Stopped;
                g.status.running = false;
                g.status.healthy = false;
                g.status.is_our_gateway = false;
                g.status.message = if killed.is_empty() {
                    "网关已停止".into()
                } else {
                    format!("网关已停止（结束 {} 个进程）", killed.len())
                };
            }
        }

        if still {
            emit_log(
                app,
                "ERR",
                &format!(
                    "停止后健康检查仍通过 · killed={killed:?} leftover={leftover:?}"
                ),
            );
            emit_status(app, &self.snapshot());
            emit_action(app, false, "停止失败：端口 4000 仍有响应");
        } else {
            emit_log(
                app,
                "OK",
                &if killed.is_empty() {
                    "网关已停止".into()
                } else {
                    format!("网关已停止（结束 {} 个进程）", killed.len())
                },
            );
            emit_status(app, &self.snapshot());
            emit_action(app, true, "网关已停止");
        }
    }

    /// Background watcher: cheap probes + push events only on change.
    pub fn start_watcher(self: &Arc<Self>, app: AppHandle) {
        if self
            .watcher_started
            .swap(true, Ordering::SeqCst)
        {
            return;
        }
        let mgr = Arc::clone(self);
        thread::Builder::new()
            .name("gw-watch".into())
            .spawn(move || {
                let mut last_sig = String::new();
                loop {
                    thread::sleep(Duration::from_secs(2));
                    let st = mgr.refresh_light();
                    let sig = format!(
                        "{:?}|{}|{}|{:?}|{}",
                        st.phase, st.healthy, st.running, st.pid, st.message
                    );
                    if sig != last_sig {
                        last_sig = sig;
                        emit_status(&app, &st);
                    }
                }
            })
            .ok();
    }
}

fn emit_status(app: &AppHandle, status: &GatewayStatus) {
    let _ = app.emit("gateway://status", status);
    crate::sync_tray(app, status);
}

fn emit_log(app: &AppHandle, level: &str, message: &str) {
    let _ = app.emit(
        "gateway://log",
        LogEvent {
            level: level.into(),
            message: message.into(),
        },
    );
}

fn emit_action(app: &AppHandle, ok: bool, message: &str) {
    let _ = app.emit(
        "gateway://action",
        serde_json::json!({ "ok": ok, "message": message }),
    );
}

fn health_probe(timeout_ms: u64) -> bool {
    // Plain HTTP to localhost — no TLS stack needed
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_millis(timeout_ms))
        .build();
    agent
        .get(&format!("{ENDPOINT}/health/liveliness"))
        .call()
        .is_ok()
}

fn list_routes() -> Result<Vec<String>, String> {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(3))
        .build();
    let resp = agent
        .get(&format!("{ENDPOINT_V1}/models"))
        .call()
        .map_err(|e| e.to_string())?;
    let value: serde_json::Value = resp.into_json().map_err(|e| e.to_string())?;
    let data = value
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| "no data".to_string())?;
    Ok(data
        .iter()
        .filter_map(|i| i.get("id").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .collect())
}

fn is_our_gateway(routes: &[String]) -> bool {
    routes.iter().any(|r| r == "codex-chat")
}

fn read_state_file(root: &Path) -> Option<GatewayStateFile> {
    let path = state_path(root);
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

fn write_state_file(root: &Path, state: &GatewayStateFile) -> Result<(), String> {
    let path = state_path(root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(state).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("json.tmp");
    {
        let mut f = fs::File::create(&tmp).map_err(|e| e.to_string())?;
        f.write_all(json.as_bytes()).map_err(|e| e.to_string())?;
    }
    fs::rename(tmp, path).map_err(|e| e.to_string())
}

fn ensure_state_for_running(root: &Path) -> Result<(), String> {
    if let Some(st) = read_state_file(root) {
        // Trust state only when pid still matches recorded identity.
        if process_matches(root, st.pid, &st.executable, &st.runner) {
            return Ok(());
        }
    }
    let pid = find_gateway_pid(root).ok_or_else(|| "无法定位网关 PID".to_string())?;
    if !process_is_our_gateway(root, pid) {
        return Err("发现的进程未能通过网关身份校验".into());
    }
    let python = python_runtime(root)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    let runner = run_gateway_py(root).to_string_lossy().to_string();
    let model = read_store()
        .ok()
        .and_then(|s| default_profile(&s).map(|p| p.litellm_model.clone()))
        .unwrap_or_default();
    write_state_file(
        root,
        &GatewayStateFile {
            pid,
            executable: python,
            runner,
            endpoint: ENDPOINT.into(),
            model,
            started_at: Utc::now().to_rfc3339(),
        },
    )
}

fn pid_alive(pid: u32) -> bool {
    let mut sys = System::new();
    let handle = sysinfo::Pid::from_u32(pid);
    sys.refresh_processes(ProcessesToUpdate::Some(&[handle]), true);
    sys.process(handle).is_some()
}

/// Kill only after identity checks. When exe/runner are provided, both path and cmdline must match.
/// When omitted (discovery), cmdline must prove this project's `run_gateway.py`.
fn kill_verified(
    root: &Path,
    pid: u32,
    expected_exe: Option<&str>,
    expected_runner: Option<&str>,
) -> bool {
    let ok = match (expected_exe, expected_runner) {
        (Some(exe), Some(runner)) => process_matches(root, pid, exe, runner),
        _ => process_is_our_gateway(root, pid),
    };
    if !ok {
        return false;
    }
    kill_pid_raw(pid)
}

fn kill_pid_raw(pid: u32) -> bool {
    kill_pid_tree(pid)
}

/// Kill the process and its children. On Windows, prefer `taskkill /T /F` so
/// orphaned python/litellm workers on port 4000 do not keep the health probe green.
fn kill_pid_tree(pid: u32) -> bool {
    #[cfg(windows)]
    {
        let mut cmd = Command::new("taskkill");
        cmd.args(["/PID", &pid.to_string(), "/T", "/F"]);
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        if let Ok(status) = cmd.status() {
            if status.success() || !pid_alive(pid) {
                return true;
            }
        }
    }

    let mut sys = System::new();
    let handle = sysinfo::Pid::from_u32(pid);
    sys.refresh_processes(ProcessesToUpdate::Some(&[handle]), true);
    if let Some(proc) = sys.process(handle) {
        if proc.kill() {
            return true;
        }
    }
    !pid_alive(pid)
}

/// Dual check: executable path (python/pythonw tolerant) + cmdline contains our runner under root.
fn process_matches(root: &Path, pid: u32, expected_exe: &str, expected_runner: &str) -> bool {
    let mut sys = System::new();
    let handle = sysinfo::Pid::from_u32(pid);
    sys.refresh_processes(ProcessesToUpdate::Some(&[handle]), true);
    let Some(proc) = sys.process(handle) else {
        return false;
    };

    let actual_exe = proc
        .exe()
        .map(|p| normalize_path_text(p))
        .unwrap_or_default();
    let expected = normalize_text(expected_exe);
    if !actual_exe.is_empty() && !expected.is_empty() {
        let actual_stem = Path::new(&actual_exe)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let expected_stem = Path::new(&expected)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let both_python = actual_stem.starts_with("python") && expected_stem.starts_with("python");
        if !both_python && actual_exe != expected {
            return false;
        }
        if both_python {
            let a_dir = Path::new(&actual_exe)
                .parent()
                .map(normalize_path_text)
                .unwrap_or_default();
            let e_dir = Path::new(&expected)
                .parent()
                .map(|p| normalize_text(&p.to_string_lossy()))
                .unwrap_or_default();
            // Prefer same runtime directory; if dirs differ still require cmdline ownership.
            if !a_dir.is_empty() && !e_dir.is_empty() && a_dir != e_dir {
                // fall through to cmdline check only
            } else if actual_exe != expected && !both_python {
                return false;
            }
        }
    }

    let runner_name = Path::new(expected_runner)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("run_gateway.py");
    let cmd = process_cmdline(proc);
    let cmd_n = normalize_text(&cmd);
    let root_n = normalize_path_text(root);
    let runner_n = normalize_text(expected_runner);

    cmd_n.contains(&runner_name.to_ascii_lowercase())
        && (cmd_n.contains(&root_n) || cmd_n.contains(&runner_n))
}

fn process_is_our_gateway(root: &Path, pid: u32) -> bool {
    let mut sys = System::new();
    let handle = sysinfo::Pid::from_u32(pid);
    sys.refresh_processes(ProcessesToUpdate::Some(&[handle]), true);
    let Some(proc) = sys.process(handle) else {
        return false;
    };
    let cmd_n = normalize_text(&process_cmdline(proc));
    if !cmd_n.contains("run_gateway.py") {
        return false;
    }
    let root_n = normalize_path_text(root);
    let runner_n = normalize_path_text(&run_gateway_py(root));
    cmd_n.contains(&root_n) || cmd_n.contains(&runner_n)
}

fn process_cmdline(proc: &sysinfo::Process) -> String {
    proc.cmd()
        .iter()
        .map(|s| s.to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

fn find_gateway_pid(root: &Path) -> Option<u32> {
    find_all_gateway_pids(root).into_iter().next()
}

/// PID of the process listening on 127.0.0.1:`port` whose cmdline contains
/// run_gateway.py, regardless of which install directory it came from.
#[cfg(windows)]
fn find_port_listener_gateway_pid(port: u16) -> Option<u32> {
    let mut cmd = Command::new("netstat");
    cmd.args(["-ano", "-p", "tcp"]);
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let out = cmd.output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let local = format!("127.0.0.1:{port}");
    for line in text.lines() {
        if !line.contains("LISTENING") {
            continue;
        }
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 5 || cols[1] != local {
            continue;
        }
        if let Ok(pid) = cols[4].parse::<u32>() {
            if pid_cmdline_is_gateway(pid) {
                return Some(pid);
            }
        }
    }
    None
}

#[cfg(not(windows))]
fn find_port_listener_gateway_pid(_port: u16) -> Option<u32> {
    None
}

fn pid_cmdline_is_gateway(pid: u32) -> bool {
    let mut sys = System::new();
    let handle = sysinfo::Pid::from_u32(pid);
    sys.refresh_processes(ProcessesToUpdate::Some(&[handle]), true);
    sys.process(handle)
        .map(|p| {
            p.cmd()
                .iter()
                .any(|a| a.to_string_lossy().to_lowercase().contains("run_gateway.py"))
        })
        .unwrap_or(false)
}

fn find_all_gateway_pids(root: &Path) -> Vec<u32> {
    let mut sys = System::new();
    sys.refresh_processes(ProcessesToUpdate::All, true);
    let runner_n = normalize_path_text(&run_gateway_py(root));
    let root_n = normalize_path_text(root);
    let mut pids = Vec::new();
    for (pid, proc) in sys.processes() {
        let cmd_n = normalize_text(&process_cmdline(proc));
        if !cmd_n.contains("run_gateway.py") {
            continue;
        }
        // Both sides normalized: no `\\?\`, lowercase, backslashes — so match works.
        if cmd_n.contains(&root_n) || cmd_n.contains(&runner_n) {
            pids.push(pid.as_u32());
        }
    }
    pids
}

fn format_uptime(started_at: &str) -> Option<String> {
    let started = DateTime::parse_from_rfc3339(started_at)
        .ok()
        .map(|d| d.with_timezone(&Local))?;
    let span = Local::now().signed_duration_since(started);
    let secs = span.num_seconds();
    if secs < 0 {
        return None;
    }
    if secs < 60 {
        return Some(format!("{secs} 秒"));
    }
    if secs < 3600 {
        return Some(format!("{} 分钟", secs / 60));
    }
    if secs < 86400 {
        return Some(format!("{} 小时 {} 分", secs / 3600, (secs % 3600) / 60));
    }
    Some(format!(
        "{} 天 {} 小时",
        secs / 86400,
        (secs % 86400) / 3600
    ))
}

pub fn open_logs_folder() -> Result<String, String> {
    let root = project_root();
    let dir = logs_dir(&root);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(strip_extended_prefix(dir).to_string_lossy().to_string())
}

pub fn project_root_string() -> String {
    project_root_display()
}

pub fn read_version() -> String {
    let root = project_root();
    fs::read_to_string(root.join("VERSION"))
        .map(|s| format!("v{}", s.trim()))
        .unwrap_or_else(|_| format!("v{}", env!("CARGO_PKG_VERSION")))
}

pub fn autostart_enabled() -> bool {
    dirs_startup()
        .map(|p| p.join("Codex Chat Gateway.lnk").is_file())
        .unwrap_or(false)
}

pub fn set_autostart(enable: bool) -> Result<String, String> {
    let root = project_root();
    let script = if enable {
        root.join("scripts").join("enable-autostart.ps1")
    } else {
        root.join("scripts").join("disable-autostart.ps1")
    };
    run_ps_script(&script)
}

pub fn run_project_script(name: &str) -> Result<ActionResult, String> {
    let root = project_root();
    let script = root.join("scripts").join(name);
    if !script.is_file() {
        return Err(format!("缺少脚本: {}", script.display()));
    }
    let (code, out, err) = run_ps_script_raw(&script)?;
    let mut logs = Vec::new();
    for line in out.lines().chain(err.lines()) {
        if !line.trim().is_empty() {
            logs.push(line.to_string());
        }
    }
    Ok(ActionResult {
        ok: code == 0,
        message: if code == 0 {
            format!("{name} 完成")
        } else {
            format!("{name} 失败（退出码 {code}）")
        },
        logs,
        status: GatewayStatus::default(),
    })
}

fn run_ps_script(script: &Path) -> Result<String, String> {
    let (code, out, err) = run_ps_script_raw(script)?;
    let mut msg = out;
    if !err.trim().is_empty() {
        if !msg.is_empty() {
            msg.push('\n');
        }
        msg.push_str(&err);
    }
    if code != 0 {
        return Err(if msg.is_empty() {
            format!("脚本失败，退出码 {code}")
        } else {
            msg
        });
    }
    Ok(msg)
}

fn run_ps_script_raw(script: &Path) -> Result<(i32, String, String), String> {
    use std::io::Read;
    use std::sync::mpsc;

    // Strip `\\?\` — PowerShell -File with extended paths leaves $PSScriptRoot empty.
    let root = strip_extended_prefix(project_root());
    let script = strip_extended_prefix(script.to_path_buf());
    let script_s = script.to_string_lossy().into_owned();
    let root_s = root.to_string_lossy().into_owned();

    let mut cmd = Command::new("powershell.exe");
    cmd.args([
        "-NoLogo",
        "-NoProfile",
        "-ExecutionPolicy",
        "Bypass",
        "-File",
        &script_s,
    ])
    .current_dir(&root)
    .env("CODEX_CHAT_GATEWAY_ROOT", &root_s)
    .stdout(Stdio::piped())
    .stderr(Stdio::piped());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("无法启动 PowerShell: {e}"))?;

    let mut stdout_pipe = child.stdout.take();
    let mut stderr_pipe = child.stderr.take();

    let (tx_out, rx_out) = mpsc::channel::<Vec<u8>>();
    let (tx_err, rx_err) = mpsc::channel::<Vec<u8>>();

    thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(mut p) = stdout_pipe.take() {
            let _ = p.read_to_end(&mut buf);
        }
        let _ = tx_out.send(buf);
    });
    thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(mut p) = stderr_pipe.take() {
            let _ = p.read_to_end(&mut buf);
        }
        let _ = tx_err.send(buf);
    });

    let timeout = Duration::from_secs(90);
    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if started.elapsed() > timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err("PowerShell 脚本超时（90s），已终止".into());
                }
                thread::sleep(Duration::from_millis(40));
            }
            Err(e) => return Err(format!("等待 PowerShell 失败: {e}")),
        }
    };

    let stdout = rx_out
        .recv_timeout(Duration::from_secs(5))
        .unwrap_or_default();
    let stderr = rx_err
        .recv_timeout(Duration::from_secs(5))
        .unwrap_or_default();

    Ok((
        status.code().unwrap_or(-1),
        String::from_utf8_lossy(&stdout).into_owned(),
        String::from_utf8_lossy(&stderr).into_owned(),
    ))
}

fn dirs_startup() -> Option<PathBuf> {
    let appdata = std::env::var_os("APPDATA")?;
    Some(
        PathBuf::from(appdata)
            .join("Microsoft")
            .join("Windows")
            .join("Start Menu")
            .join("Programs")
            .join("Startup"),
    )
}
