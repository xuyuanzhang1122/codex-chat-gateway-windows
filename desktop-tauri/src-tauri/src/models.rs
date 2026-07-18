use crate::paths::{models_path, project_root};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelProfile {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub model_id: String,
    pub litellm_model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelStore {
    pub version: i32,
    pub default_id: String,
    pub profiles: Vec<ModelProfile>,
}

impl Default for ModelStore {
    fn default() -> Self {
        Self {
            version: 1,
            default_id: String::new(),
            profiles: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInput {
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub model_id: String,
}

pub fn litellm_model(base_url: &str, model_id: &str) -> String {
    if base_url.to_ascii_lowercase().contains("deepseek") {
        if model_id.to_ascii_lowercase().starts_with("deepseek/") {
            return model_id.to_string();
        }
        return format!("deepseek/{model_id}");
    }
    if model_id.to_ascii_lowercase().starts_with("openai/") {
        return model_id.to_string();
    }
    format!("openai/{model_id}")
}

pub fn claude_litellm_model(litellm_model: &str) -> String {
    if let Some(rest) = litellm_model.strip_prefix("openai/") {
        return format!("custom_openai/{rest}");
    }
    litellm_model.to_string()
}

pub fn read_store() -> Result<ModelStore, String> {
    let root = project_root();
    let path = models_path(&root);
    if !path.exists() {
        if let Some(store) = import_legacy_env(&root)? {
            return Ok(store);
        }
        return Ok(ModelStore::default());
    }
    let text = fs::read_to_string(&path).map_err(|e| format!("读取模型配置失败: {e}"))?;
    let mut store: ModelStore =
        serde_json::from_str(&text).map_err(|e| format!("解析 models.json 失败: {e}"))?;
    if store.default_id.is_empty() {
        if let Some(first) = store.profiles.first() {
            store.default_id = first.id.clone();
        }
    }
    Ok(store)
}

pub fn save_store(store: &ModelStore) -> Result<(), String> {
    let root = project_root();
    let path = models_path(&root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("创建 .gateway 失败: {e}"))?;
    }
    let json = serde_json::to_string_pretty(store).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("json.tmp");
    {
        let mut f = fs::File::create(&tmp).map_err(|e| format!("写入临时文件失败: {e}"))?;
        f.write_all(json.as_bytes())
            .map_err(|e| format!("写入临时文件失败: {e}"))?;
        f.sync_all().ok();
    }
    fs::rename(&tmp, &path).map_err(|e| format!("保存 models.json 失败: {e}"))?;
    Ok(())
}

fn import_legacy_env(root: &Path) -> Result<Option<ModelStore>, String> {
    let store_path = models_path(root);
    if store_path.exists() {
        return Ok(None);
    }
    let env_path = root.join(".env");
    if !env_path.exists() {
        return Ok(None);
    }
    let text = fs::read_to_string(&env_path).map_err(|e| e.to_string())?;
    let mut values = std::collections::HashMap::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = trimmed.split_once('=') {
            values.insert(
                k.trim().to_string(),
                v.trim()
                    .trim_matches(|c| c == '"' || c == '\'')
                    .to_string(),
            );
        }
    }
    let model = values.get("UPSTREAM_MODEL").cloned().unwrap_or_default();
    let base_url = values.get("UPSTREAM_BASE_URL").cloned().unwrap_or_default();
    let api_key = values.get("UPSTREAM_API_KEY").cloned().unwrap_or_default();
    if model.is_empty()
        || base_url.is_empty()
        || api_key.is_empty()
        || api_key == "replace-with-new-key"
    {
        return Ok(None);
    }
    let model_id = model
        .split_once('/')
        .map(|(_, r)| r.to_string())
        .unwrap_or_else(|| model.clone());
    let id = Uuid::new_v4().simple().to_string();
    let profile = ModelProfile {
        id: id.clone(),
        name: "Imported model".into(),
        base_url: base_url.trim_end_matches('/').to_string(),
        api_key,
        model_id,
        litellm_model: model,
    };
    let store = ModelStore {
        version: 1,
        default_id: id,
        profiles: vec![profile],
    };
    save_store(&store)?;
    Ok(Some(store))
}

pub fn default_profile(store: &ModelStore) -> Option<&ModelProfile> {
    store
        .profiles
        .iter()
        .find(|p| p.id == store.default_id)
        .or_else(|| store.profiles.first())
}

pub fn add_profile(input: ModelInput) -> Result<ModelStore, String> {
    validate_input(&input)?;
    let mut store = read_store()?;
    let id = Uuid::new_v4().simple().to_string();
    let base_url = input.base_url.trim().trim_end_matches('/').to_string();
    let model_id = input.model_id.trim().to_string();
    let profile = ModelProfile {
        id: id.clone(),
        name: input.name.trim().to_string(),
        base_url: base_url.clone(),
        api_key: input.api_key,
        model_id: model_id.clone(),
        litellm_model: litellm_model(&base_url, &model_id),
    };
    if store.default_id.is_empty() {
        store.default_id = id;
    }
    store.profiles.push(profile);
    save_store(&store)?;
    Ok(store)
}

pub fn update_profile(id: &str, input: ModelInput) -> Result<ModelStore, String> {
    validate_input(&input)?;
    let mut store = read_store()?;
    let Some(profile) = store.profiles.iter_mut().find(|p| p.id == id) else {
        return Err("未找到该模型配置".into());
    };
    let base_url = input.base_url.trim().trim_end_matches('/').to_string();
    let model_id = input.model_id.trim().to_string();
    profile.name = input.name.trim().to_string();
    profile.base_url = base_url.clone();
    profile.api_key = input.api_key;
    profile.model_id = model_id.clone();
    profile.litellm_model = litellm_model(&base_url, &model_id);
    save_store(&store)?;
    Ok(store)
}

pub fn delete_profile(id: &str) -> Result<ModelStore, String> {
    let mut store = read_store()?;
    store.profiles.retain(|p| p.id != id);
    if store.default_id == id {
        store.default_id = store
            .profiles
            .first()
            .map(|p| p.id.clone())
            .unwrap_or_default();
    }
    save_store(&store)?;
    Ok(store)
}

pub fn set_default(id: &str) -> Result<ModelStore, String> {
    let mut store = read_store()?;
    if !store.profiles.iter().any(|p| p.id == id) {
        return Err("未找到该模型配置".into());
    }
    store.default_id = id.to_string();
    save_store(&store)?;
    Ok(store)
}

fn validate_input(input: &ModelInput) -> Result<(), String> {
    if input.name.trim().is_empty() {
        return Err("请填写配置名称".into());
    }
    let url = input.base_url.trim();
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err("请填写有效的 HTTP(S) API 地址".into());
    }
    if input.api_key.trim().is_empty() {
        return Err("请填写 API Key".into());
    }
    if input.model_id.trim().is_empty() {
        return Err("请填写模型 ID".into());
    }
    Ok(())
}

pub fn fetch_remote_models(base_url: &str, api_key: &str) -> Result<Vec<String>, String> {
    let base = base_url.trim().trim_end_matches('/');
    let url = format!("{base}/models");
    // Upstream model listing only — keep TLS dependency minimal (http ok for many local proxies)
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(20))
        .build();
    let resp = agent
        .get(&url)
        .set("Authorization", &format!("Bearer {api_key}"))
        .set("Accept", "application/json")
        .call()
        .map_err(|e| format!("获取模型列表失败: {e}"))?;
    let value: serde_json::Value = resp
        .into_json()
        .map_err(|e| format!("解析模型列表失败: {e}"))?;
    let data = value
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| "接口返回中没有 data 字段".to_string())?;
    let mut ids: Vec<String> = data
        .iter()
        .filter_map(|item| item.get("id").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .filter(|s| !s.is_empty())
        .collect();
    ids.sort_by(|a, b| a.to_ascii_lowercase().cmp(&b.to_ascii_lowercase()));
    ids.dedup();
    if ids.is_empty() {
        return Err("该接口没有返回任何模型".into());
    }
    Ok(ids)
}

