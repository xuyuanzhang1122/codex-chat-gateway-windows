mod config;
mod protocol;
mod server;

use serde_json::json;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("native gateway error: {error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), String> {
    let root = env::var_os("CCG_ROOT")
        .map(PathBuf::from)
        .unwrap_or(env::current_dir().map_err(|e| e.to_string())?);
    let models_path = env_path("CCG_MODELS_PATH", root.join(".gateway/models.json"));
    let traffic_path = env_path(
        "CCG_ROUTING_TRAFFIC_PATH",
        root.join(".gateway/routing-traffic.json"),
    );
    let state_path = env_path("CCG_STATE_PATH", root.join(".gateway/state.json"));
    let port = env::var("CCG_PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(4000);
    let state = server::GatewayState::load(models_path, traffic_path)?;
    write_state(&state_path, port)?;
    let result = server::run(state, port).await;
    let _ = fs::remove_file(state_path);
    result
}

fn env_path(name: &str, fallback: PathBuf) -> PathBuf {
    env::var_os(name).map(PathBuf::from).unwrap_or(fallback)
}

fn write_state(path: &Path, port: u16) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let executable = env::current_exe().map_err(|e| e.to_string())?;
    let value = json!({
        "pid": std::process::id(),
        "executable": executable,
        "runner": "native-rust",
        "endpoint": format!("http://127.0.0.1:{port}"),
        "model": "codex-chat",
        "started_at": chrono::Utc::now().to_rfc3339(),
    });
    let temporary = path.with_extension("json.tmp");
    fs::write(
        &temporary,
        serde_json::to_vec_pretty(&value).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    fs::rename(temporary, path).map_err(|e| e.to_string())
}
