use chrono::Local;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use toml_edit::{value, DocumentMut, Item, Table};

const CODEX_PROVIDER: &str = "local-chat-gateway";
const CLAUDE_PROFILE_ID: &str = "3b6a62c4-e961-55b4-8e65-661d52f99e0d";
const CLAUDE_PROFILE_NAME: &str = "Codex Chat Gateway";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FieldSnapshot {
    present: bool,
    value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CodexSnapshot {
    version: u32,
    model: FieldSnapshot,
    model_provider: FieldSnapshot,
    #[serde(default)]
    local_provider_toml: Option<String>,
}

pub fn configure_codex(port: u16) -> Result<Vec<String>, String> {
    let (config_path, state_path) = codex_paths()?;
    let original = fs::read_to_string(&config_path).unwrap_or_default();
    let mut document = parse_toml(&original)?;
    if !state_path.exists() {
        let snapshot = capture_codex(&document);
        write_json_atomic(
            &state_path,
            &serde_json::to_value(snapshot).map_err(|e| e.to_string())?,
        )?;
    }
    let mut logs = Vec::new();
    if let Some(backup) = backup_file(&config_path)? {
        logs.push(format!("Backup: {}", backup.display()));
    }
    document["model"] = value("codex-chat");
    document["model_provider"] = value(CODEX_PROVIDER);
    if !document["model_providers"].is_table() {
        document["model_providers"] = Item::Table(Table::new());
    }
    let mut provider = Table::new();
    provider["name"] = value("Local Native Protocol Gateway");
    provider["base_url"] = value(format!("http://127.0.0.1:{port}/v1"));
    provider["wire_api"] = value("responses");
    document["model_providers"][CODEX_PROVIDER] = Item::Table(provider);
    write_text_atomic(&config_path, &document.to_string())?;
    logs.push(format!("Configured: {}", config_path.display()));
    logs.push("Fully exit and restart Codex.".into());
    Ok(logs)
}

pub fn restore_codex() -> Result<Vec<String>, String> {
    let (config_path, state_path) = codex_paths()?;
    let original = fs::read_to_string(&config_path).unwrap_or_default();
    let mut document = parse_toml(&original)?;
    let snapshot: CodexSnapshot = if state_path.exists() {
        serde_json::from_str(&fs::read_to_string(&state_path).map_err(|e| e.to_string())?)
            .map_err(|e| format!("restore state is invalid: {e}"))?
    } else {
        CodexSnapshot {
            version: 2,
            model: FieldSnapshot {
                present: false,
                value: None,
            },
            model_provider: FieldSnapshot {
                present: false,
                value: None,
            },
            local_provider_toml: None,
        }
    };
    let mut logs = Vec::new();
    if let Some(backup) = backup_file(&config_path)? {
        logs.push(format!("Backup: {}", backup.display()));
    }
    restore_string_field(&mut document, "model", &snapshot.model);
    restore_string_field(&mut document, "model_provider", &snapshot.model_provider);
    if snapshot.local_provider_toml.is_some() && !document["model_providers"].is_table() {
        document["model_providers"] = Item::Table(Table::new());
    }
    if let Some(providers) = document["model_providers"].as_table_mut() {
        match snapshot.local_provider_toml.as_deref() {
            Some(saved) => {
                providers[CODEX_PROVIDER] = parse_saved_provider(saved)?;
            }
            None => {
                providers.remove(CODEX_PROVIDER);
            }
        }
    }
    write_text_atomic(&config_path, &document.to_string())?;
    logs.push(format!("Restored: {}", config_path.display()));
    logs.push("Other Codex settings, MCP servers, and plugins were preserved.".into());
    Ok(logs)
}

pub fn configure_claude() -> Result<Vec<String>, String> {
    update_claude(true)
}

pub fn restore_claude() -> Result<Vec<String>, String> {
    update_claude(false)
}

fn update_claude(apply: bool) -> Result<Vec<String>, String> {
    let paths = claude_paths()?;
    let all = [
        paths.normal.clone(),
        paths.threep.clone(),
        paths.profile.clone(),
        paths.meta.clone(),
    ];
    let snapshots: HashMap<PathBuf, Option<Vec<u8>>> = all
        .iter()
        .map(|path| (path.clone(), fs::read(path).ok()))
        .collect();
    let result = (|| {
        let mut normal = read_json_object(&paths.normal)?;
        let mut threep = read_json_object(&paths.threep)?;
        normal.insert(
            "deploymentMode".into(),
            json!(if apply { "3p" } else { "1p" }),
        );
        threep.insert(
            "deploymentMode".into(),
            json!(if apply { "3p" } else { "1p" }),
        );
        write_json_atomic(&paths.normal, &Value::Object(normal))?;
        write_json_atomic(&paths.threep, &Value::Object(threep))?;

        let mut meta = read_json_object(&paths.meta)?;
        let mut entries = meta
            .remove("entries")
            .and_then(|value| value.as_array().cloned())
            .unwrap_or_default();
        entries.retain(|entry| entry.get("id").and_then(Value::as_str) != Some(CLAUDE_PROFILE_ID));
        if apply {
            let profile = json!({
                "coworkEgressAllowedHosts": ["*"],
                "disableDeploymentModeChooser": true,
                "inferenceGatewayApiKey": "local-gateway",
                "inferenceGatewayAuthScheme": "bearer",
                "inferenceGatewayBaseUrl": "http://127.0.0.1:4000",
                "inferenceProvider": "gateway",
                "inferenceModels": [
                    {"name": "claude-sonnet-5", "labelOverride": "Codex Chat Gateway (Sonnet)"},
                    {"name": "claude-opus-4-8", "labelOverride": "Codex Chat Gateway (Opus)"},
                    {"name": "claude-haiku-4-5", "labelOverride": "Codex Chat Gateway (Haiku)"}
                ]
            });
            write_json_atomic(&paths.profile, &profile)?;
            entries.push(json!({"id": CLAUDE_PROFILE_ID, "name": CLAUDE_PROFILE_NAME}));
            meta.insert("appliedId".into(), json!(CLAUDE_PROFILE_ID));
        } else {
            if paths.profile.exists() {
                fs::remove_file(&paths.profile).map_err(|e| e.to_string())?;
            }
            if meta.get("appliedId").and_then(Value::as_str) == Some(CLAUDE_PROFILE_ID) {
                if let Some(next) = entries.first().and_then(|entry| entry.get("id")).cloned() {
                    meta.insert("appliedId".into(), next);
                } else {
                    meta.remove("appliedId");
                }
            }
        }
        meta.insert("entries".into(), Value::Array(entries));
        write_json_atomic(&paths.meta, &Value::Object(meta))?;
        Ok::<(), String>(())
    })();
    if let Err(error) = result {
        for (path, content) in snapshots {
            match content {
                Some(bytes) => {
                    if let Some(parent) = path.parent() {
                        let _ = fs::create_dir_all(parent);
                    }
                    let _ = fs::write(path, bytes);
                }
                None => {
                    let _ = fs::remove_file(path);
                }
            }
        }
        return Err(error);
    }
    Ok(vec![if apply {
        format!(
            "Claude Desktop Code profile configured: {}",
            paths.profile.display()
        )
    } else {
        "Claude Desktop was switched back to official 1P mode.".into()
    }])
}

fn codex_paths() -> Result<(PathBuf, PathBuf), String> {
    let home = std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .or_else(|| user_home().map(|path| path.join(".codex")))
        .ok_or("cannot resolve Codex home")?;
    Ok((
        home.join("config.toml"),
        home.join("codex-chat-gateway-restore.json"),
    ))
}

struct ClaudePaths {
    normal: PathBuf,
    threep: PathBuf,
    profile: PathBuf,
    meta: PathBuf,
}

fn claude_paths() -> Result<ClaudePaths, String> {
    let local = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .or_else(|| user_home().map(|path| path.join("AppData/Local")))
        .ok_or("cannot resolve LOCALAPPDATA")?;
    let normal_dir = pick_claude_dir(&local, false);
    let threep_dir = pick_claude_dir(&local, true);
    let library = threep_dir.join("configLibrary");
    Ok(ClaudePaths {
        normal: normal_dir.join("claude_desktop_config.json"),
        threep: threep_dir.join("claude_desktop_config.json"),
        profile: library.join(format!("{CLAUDE_PROFILE_ID}.json")),
        meta: library.join("_meta.json"),
    })
}

fn pick_claude_dir(local: &Path, threep: bool) -> PathBuf {
    let exact = local.join(if threep { "Claude-3p" } else { "Claude" });
    if exact.exists() {
        return exact;
    }
    let mut candidates: Vec<PathBuf> = fs::read_dir(local)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_dir()
                && path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .map(|name| name.starts_with("Claude") && name.contains("-3p") == threep)
                    .unwrap_or(false)
        })
        .collect();
    candidates.sort();
    candidates.into_iter().next().unwrap_or(exact)
}

