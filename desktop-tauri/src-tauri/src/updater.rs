//! In-app update delivery: download the full Studio (Inno) installer from the
//! GitHub Release, verify its SHA-256, launch it, and exit the console.
//!
//! The Tauri NSIS updater package is intentionally NOT used for installation:
//! it bundles only the bare console exe and installs to a different directory
//! than the Studio (Inno) installer, which produced broken half-installs
//! (no gateway runtime, no VERSION file, endless update prompts).

use serde_json::json;
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use std::process::Command;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

const RELEASE_DOWNLOAD_BASE: &str =
    "https://github.com/xuyuanzhang1122/codex-chat-gateway-windows/releases/download";

fn installer_name(version: &str) -> String {
    format!("CodexChatGateway-Studio-Setup-v{version}.exe")
}

fn http_get(url: &str) -> Result<ureq::Response, String> {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(15))
        .build();
    agent
        .get(url)
        .set("User-Agent", "codex-chat-gateway-updater")
        .call()
        .map_err(|e| format!("下载失败 {url}: {e}"))
}

/// Fetch "<sha256>  <filename>" sidecar from the same release and keep the hash.
fn expected_sha256(version: &str) -> Result<String, String> {
    let url = format!(
        "{RELEASE_DOWNLOAD_BASE}/v{version}/{}.sha256",
        installer_name(version)
    );
    let text = http_get(&url)?
        .into_string()
        .map_err(|e| format!("读取 SHA-256 清单失败: {e}"))?;
    text.split_whitespace()
        .next()
        .filter(|h| h.len() == 64 && h.chars().all(|c| c.is_ascii_hexdigit()))
        .map(|h| h.to_ascii_lowercase())
        .ok_or_else(|| "安装包 SHA-256 清单格式异常".into())
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

/// Download the Studio installer for `version`, verify it, launch it, and exit.
/// Progress is streamed as `update://progress` events
/// (`{ downloaded, total, verified? }`).
#[tauri::command]
pub async fn download_studio_installer(app: AppHandle, version: String) -> Result<(), String> {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() != 3 || !parts.iter().all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit())) {
        return Err(format!("非法版本号: {version}"));
    }

    let expected = expected_sha256(&version)?;
    let url = format!(
        "{RELEASE_DOWNLOAD_BASE}/v{version}/{}",
        installer_name(&version)
    );
    let dest = std::env::temp_dir().join(installer_name(&version));

    let resp = http_get(&url)?;
    let total = resp
        .header("Content-Length")
        .and_then(|h| h.parse::<u64>().ok());
    let mut reader = resp.into_reader();
    let mut file = File::create(&dest).map_err(|e| format!("无法写入 {}: {e}", dest.display()))?;
    let mut downloaded: u64 = 0;
    let mut buf = [0u8; 256 * 1024];
    loop {
        let n = reader.read(&mut buf).map_err(|e| format!("下载中断: {e}"))?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        downloaded += n as u64;
        let _ = app.emit(
            "update://progress",
            json!({ "downloaded": downloaded, "total": total }),
        );
    }
    drop(file);

    let actual = sha256_file(&dest)?;
    if actual != expected {
        let _ = fs::remove_file(&dest);
        return Err("安装包 SHA-256 校验失败，已删除下载文件".into());
    }

    let _ = app.emit(
        "update://progress",
        json!({ "downloaded": downloaded, "total": total, "verified": true }),
    );

    // Interactive installer: same wizard as a manual install, upgrades in place
    // (CloseApplications=yes handles the still-running console). User data under
    // .gateway/ is excluded from both install and uninstall by the .iss script.
    Command::new(&dest)
        .spawn()
        .map_err(|e| format!("无法启动安装器 {}: {e}", dest.display()))?;
    app.exit(0);
    Ok(())
}
