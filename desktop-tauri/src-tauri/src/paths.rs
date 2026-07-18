use std::path::{Path, PathBuf};
use std::sync::OnceLock;

static ROOT: OnceLock<PathBuf> = OnceLock::new();

/// Resolve once and cache the gateway project root (config.yaml + scripts/).
/// Always returns a path **without** the Windows `\\?\` extended prefix so
/// PowerShell `$PSScriptRoot` and process cmdline matching stay consistent.
pub fn project_root() -> PathBuf {
    ROOT.get_or_init(discover_root).clone()
}

fn discover_root() -> PathBuf {
    if let Ok(explicit) = std::env::var("CODEX_CHAT_GATEWAY_ROOT") {
        let p = PathBuf::from(explicit);
        if is_project_root(&p) {
            return canonicalize_clean(&p);
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
            return canonicalize_clean(&c);
        }
        if let Ok(resolved) = c.canonicalize() {
            let cleaned = strip_extended_prefix(resolved);
            if is_project_root(&cleaned) {
                return cleaned;
            }
        }
    }

    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn canonicalize_clean(path: &Path) -> PathBuf {
    strip_extended_prefix(path.canonicalize().unwrap_or_else(|_| path.to_path_buf()))
}

/// Remove Windows extended-length prefix `\\?\` / `//?/` so paths interop with
/// PowerShell, CreateProcess consumers, and sysinfo command lines.
pub fn strip_extended_prefix(path: PathBuf) -> PathBuf {
    let raw = path.to_string_lossy();
    let trimmed = raw
        .strip_prefix(r"\\?\")
        .or_else(|| raw.strip_prefix("//?/"))
        .unwrap_or(raw.as_ref());
    PathBuf::from(trimmed)
}

/// Normalize path text for comparison (no extended prefix, backslashes, lowercase ASCII).
pub fn normalize_path_text(path: &Path) -> String {
    let cleaned = strip_extended_prefix(path.to_path_buf());
    cleaned
        .to_string_lossy()
        .replace('/', "\\")
        .to_ascii_lowercase()
}

/// Normalize free-form cmdline / path strings the same way as [`normalize_path_text`].
pub fn normalize_text(s: &str) -> String {
    s.trim_start_matches(r"\\?\")
        .trim_start_matches("//?/")
        .replace('/', "\\")
        .to_ascii_lowercase()
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
            return Some(strip_extended_prefix(p));
        }
    }
    None
}

/// Human-readable root without `\\?\` (for logs / UI).
pub fn project_root_display() -> String {
    project_root().to_string_lossy().into_owned()
}
