use std::path::{Path, PathBuf};
use std::sync::OnceLock;

static ROOT: OnceLock<PathBuf> = OnceLock::new();

/// Resolve once and cache the gateway project root (config.yaml + scripts/).
pub fn project_root() -> PathBuf {
    ROOT.get_or_init(discover_root).clone()
}

fn discover_root() -> PathBuf {
    if let Ok(explicit) = std::env::var("CODEX_CHAT_GATEWAY_ROOT") {
        let p = PathBuf::from(explicit);
        if is_project_root(&p) {
            return canonicalize(&p);
        }
    }

    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.to_path_buf());
            candidates.push(dir.join(".."));
            candidates.push(dir.join("../.."));
            candidates.push(dir.join("../../.."));
        }
    }

    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.clone());
        candidates.push(cwd.join(".."));
        candidates.push(cwd.join("../.."));
    }

    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let m = PathBuf::from(manifest);
        candidates.push(m.join("../.."));
        candidates.push(m.join("../../.."));
    }

    for c in candidates {
        if is_project_root(&c) {
            return canonicalize(&c);
        }
        if let Ok(resolved) = c.canonicalize() {
            if is_project_root(&resolved) {
                return resolved;
            }
        }
    }

    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn canonicalize(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn is_project_root(path: &Path) -> bool {
    path.join("config.yaml").is_file() && path.join("scripts").is_dir()
}

pub fn models_path(root: &Path) -> PathBuf {
    root.join(".gateway").join("models.json")
}

pub fn state_path(root: &Path) -> PathBuf {
    root.join(".gateway").join("state.json")
}

pub fn logs_dir(root: &Path) -> PathBuf {
    root.join("logs")
}

pub fn config_yaml(root: &Path) -> PathBuf {
    root.join("config.yaml")
}

pub fn run_gateway_py(root: &Path) -> PathBuf {
    root.join("run_gateway.py")
}

pub fn python_runtime(root: &Path) -> Option<PathBuf> {
    for rel in [
        "runtime/pythonw.exe",
        ".venv/Scripts/pythonw.exe",
        "runtime/python.exe",
        ".venv/Scripts/python.exe",
    ] {
        let p = root.join(rel);
        if p.is_file() {
            return Some(p);
        }
    }
    None
}
