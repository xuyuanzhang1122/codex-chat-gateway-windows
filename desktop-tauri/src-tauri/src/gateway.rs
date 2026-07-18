use crate::models::{claude_litellm_model, default_profile, read_store};
use crate::paths::{
    config_yaml, logs_dir, project_root, python_runtime, run_gateway_py, state_path,
};
use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;
use sysinfo::{ProcessesToUpdate, System};

pub const GATEWAY_HOST: &str = "127.0.0.1";
pub const GATEWAY_PORT: u16 = 4000;
pub const ENDPOINT: &str = "http://127.0.0.1:4000";
pub const ENDPOINT_V1: &str = "http://127.0.0.1:4000/v1";

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayStatus {
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    pub ok: bool,
    pub message: String,
    pub logs: Vec<String>,
    pub status: GatewayStatus,
}

/// Lightweight status for UI polling — no full process table scan, no /models fan-out.
pub fn status() -> GatewayStatus {
    status_inner(false)
}

/// Full status after start/stop/check — may scan processes and list model routes.
pub fn status_full() -> GatewayStatus {
    status_inner(true)
}

fn status_inner(detailed: bool) -> GatewayStatus {
    let root = project_root();
    let store = read_store().ok();
    let default_name = store
        .as_ref()
        .and_then(|s| default_profile(s))
        .map(|p| p.name.clone());

    let healthy = health_alive_ms(if detailed { 1200 } else { 500 });
    let routes = if detailed && healthy {
        list_routes().unwrap_or_default()
    } else {
        Vec::new()
    };

    let file = read_state_file(&root);
    let (pid, model, started_at, uptime) = if let Some(ref st) = file {
        if pid_alive(st.pid) {
            (
                Some(st.pid),
                Some(st.model.clone()),
                Some(st.started_at.clone()),
                format_uptime(&st.started_at),
            )
        } else if healthy {
            // Process gone from state but port still answers
            (None, Some(st.model.clone()), None, None)
        } else {
            // stale state — only remove on detailed path to avoid disk churn while polling
            if detailed {
                let _ = fs::remove_file(state_path(&root));
            }
            (None, None, None, None)
        }
    } else if detailed {
        if let Some(pid) = find_gateway_pid(&root) {
            (Some(pid), None, None, None)
        } else {
            (None, None, None, None)
        }
    } else {
        (None, None, None, None)
    };

    // Polling path: liveliness only. Start/check path: verify codex-chat route.
    let is_our = if detailed {
        healthy && is_our_gateway(&routes)
    } else {
        healthy
    };

    let running = healthy || pid.is_some();
    let message = if !running {
        "网关未在运行".into()
    } else if healthy && is_our {
        if detailed && !routes.is_empty() {
            "运行中 · 本机网关身份校验通过".into()
        } else {
            "运行中".into()
        }
    } else if healthy && !is_our {
        "端口 4000 有服务响应，但缺少 codex-chat 路由（可能不是本网关）".into()
    } else {
        "检测到相关进程，但健康检查未通过".into()
    };

    GatewayStatus {
        running,
        healthy,
        is_our_gateway: is_our,
        endpoint: ENDPOINT_V1.into(),
        pid,
        model,
        started_at,
        uptime,
        default_model_name: default_name,
        message,
        routes,
    }
}

