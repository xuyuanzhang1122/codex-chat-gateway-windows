use std::path::{Path, PathBuf};
use std::sync::OnceLock;

static ROOT: OnceLock<PathBuf> = OnceLock::new();

/// Resolve once and cache the Studio/native-gateway project root.
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
    normalize_text(&path.to_string_lossy())
}

/// Normalize free-form cmdline / path strings the same way as [`normalize_path_text`].
/// Strips **all** Windows extended-length markers (`\\?\` / `//?/`), not only a leading one —
/// process command lines often embed them mid-string after quoted argv[0].
pub fn normalize_text(s: &str) -> String {
    s.replace(r"\\?\", "")
        .replace("//?/", "")
        .replace('/', "\\")
        .to_ascii_lowercase()
}

fn is_project_root(path: &Path) -> bool {
    path.join("VERSION").is_file()
        && path.join("scripts").is_dir()
        && (path.join("desktop-tauri").is_dir()
            || path.join("ccg-native-gateway.exe").is_file()
            || path.join("ccg-native-gateway").is_file())
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

pub fn native_gateway_binary(root: &Path) -> Option<PathBuf> {
    for rel in [
        "ccg-native-gateway.exe",
        "native-gateway/target/release/ccg-native-gateway.exe",
        "native-gateway/target/debug/ccg-native-gateway.exe",
        "ccg-native-gateway",
        "native-gateway/target/release/ccg-native-gateway",
        "native-gateway/target/debug/ccg-native-gateway",
    ] {
        let path = root.join(rel);
        if path.is_file() {
            return Some(strip_extended_prefix(path));
        }
    }
    None
}

/// Human-readable root without `\\?\` (for logs / UI).
pub fn project_root_display() -> String {
    project_root().to_string_lossy().into_owned()
}