/// Parsed plaintext model config (e.g. api.txt).
///
/// Supported shapes (keys case-insensitive; `:` / `：` / `=` delimiters):
/// ```text
/// baseurl：https://api.deepseek.com
/// key:sk-xxx
/// model:deepseek-v4-flash,deepseek-v4-pro
/// ```
/// `model` may be empty; callers should offer online list fetch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedApiText {
    pub base_url: String,
    pub api_key: String,
    pub models: Vec<String>,
    pub name_hint: Option<String>,
    /// True when model key was absent or blank after parse.
    pub model_missing: bool,
}

pub fn parse_api_text(text: &str) -> Result<ParsedApiText, String> {
    let mut map: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
            continue;
        }
        let (k, v) = split_kv(line).ok_or_else(|| {
            format!("无法解析行（需要 key:value）: {}", truncate_for_err(line))
        })?;
        let key = normalize_key(&k);
        if key.is_empty() {
            continue;
        }
        map.insert(key, v.trim().to_string());
    }

    let base_url = take_first(
        &map,
        &[
            "baseurl",
            "base_url",
            "apiurl",
            "api_url",
            "url",
            "endpoint",
            "host",
        ],
    )
    .unwrap_or_default();
    let api_key = take_first(
        &map,
        &["key", "api_key", "apikey", "token", "secret", "authorization"],
    )
    .unwrap_or_default();
    let model_raw = take_first(
        &map,
        &["model", "models", "model_id", "modelid", "model_name"],
    );
    let name_hint = take_first(&map, &["name", "title", "label"]);

    let base_url = base_url.trim().trim_end_matches('/').to_string();
    if !(base_url.starts_with("http://") || base_url.starts_with("https://")) {
        return Err("未识别到有效的 baseurl / API 地址（需 http/https）".into());
    }
    if api_key.trim().is_empty() {
        return Err("未识别到 API Key（key / api_key）".into());
    }

    let model_missing = model_raw.as_ref().map(|s| s.trim().is_empty()).unwrap_or(true);
    let models = model_raw
        .map(|s| split_models(&s))
        .unwrap_or_default();

    Ok(ParsedApiText {
        base_url,
        api_key: api_key.trim().to_string(),
        models,
        name_hint,
        model_missing,
    })
}

