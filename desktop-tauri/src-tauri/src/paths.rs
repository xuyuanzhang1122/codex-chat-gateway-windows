use std::path::{Path, PathBuf};

/// Resolve the gateway project root (directory containing config.yaml + scripts/).
pub fn project_root() -> PathBuf {
    if let Ok(explicit) = std::env::var("CODEX_CHAT_GATEWAY_ROOT") {
        let p = PathBuf::from(explicit);
        if is_project_root(&p) {
            return p;
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

    // Dev: desktop-tauri/src-tauri -> repo root
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let m = PathBuf::from(manifest);
        candidates.push(m.join("../.."));
        candidates.push(m.join("../../.."));
    }

    for c in candidates {
        if let Ok(resolved) = c.canonicalize() {
            if is_project_root(&resolved) {
                return resolved;
            }
        } else if is_project_root(&c) {
            return c;
        }
    }

    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
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
    let portable = root.join("runtime").join("pythonw.exe");
    if portable.is_file() {
        return Some(portable);
    }
    let venv = root.join(".venv").join("Scripts").join("pythonw.exe");
    if venv.is_file() {
        return Some(venv);
    }
    let portable_console = root.join("runtime").join("python.exe");
    if portable_console.is_file() {
        return Some(portable_console);
    }
    let venv_console = root.join(".venv").join("Scripts").join("python.exe");
    if venv_console.is_file() {
        return Some(venv_console);
    }
    None
}

