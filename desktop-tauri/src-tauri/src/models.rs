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
        // try legacy .env migration
        if let Some(store) = import_legacy_env(&root)? {
            return Ok(store);
        }
        return Ok(ModelStore::default());
    }
    let text = fs::read_to_string(&path).map_err(|e| format!("读取模型配置失败: {e}"))?;
    let mut store: ModelStore =
        serde_json::from_str(&text).map_err(|e| format!("解析 models.json 失败: {e}"))?;
    // Normalize: if profiles arrived as a single object from legacy writers, serde already rejects;
    // empty default when missing profile.
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
            values.insert(k.trim().to_string(), v.trim().trim_matches(|c| c == '"' || c == '\'').to_string());
        }
    }
    let model = values.get("UPSTREAM_MODEL").cloned().unwrap_or_default();
    let base_url = values.get("UPSTREAM_BASE_URL").cloned().unwrap_or_default();
    let api_key = values.get("UPSTREAM_API_KEY").cloned().unwrap_or_default();
    if model.is_empty() || base_url.is_empty() || api_key.is_empty() || api_key == "replace-with-new-key"
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
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(30))
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