pub fn start_gateway() -> ActionResult {
    let mut logs = Vec::new();
    let root = project_root();
    logs.push(format!("项目目录: {}", root.display()));

    // Already healthy & ours → ensure state and return ok
    let current = status_full();
    if current.healthy && current.is_our_gateway {
        if let Err(e) = ensure_state_for_running(&root, &mut logs) {
            logs.push(format!("补写状态失败: {e}"));
        } else {
            logs.push("网关已在运行，已同步 state.json".into());
        }
        return ActionResult {
            ok: true,
            message: "Gateway is already running".into(),
            logs,
            status: status_full(),
        };
    }
    if current.healthy && !current.is_our_gateway {
        logs.push("端口 4000 已被其他服务占用，拒绝启动。".into());
        return ActionResult {
            ok: false,
            message: "Port 4000 is occupied by another service".into(),
            logs,
            status: current,
        };
    }

    let store = match read_store() {
        Ok(s) => s,
        Err(e) => {
            logs.push(e.clone());
            return ActionResult {
                ok: false,
                message: e,
                logs,
                status: status_full(),
            };
        }
    };
    let Some(profile) = default_profile(&store) else {
        let msg = "尚未配置默认模型，请先添加模型".to_string();
        logs.push(msg.clone());
        return ActionResult {
            ok: false,
            message: msg,
            logs,
            status: status_full(),
        };
    };

    let python = match python_runtime(&root) {
        Some(p) => p,
        None => {
            let msg = "缺少 Python 运行时。请使用便携版或执行开发安装。".to_string();
            logs.push(msg.clone());
            return ActionResult {
                ok: false,
                message: msg,
                logs,
                status: status_full(),
            };
        }
    };
    let runner = run_gateway_py(&root);
    let config = config_yaml(&root);
    if !runner.is_file() || !config.is_file() {
        let msg = "缺少 run_gateway.py 或 config.yaml".to_string();
        logs.push(msg.clone());
        return ActionResult {
            ok: false,
            message: msg,
            logs,
            status: status_full(),
        };
    }

    let log_dir = logs_dir(&root);
    let _ = fs::create_dir_all(&log_dir);
    let _ = fs::create_dir_all(root.join(".gateway"));
    let stdout_path = log_dir.join("gateway.stdout.log");
    let stderr_path = log_dir.join("gateway.stderr.log");

    let stdout_file = match fs::File::create(&stdout_path) {
        Ok(f) => f,
        Err(e) => {
            let msg = format!("无法创建日志: {e}");
            logs.push(msg.clone());
            return ActionResult {
                ok: false,
                message: msg,
                logs,
                status: status_full(),
            };
        }
    };
    let stderr_file = match fs::File::create(&stderr_path) {
        Ok(f) => f,
        Err(e) => {
            let msg = format!("无法创建日志: {e}");
            logs.push(msg.clone());
            return ActionResult {
                ok: false,
                message: msg,
                logs,
                status: status_full(),
            };
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
            let msg = format!("启动失败: {e}");
            logs.push(msg.clone());
            return ActionResult {
                ok: false,
                message: msg,
                logs,
                status: status_full(),
            };
        }
    };

    let pid = child.id();
    logs.push(format!("已启动进程 PID {pid}"));
    let started_at = Utc::now().to_rfc3339();
    let state = GatewayStateFile {
        pid,
        executable: python.to_string_lossy().to_string(),
        runner: runner.to_string_lossy().to_string(),
        endpoint: ENDPOINT.into(),
        model: profile.litellm_model.clone(),
        started_at: started_at.clone(),
    };
    if let Err(e) = write_state_file(&root, &state) {
        logs.push(format!("写入 state 失败: {e}"));
    }

    // Wait for readiness + identity
    let mut ready = false;
    for attempt in 0..40 {
        thread::sleep(Duration::from_millis(500));
        // If process already dead, break
        if !pid_alive(pid) {
            logs.push(format!("进程在就绪前退出（尝试 {attempt}）"));
            break;
        }
        if health_alive_ms(800) {
            let routes = list_routes().unwrap_or_default();
            if is_our_gateway(&routes) {
                ready = true;
                break;
            }
        }
    }

    if !ready {
        let _ = kill_pid(pid);
        let _ = fs::remove_file(state_path(&root));
        logs.push("网关未能在时限内就绪，已回滚。详见 logs/gateway.stderr.log".into());
        return ActionResult {
            ok: false,
            message: "Gateway failed to become ready".into(),
            logs,
            status: status_full(),
        };
    }

    logs.push(format!(
        "网关已启动: {ENDPOINT_V1} · 默认模型 {} ({})",
        profile.name, profile.litellm_model
    ));
    ActionResult {
        ok: true,
        message: "Gateway started".into(),
        logs,
        status: status_full(),
    }
}