fn capture_codex(document: &DocumentMut) -> CodexSnapshot {
    let field = |name: &str| FieldSnapshot {
        present: document.get(name).is_some(),
        value: document
            .get(name)
            .and_then(Item::as_str)
            .map(str::to_string),
    };
    CodexSnapshot {
        version: 2,
        model: field("model"),
        model_provider: field("model_provider"),
        local_provider_toml: document
            .get("model_providers")
            .and_then(Item::as_table)
            .and_then(|providers| providers.get(CODEX_PROVIDER))
            .map(Item::to_string),
    }
}

fn restore_string_field(document: &mut DocumentMut, name: &str, snapshot: &FieldSnapshot) {
    if snapshot.present {
        document[name] = value(snapshot.value.clone().unwrap_or_default());
    } else {
        document.remove(name);
    }
}

fn parse_toml(text: &str) -> Result<DocumentMut, String> {
    if text.trim().is_empty() {
        Ok(DocumentMut::new())
    } else {
        text.parse::<DocumentMut>()
            .map_err(|e| format!("Codex config could not be parsed; no changes were made: {e}"))
    }
}

fn parse_saved_provider(text: &str) -> Result<Item, String> {
    let as_value = format!("saved = {text}");
    if let Ok(document) = as_value.parse::<DocumentMut>() {
        if let Some(item) = document.get("saved") {
            return Ok(item.clone());
        }
    }
    let as_table = format!("[saved]\n{text}");
    let document = as_table
        .parse::<DocumentMut>()
        .map_err(|e| format!("saved Codex provider could not be restored: {e}"))?;
    document
        .get("saved")
        .cloned()
        .ok_or_else(|| "saved Codex provider is missing".into())
}

