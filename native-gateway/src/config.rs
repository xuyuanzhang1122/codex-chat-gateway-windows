use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WireProtocol {
    #[default]
    OpenaiChat,
    OpenaiResponses,
    AnthropicMessages,
}

impl WireProtocol {
    pub fn endpoint(self) -> &'static str {
        match self {
            Self::OpenaiChat => "chat/completions",
            Self::OpenaiResponses => "responses",
            Self::AnthropicMessages => "messages",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthMode {
    #[default]
    Auto,
    Bearer,
    XApiKey,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub model_id: String,
    #[serde(default)]
    pub protocol: WireProtocol,
    #[serde(default)]
    pub auth_mode: AuthMode,
    #[serde(default = "default_true")]
    pub routing_enabled: bool,
    #[serde(default = "default_weight")]
    pub routing_weight: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelRule {
    pub model_id: String,
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RoutingSettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub model_rules: Vec<ModelRule>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelStore {
    #[serde(default)]
    pub default_id: String,
    #[serde(default)]
    pub profiles: Vec<Profile>,
    #[serde(default)]
    pub routing: RoutingSettings,
}

impl ModelStore {
    pub fn load(path: &Path) -> Result<Self, String> {
        let text =
            fs::read_to_string(path).map_err(|e| format!("failed to read model store: {e}"))?;
        let mut store: Self =
            serde_json::from_str(&text).map_err(|e| format!("failed to parse model store: {e}"))?;
        for profile in &mut store.profiles {
            profile.routing_weight = profile.routing_weight.clamp(1, 100);
        }
        store.validate()?;
        Ok(store)
    }

    pub fn validate(&self) -> Result<(), String> {
        let default = self
            .default_profile()
            .ok_or("no upstream profile configured")?;
        if !(default.base_url.starts_with("http://") || default.base_url.starts_with("https://")) {
            return Err("default upstream has an invalid HTTP(S) URL".into());
        }
        if default.api_key.trim().is_empty() {
            return Err("default upstream has no API key".into());
        }
        Ok(())
    }

    pub fn default_profile(&self) -> Option<&Profile> {
        self.profiles
            .iter()
            .find(|profile| profile.id == self.default_id)
            .or_else(|| self.profiles.first())
    }

    pub fn candidates<'a>(
        &'a self,
        affinity_key: Option<&str>,
        counter: &AtomicU64,
    ) -> Vec<&'a Profile> {
        let Some(default) = self.default_profile() else {
            return Vec::new();
        };
        let target = normalized_model(&default.model_id);
        let routing_enabled = self
            .routing
            .model_rules
            .iter()
            .any(|rule| rule.enabled && normalized_model(&rule.model_id) == target);
        let mut pool: Vec<&Profile> = if routing_enabled {
            self.profiles
                .iter()
                .filter(|profile| {
                    profile.routing_enabled && normalized_model(&profile.model_id) == target
                })
                .collect()
        } else {
            vec![default]
        };
        if pool.is_empty() {
            pool.push(default);
        }

        let total: u64 = pool
            .iter()
            .map(|profile| profile.routing_weight.max(1) as u64)
            .sum();
        let slot = affinity_key
            .map(stable_slot)
            .unwrap_or_else(|| counter.fetch_add(1, Ordering::Relaxed))
            % total.max(1);
        let mut cursor = 0u64;
        let preferred = pool
            .iter()
            .position(|profile| {
                cursor += profile.routing_weight.max(1) as u64;
                slot < cursor
            })
            .unwrap_or(0);
        pool.rotate_left(preferred);
        pool
    }
}

fn default_true() -> bool {
    true
}

fn default_weight() -> u32 {
    1
}

pub fn normalized_model(value: &str) -> String {
    let value = value.trim().to_ascii_lowercase();
    match value.split_once('/') {
        Some(("openai" | "custom_openai" | "deepseek", rest)) => rest.to_string(),
        _ => value,
    }
}

fn stable_slot(value: &str) -> u64 {
    let digest = Sha256::digest(value.as_bytes());
    u64::from_be_bytes(digest[..8].try_into().expect("sha256 prefix"))
}

pub fn affinity_key(body: &serde_json::Value) -> Option<String> {
    for field in ["previous_response_id", "prompt_cache_key", "user"] {
        if let Some(value) = body.get(field).and_then(|value| value.as_str()) {
            if !value.is_empty() {
                return Some(format!("{field}:{value}"));
            }
        }
    }
    body.get("metadata")
        .and_then(|value| value.as_object())
        .and_then(|metadata| {
            ["session_id", "conversation_id", "user_id"]
                .iter()
                .find_map(|key| metadata.get(*key).and_then(|value| value.as_str()))
        })
        .filter(|value| !value.is_empty())
        .map(|value| format!("metadata:{value}"))
}

pub fn model_aliases() -> HashMap<&'static str, &'static str> {
    HashMap::from([
        ("codex-chat", "Codex Responses"),
        ("claude-sonnet-5", "Claude Sonnet"),
        ("claude-opus-4-8", "Claude Opus"),
        ("claude-haiku-4-5", "Claude Haiku"),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(id: &str, weight: u32) -> Profile {
        Profile {
            id: id.into(),
            name: id.into(),
            base_url: "https://example.test/v1".into(),
            api_key: "test".into(),
            model_id: "gpt-test".into(),
            protocol: WireProtocol::OpenaiChat,
            auth_mode: AuthMode::Auto,
            routing_enabled: true,
            routing_weight: weight,
        }
    }

    #[test]
    fn v3_profiles_default_to_chat() {
        let parsed: Profile = serde_json::from_value(serde_json::json!({
            "id": "a", "name": "a", "base_url": "https://example.test/v1",
            "api_key": "secret", "model_id": "model"
        }))
        .unwrap();
        assert_eq!(parsed.protocol, WireProtocol::OpenaiChat);
        assert_eq!(parsed.auth_mode, AuthMode::Auto);
    }

    #[test]
    fn affinity_keeps_weighted_choice_stable() {
        let store = ModelStore {
            default_id: "a".into(),
            profiles: vec![profile("a", 3), profile("b", 1)],
            routing: RoutingSettings {
                enabled: true,
                model_rules: vec![ModelRule {
                    model_id: "gpt-test".into(),
                    enabled: true,
                }],
            },
        };
        let counter = AtomicU64::new(0);
        let first = store.candidates(Some("session-1"), &counter)[0].id.clone();
        let second = store.candidates(Some("session-1"), &counter)[0].id.clone();
        assert_eq!(first, second);
    }
}