pub fn stop_gateway() -> ActionResult {
    let mut logs = Vec::new();
    let root = project_root();
    let mut killed = Vec::new();

    // 1) Prefer recorded state
    if let Some(st) = read_state_file(&root) {
        if process_matches(&root, st.pid, &st.executable, &st.runner) {
            if kill_pid(st.pid) {
                logs.push(format!("已停止 state 记录的 PID {}", st.pid));
                killed.push(st.pid);
            } else {
                logs.push(format!("无法停止 PID {}", st.pid));
            }
        } else {
            logs.push("state 中的 PID 已失效或不是本网关进程".into());
        }
    } else {
        logs.push("无 state.json，尝试按进程特征查找…".into());
    }

    // 2) Fallback: scan for run_gateway.py belonging to this project
    for pid in find_all_gateway_pids(&root) {
        if killed.contains(&pid) {
            continue;
        }
        if kill_pid(pid) {
            logs.push(format!("已停止发现的网关进程 PID {pid}"));
            killed.push(pid);
        }
    }

    let _ = fs::remove_file(state_path(&root));

    // 3) If something still healthy on port and ours, report failure
    thread::sleep(Duration::from_millis(300));
    let st = status_full();
    if st.healthy && st.is_our_gateway {
        logs.push("停止后健康检查仍通过，可能有未识别的残留进程".into());
        return ActionResult {
            ok: false,
            message: "Gateway still responding after stop attempts".into(),
            logs,
            status: st,
        };
    }

    if killed.is_empty() && !st.running {
        logs.push("网关已停止（无活动进程）".into());
    }

    ActionResult {
        ok: true,
        message: "Gateway stopped".into(),
        logs,
        status: status_full(),
    }
}

pub fn restart_gateway() -> ActionResult {
    let mut logs = Vec::new();
    let stop = stop_gateway();
    logs.extend(stop.logs);
    // brief pause so port releases
    thread::sleep(Duration::from_millis(400));
    let start = start_gateway();
    logs.extend(start.logs);
    ActionResult {
        ok: start.ok,
        message: if start.ok {
            "Gateway restarted".into()
        } else {
            start.message
        },
        logs,
        status: start.status,
    }
}