/// Import one or more model IDs with shared base_url + key. Empty `model_ids` is an error.
pub fn import_profiles(
    base_url: &str,
    api_key: &str,
    model_ids: &[String],
    name_hint: Option<&str>,
) -> Result<ModelStore, String> {
    if model_ids.is_empty() {
        return Err("没有可导入的模型 ID".into());
    }
    let base_url = base_url.trim().trim_end_matches('/').to_string();
    if !(base_url.starts_with("http://") || base_url.starts_with("https://")) {
        return Err("请填写有效的 HTTP(S) API 地址".into());
    }
    if api_key.trim().is_empty() {
        return Err("请填写 API Key".into());
    }

    let mut store = read_store()?;
    let host_hint = host_label(&base_url);
    for (idx, model_id) in model_ids.iter().enumerate() {
        let mid = model_id.trim();
        if mid.is_empty() {
            continue;
        }
        let name = if model_ids.len() == 1 {
            name_hint
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .unwrap_or_else(|| mid.to_string())
        } else if idx == 0 {
            name_hint
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|s| format!("{s} · {mid}"))
                .unwrap_or_else(|| format!("{host_hint} · {mid}"))
        } else {
            format!("{host_hint} · {mid}")
        };
        // Skip exact duplicates (same url + model_id)
        if store
            .profiles
            .iter()
            .any(|p| p.base_url == base_url && p.model_id == mid)
        {
            continue;
        }
        let id = Uuid::new_v4().simple().to_string();
        let profile = ModelProfile {
            id: id.clone(),
            name,
            base_url: base_url.clone(),
            api_key: api_key.trim().to_string(),
            model_id: mid.to_string(),
            litellm_model: litellm_model(&base_url, mid),
        };
        if store.default_id.is_empty() {
            store.default_id = id;
        }
        store.profiles.push(profile);
    }
    if store.profiles.is_empty() {
        return Err("导入结果为空（可能全部与现有配置重复）".into());
    }
    save_store(&store)?;
    Ok(store)
}

fn split_kv(line: &str) -> Option<(String, String)> {
    // Full-width colon first (common in Chinese notes).
    if let Some((k, v)) = line.split_once('：') {
        let key = k.trim();
        if !key.is_empty() {
            return Some((key.to_string(), v.to_string()));
        }
    }
    // Prefer '=' over ASCII ':' so `base_url=https://...` is not split at "https:".
    if let Some((k, v)) = line.split_once('=') {
        let key = k.trim();
        if !key.is_empty() && !key.contains(':') && !key.contains('/') {
            return Some((key.to_string(), v.to_string()));
        }
    }
    // `key:value` — reject splits where the left side looks like a URL scheme/path.
    if let Some((k, v)) = line.split_once(':') {
        let key = k.trim();
        if !key.is_empty()
            && !key.contains('/')
            && key
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return Some((key.to_string(), v.to_string()));
        }
    }
    None
}

fn normalize_key(k: &str) -> String {
    k.trim()
        .trim_matches(|c: char| c == '"' || c == '\'' || c == '【' || c == '】' || c == '[' || c == ']')
        .chars()
        .map(|c| if c == '-' || c == ' ' { '_' } else { c })
        .collect::<String>()
        .to_ascii_lowercase()
}

fn take_first(map: &std::collections::HashMap<String, String>, keys: &[&str]) -> Option<String> {
    for k in keys {
        if let Some(v) = map.get(*k) {
            return Some(v.clone());
        }
    }
    None
}

fn split_models(raw: &str) -> Vec<String> {
    let mut out = Vec::new();
    for part in raw.split(|c: char| c == ',' || c == ';' || c == '|' || c == '\n' || c == '、') {
        let m = part.trim().trim_matches(|c| c == '"' || c == '\'').to_string();
        if !m.is_empty() && !out.iter().any(|x: &String| x == &m) {
            out.push(m);
        }
    }
    out
}

fn host_label(base_url: &str) -> String {
    base_url
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .split('/')
        .next()
        .unwrap_or("upstream")
        .to_string()
}

fn truncate_for_err(s: &str) -> String {
    let t = s.trim();
    if t.chars().count() <= 48 {
        t.to_string()
    } else {
        format!("{}…", t.chars().take(48).collect::<String>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_api_txt_sample() {
        let text = "baseurl：https://api.deepseek.com\nkey:sk-12334454464\nmodel:deepseek-v4-flash,deepseek-v4-pro\n";
        let p = parse_api_text(text).unwrap();
        assert_eq!(p.base_url, "https://api.deepseek.com");
        assert_eq!(p.api_key, "sk-12334454464");
        assert_eq!(p.models, vec!["deepseek-v4-flash", "deepseek-v4-pro"]);
        assert!(!p.model_missing);
    }

    #[test]
    fn parse_empty_model_prompts_fetch() {
        let text = "base_url=https://api.example.com/v1\napi_key: sk-abc\nmodel:\n";
        let p = parse_api_text(text).unwrap();
        assert!(p.models.is_empty());
        assert!(p.model_missing);
    }
}