fn read_json_object(path: &Path) -> Result<serde_json::Map<String, Value>, String> {
    if !path.exists() {
        return Ok(serde_json::Map::new());
    }
    let value: Value = serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?)
        .map_err(|e| {
            format!(
                "Cannot safely parse existing JSON file {}: {e}",
                path.display()
            )
        })?;
    value
        .as_object()
        .cloned()
        .ok_or_else(|| format!("Existing JSON root must be an object: {}", path.display()))
}

fn backup_file(path: &Path) -> Result<Option<PathBuf>, String> {
    if !path.exists() {
        return Ok(None);
    }
    let stamp = Local::now().format("%Y%m%d-%H%M%S-%6f");
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("config");
    let backup = path.with_file_name(format!("{name}.bak-{stamp}-chat-gateway"));
    fs::copy(path, &backup).map_err(|e| e.to_string())?;
    Ok(Some(backup))
}

fn write_json_atomic(path: &Path, value: &Value) -> Result<(), String> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    bytes.push(b'\n');
    write_bytes_atomic(path, &bytes)
}

fn write_text_atomic(path: &Path, text: &str) -> Result<(), String> {
    write_bytes_atomic(path, text.as_bytes())
}

fn write_bytes_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let temporary = path.with_extension("tmp-chat-gateway");
    let mut file = fs::File::create(&temporary).map_err(|e| e.to_string())?;
    file.write_all(bytes).map_err(|e| e.to_string())?;
    file.sync_all().ok();
    fs::rename(temporary, path).map_err(|e| e.to_string())
}

fn user_home() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE").map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_codex_provider_round_trips() {
        let document = r#"
[model_providers.local-chat-gateway]
name = "previous provider"
base_url = "http://127.0.0.1:4999/v1"
wire_api = "responses"
"#
        .parse::<DocumentMut>()
        .unwrap();
        let saved = document["model_providers"][CODEX_PROVIDER].to_string();
        let restored = parse_saved_provider(&saved).unwrap();
        assert_eq!(restored["name"].as_str(), Some("previous provider"));
        assert_eq!(
            restored["base_url"].as_str(),
            Some("http://127.0.0.1:4999/v1")
        );
    }
}