pub fn check_gateway() -> ActionResult {
    let mut logs = Vec::new();
    let st = status_full();
    if !st.healthy {
        logs.push("健康检查失败：本地网关不可达".into());
        return ActionResult {
            ok: false,
            message: "not reachable".into(),
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
        logs.push(format!("缺少路由: {}", missing.join(", ")));
        return ActionResult {
            ok: false,
            message: "missing routes".into(),
            logs,
            status: st,
        };
    }
    if !st.is_our_gateway {
        logs.push("响应存在但身份校验未通过".into());
        return ActionResult {
            ok: false,
            message: "identity check failed".into(),
            logs,
            status: st,
        };
    }
    logs.push(format!("Gateway OK: {ENDPOINT}"));
    logs.push(format!("路由: {}", st.routes.join(", ")));
    ActionResult {
        ok: true,
        message: "ok".into(),
        logs,
        status: st,
    }
}

fn health_alive_ms(timeout_ms: u64) -> bool {
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
        .timeout(Duration::from_secs(5))
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

fn ensure_state_for_running(root: &Path, logs: &mut Vec<String>) -> Result<(), String> {
    if let Some(st) = read_state_file(root) {
        if process_matches(root, st.pid, &st.executable, &st.runner) {
            logs.push(format!("state 有效 PID {}", st.pid));
            return Ok(());
        }
    }
    let pid = find_gateway_pid(root).ok_or_else(|| "无法定位网关 PID".to_string())?;
    let python = python_runtime(root)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    let runner = run_gateway_py(root).to_string_lossy().to_string();
    let model = read_store()
        .ok()
        .and_then(|s| default_profile(&s).map(|p| p.litellm_model.clone()))
        .unwrap_or_default();
    let state = GatewayStateFile {
        pid,
        executable: python,
        runner,
        endpoint: ENDPOINT.into(),
        model,
        started_at: Utc::now().to_rfc3339(),
    };
    write_state_file(root, &state)?;
    logs.push(format!("已补写 state.json → PID {pid}"));
    Ok(())
}

fn process_matches(root: &Path, pid: u32, expected_exe: &str, expected_runner: &str) -> bool {
    let mut sys = System::new();
    let handle = sysinfo::Pid::from_u32(pid);
    sys.refresh_processes(ProcessesToUpdate::Some(&[handle]), true);
    let Some(proc) = sys.process(handle) else {
        return false;
    };
    let exe = proc
        .exe()
        .map(|p| normalize_path(p))
        .unwrap_or_default();
    let expected = normalize_path(Path::new(expected_exe));
    if !exe.is_empty() && !expected.is_empty() && exe != expected {
        // allow python.exe vs pythonw.exe mismatch for same runtime dir
        let exe_stem = Path::new(&exe)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let exp_stem = Path::new(&expected)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let both_python = exe_stem.starts_with("python") && exp_stem.starts_with("python");
        if !both_python {
            return false;
        }
        // same directory preferred
        let exe_dir = Path::new(&exe).parent().map(|p| normalize_path(p));
        let exp_dir = Path::new(&expected).parent().map(|p| normalize_path(p));
        if exe_dir != exp_dir {
            // still accept if cmd contains our runner
        }
    }
    let runner_name = Path::new(expected_runner)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("run_gateway.py");
    let cmd = proc
        .cmd()
        .iter()
        .map(|s| s.to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join(" ");
    let root_s = normalize_path(root);
    cmd.contains(runner_name) && (cmd.contains(&root_s) || cmd.contains("run_gateway.py"))
}

fn pid_alive(pid: u32) -> bool {
    let mut sys = System::new();
    let handle = sysinfo::Pid::from_u32(pid);
    sys.refresh_processes(ProcessesToUpdate::Some(&[handle]), true);
    sys.process(handle).is_some()
}

fn kill_pid(pid: u32) -> bool {
    let mut sys = System::new();
    let handle = sysinfo::Pid::from_u32(pid);
    sys.refresh_processes(ProcessesToUpdate::Some(&[handle]), true);
    if let Some(proc) = sys.process(handle) {
        return proc.kill();
    }
    false
}

fn find_gateway_pid(root: &Path) -> Option<u32> {
    find_all_gateway_pids(root).into_iter().next()
}

fn find_all_gateway_pids(root: &Path) -> Vec<u32> {
    let mut sys = System::new();
    sys.refresh_processes(ProcessesToUpdate::All, true);
    let runner = run_gateway_py(root);
    let runner_s = normalize_path(&runner);
    let runner_name = "run_gateway.py";
    let root_s = normalize_path(root);
    let mut pids = Vec::new();
    for (pid, proc) in sys.processes() {
        let cmd = proc
            .cmd()
            .iter()
            .map(|s| s.to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join(" ");
        if !cmd.contains(runner_name) {
            continue;
        }
        // Prefer exact project path match; also accept absolute runner path
        if cmd.contains(&root_s) || cmd.contains(&runner_s) || cmd.contains("run_gateway.py") {
            // Avoid matching unrelated projects: require root path if possible
            if cmd.contains(&root_s) || cmd.contains(&runner_s) {
                pids.push(pid.as_u32());
            }
        }
    }
    pids
}

fn normalize_path(path: &Path) -> String {
    path.canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .trim_start_matches(r"\\?\")
        .replace('/', "\\")
        .to_ascii_lowercase()
}

fn format_uptime(started_at: &str) -> Option<String> {
    let started = DateTime::parse_from_rfc3339(started_at)
        .ok()
        .map(|d| d.with_timezone(&Local))
        .or_else(|| {
            DateTime::parse_from_rfc3339(started_at)
                .ok()
                .map(|d| d.with_timezone(&Local))
        })?;
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
    Some(format!("{} 天 {} 小时", secs / 86400, (secs % 86400) / 3600))
}

pub fn open_logs_folder() -> Result<String, String> {
    let root = project_root();
    let dir = logs_dir(&root);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.to_string_lossy().to_string())
}

pub fn project_root_string() -> String {
    project_root().to_string_lossy().to_string()
}

pub fn read_version() -> String {
    let root = project_root();
    fs::read_to_string(root.join("VERSION"))
        .map(|s| format!("v{}", s.trim()))
        .unwrap_or_else(|_| "v1.3.0-tauri".into())
}

pub fn autostart_enabled() -> bool {
    let startup = dirs_startup();
    startup
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
        status: status_full(),
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
    let root = project_root();
    let mut cmd = Command::new("powershell.exe");
    cmd.args([
        "-NoLogo",
        "-NoProfile",
        "-ExecutionPolicy",
        "Bypass",
        "-File",
        &script.to_string_lossy(),
    ])
    .current_dir(&root)
    .stdout(Stdio::piped())
    .stderr(Stdio::piped());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let output = cmd.output().map_err(|e| format!("无法启动 PowerShell: {e}"))?;
    let code = output.status.code().unwrap_or(-1);
    let out = String::from_utf8_lossy(&output.stdout).to_string();
    let err = String::from_utf8_lossy(&output.stderr).to_string();
    Ok((code, out, err))
}

fn dirs_startup() -> Option<PathBuf> {
    // %APPDATA%\Microsoft\Windows\Start Menu\Programs\Startup
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
