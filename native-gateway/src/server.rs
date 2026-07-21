use crate::config::{affinity_key, model_aliases, AuthMode, ModelStore, Profile, WireProtocol};
use crate::protocol::{convert_request, convert_response, StreamBridge};
use async_stream::try_stream;
use axum::body::{Body, Bytes};
use axum::extract::State;
use axum::http::{HeaderMap, HeaderName, HeaderValue, StatusCode};
use axum::response::Response;
use axum::routing::{get, post};
use axum::Router;
use chrono::Utc;
use eventsource_stream::Eventsource;
use futures_util::StreamExt;
use serde_json::{json, Value};
use std::io;
use std::path::PathBuf;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

const HOST: &str = "127.0.0.1";

#[derive(Clone)]
pub struct GatewayState {
    store: Arc<RwLock<ModelStore>>,
    models_path: PathBuf,
    traffic_path: PathBuf,
    client: reqwest::Client,
    selection_counter: Arc<AtomicU64>,
    telemetry_lock: Arc<Mutex<()>>,
}

impl GatewayState {
    pub fn load(models_path: PathBuf, traffic_path: PathBuf) -> Result<Self, String> {
        let store = ModelStore::load(&models_path)?;
        let client = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(15))
            .timeout(std::time::Duration::from_secs(600))
            .pool_idle_timeout(std::time::Duration::from_secs(90))
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Self {
            store: Arc::new(RwLock::new(store)),
            models_path,
            traffic_path,
            client,
            selection_counter: Arc::new(AtomicU64::new(0)),
            telemetry_lock: Arc::new(Mutex::new(())),
        })
    }

    async fn reload(&self) -> Result<usize, String> {
        let store = ModelStore::load(&self.models_path)?;
        let count = store.profiles.len();
        *self.store.write().await = store;
        Ok(count)
    }

    async fn candidates(&self, body: &Value) -> Vec<Profile> {
        let key = affinity_key(body);
        self.store
            .read()
            .await
            .candidates(key.as_deref(), &self.selection_counter)
            .into_iter()
            .cloned()
            .collect()
    }

    async fn record_hit(&self, profile: &Profile) {
        let _guard = self.telemetry_lock.lock().await;
        let now = Utc::now().to_rfc3339();
        let mut document = std::fs::read_to_string(&self.traffic_path)
            .ok()
            .and_then(|text| serde_json::from_str::<Value>(&text).ok())
            .unwrap_or_else(|| json!({"version": 1, "routes": []}));
        let routes = document
            .get_mut("routes")
            .and_then(Value::as_array_mut)
            .expect("new telemetry document has routes");
        if let Some(route) = routes.iter_mut().find(|route| {
            route.get("profile_id").and_then(Value::as_str) == Some(profile.id.as_str())
                && route.get("model_id").and_then(Value::as_str) == Some(profile.model_id.as_str())
        }) {
            let hits = route.get("hit_count").and_then(Value::as_u64).unwrap_or(0);
            route["hit_count"] = json!(hits + 1);
            route["last_seen_at"] = json!(now);
        } else {
            routes.push(json!({
                "model_id": profile.model_id,
                "profile_id": profile.id,
                "profile_name": profile.name,
                "upstream_host": upstream_host(&profile.base_url),
                "hit_count": 1,
                "first_seen_at": now,
                "last_seen_at": now,
            }));
        }
        if let Some(parent) = self.traffic_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(text) = serde_json::to_string_pretty(&document) {
            let temporary = self.traffic_path.with_extension("json.tmp");
            if std::fs::write(&temporary, text).is_ok() {
                let _ = std::fs::rename(temporary, &self.traffic_path);
            }
        }
    }
}

pub async fn run(state: GatewayState, port: u16) -> Result<(), String> {
    let app = Router::new()
        .route("/health/liveliness", get(health))
        .route("/health", get(health))
        .route("/v1/models", get(models))
        .route("/models", get(models))
        .route("/v1/responses", post(responses))
        .route("/responses", post(responses))
        .route("/v1/messages", post(messages))
        .route("/messages", post(messages))
        .route("/v1/chat/completions", post(chat_completions))
        .route("/chat/completions", post(chat_completions))
        .route("/internal/ccg/reload", post(reload))
        .with_state(state);
    let address = format!("{HOST}:{port}");
    let listener = tokio::net::TcpListener::bind(&address)
        .await
        .map_err(|e| format!("failed to bind {address}: {e}"))?;
    println!("native gateway ready on http://{address}");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|e| e.to_string())
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}

async fn health() -> Response {
    json_response(
        StatusCode::OK,
        json!({"status": "ok", "engine": "native-rust"}),
    )
}

async fn models() -> Response {
    let data: Vec<Value> = model_aliases()
        .keys()
        .map(|id| json!({"id": id, "object": "model", "owned_by": "local"}))
        .collect();
    json_response(StatusCode::OK, json!({"object": "list", "data": data}))
}

async fn reload(State(state): State<GatewayState>) -> Response {
    match state.reload().await {
        Ok(profiles) => json_response(
            StatusCode::OK,
            json!({
                "ok": true,
                "profiles": profiles,
                "routes": model_aliases().keys().collect::<Vec<_>>()
            }),
        ),
        Err(error) => error_response(StatusCode::BAD_REQUEST, error),
    }
}

async fn responses(State(state): State<GatewayState>, headers: HeaderMap, body: Bytes) -> Response {
    proxy(state, headers, body, WireProtocol::OpenaiResponses).await
}

async fn messages(State(state): State<GatewayState>, headers: HeaderMap, body: Bytes) -> Response {
    proxy(state, headers, body, WireProtocol::AnthropicMessages).await
}

async fn chat_completions(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    proxy(state, headers, body, WireProtocol::OpenaiChat).await
}

async fn proxy(
    state: GatewayState,
    incoming_headers: HeaderMap,
    raw_body: Bytes,
    client_protocol: WireProtocol,
) -> Response {
    let original: Value = match serde_json::from_slice(&raw_body) {
        Ok(value) => value,
        Err(error) => {
            return error_response(StatusCode::BAD_REQUEST, format!("invalid JSON: {error}"))
        }
    };
    let wants_stream = original
        .get("stream")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let candidates = state.candidates(&original).await;
    if candidates.is_empty() {
        return error_response(StatusCode::SERVICE_UNAVAILABLE, "no upstream configured");
    }

    let candidate_count = candidates.len();
    let mut last_failure: Option<(StatusCode, Bytes, HeaderMap)> = None;
    let mut last_error = String::new();
    for (index, profile) in candidates.into_iter().enumerate() {
        let mut outbound =
            match convert_request(original.clone(), client_protocol, profile.protocol) {
                Ok(value) => value,
                Err(error) => {
                    last_error = format!("request conversion failed: {error}");
                    continue;
                }
            };
        if let Some(object) = outbound.as_object_mut() {
            object.insert("model".into(), Value::String(profile.model_id.clone()));
            object.insert("stream".into(), Value::Bool(wants_stream));
        }
        let url = upstream_url(&profile.base_url, profile.protocol);
        let request = add_request_headers(
            state.client.post(url).json(&outbound),
            &profile,
            &incoming_headers,
        );
        match request.send().await {
            Ok(response) => {
                let status = response.status();
                if (status.as_u16() == 429 || status.is_server_error())
                    && index + 1 < candidate_count
                {
                    let headers = response.headers().clone();
                    let bytes = response.bytes().await.unwrap_or_default();
                    last_failure = Some((status, bytes, headers));
                    continue;
                }
                state.record_hit(&profile).await;
                return process_response(response, profile.protocol, client_protocol, wants_stream)
                    .await;
            }
            Err(error) => {
                last_error = format!("upstream request failed: {error}");
            }
        }
    }

    if let Some((status, body, headers)) = last_failure {
        return raw_response(status, body, &headers);
    }
    error_response(StatusCode::BAD_GATEWAY, last_error)
}

fn add_request_headers(
    mut request: reqwest::RequestBuilder,
    profile: &Profile,
    incoming: &HeaderMap,
) -> reqwest::RequestBuilder {
    let mode = match profile.auth_mode {
        AuthMode::Auto if profile.protocol == WireProtocol::AnthropicMessages => AuthMode::XApiKey,
        AuthMode::Auto => AuthMode::Bearer,
        mode => mode,
    };
    request = match mode {
        AuthMode::XApiKey => request.header("x-api-key", &profile.api_key).header(
            "anthropic-version",
            incoming
                .get("anthropic-version")
                .cloned()
                .unwrap_or_else(|| HeaderValue::from_static("2023-06-01")),
        ),
        AuthMode::Bearer | AuthMode::Auto => request.bearer_auth(&profile.api_key),
    };
    for name in ["anthropic-beta", "openai-organization", "openai-project"] {
        if let Some(value) = incoming.get(name) {
            request = request.header(name, value);
        }
    }
    request
}

async fn process_response(
    response: reqwest::Response,
    upstream_protocol: WireProtocol,
    client_protocol: WireProtocol,
    streaming: bool,
) -> Response {
    let status = response.status();
    if !status.is_success() {
        let headers = response.headers().clone();
        let body = response.bytes().await.unwrap_or_default();
        return raw_response(status, body, &headers);
    }

    if upstream_protocol == client_protocol {
        let headers = response.headers().clone();
        if streaming {
            return streaming_passthrough(status, response, &headers);
        }
        let body = response.bytes().await.unwrap_or_default();
        return raw_response(status, body, &headers);
    }

    if streaming {
        return converted_stream(response, upstream_protocol, client_protocol);
    }

    let value: Value = match response.json().await {
        Ok(value) => value,
        Err(error) => {
            return error_response(
                StatusCode::BAD_GATEWAY,
                format!("upstream returned invalid JSON: {error}"),
            )
        }
    };
    match convert_response(value, upstream_protocol, client_protocol) {
        Ok(value) => json_response(status, value),
        Err(error) => error_response(
            StatusCode::BAD_GATEWAY,
            format!("response conversion failed: {error}"),
        ),
    }
}

fn streaming_passthrough(
    status: StatusCode,
    response: reqwest::Response,
    headers: &HeaderMap,
) -> Response {
    let mut builder = Response::builder().status(status);
    copy_response_headers(&mut builder, headers, false);
    builder
        .body(Body::from_stream(response.bytes_stream()))
        .unwrap_or_else(|_| {
            error_response(StatusCode::INTERNAL_SERVER_ERROR, "response build failed")
        })
}

fn converted_stream(
    response: reqwest::Response,
    upstream_protocol: WireProtocol,
    client_protocol: WireProtocol,
) -> Response {
    let Some(bridge) = StreamBridge::new(upstream_protocol, client_protocol) else {
        return error_response(StatusCode::BAD_GATEWAY, "unsupported streaming conversion");
    };
    let output = converted_sse_stream(response, bridge, upstream_protocol, client_protocol);
    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "text/event-stream")
        .header("cache-control", "no-cache")
        .body(Body::from_stream(output))
        .unwrap_or_else(|_| {
            error_response(StatusCode::INTERNAL_SERVER_ERROR, "response build failed")
        })
}

fn converted_sse_stream(
    response: reqwest::Response,
    mut bridge: StreamBridge,
    _upstream_protocol: WireProtocol,
    client_protocol: WireProtocol,
) -> impl futures_util::Stream<Item = Result<Bytes, io::Error>> {
    let mut upstream = response.bytes_stream().eventsource();
    try_stream! {
        while let Some(item) = upstream.next().await {
            let event = item.map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?;
            if event.data.trim().is_empty() || event.data.trim() == "[DONE]" {
                continue;
            }
            let value: Value = serde_json::from_str(&event.data)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?;
            let converted = bridge
                .transform(value)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?;
            for value in converted {
                yield encode_sse(client_protocol, &value)?;
            }
        }
        let flushed = bridge
            .flush()
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?;
        for value in flushed {
            yield encode_sse(client_protocol, &value)?;
        }
        if client_protocol == WireProtocol::OpenaiChat {
            yield Bytes::from_static(b"data: [DONE]\n\n");
        }
    }
}

fn encode_sse(protocol: WireProtocol, value: &Value) -> Result<Bytes, io::Error> {
    let json = serde_json::to_string(value)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?;
    match protocol {
        WireProtocol::OpenaiChat => Ok(Bytes::from(format!("data: {json}\n\n"))),
        WireProtocol::OpenaiResponses | WireProtocol::AnthropicMessages => {
            let event = value
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or("message");
            Ok(Bytes::from(format!("event: {event}\ndata: {json}\n\n")))
        }
    }
}

fn upstream_url(base_url: &str, protocol: WireProtocol) -> String {
    let base = base_url.trim().trim_end_matches('/');
    let endpoint = protocol.endpoint();
    if base.ends_with(endpoint) {
        base.to_string()
    } else {
        format!("{base}/{endpoint}")
    }
}

fn upstream_host(base_url: &str) -> String {
    let without_scheme = base_url
        .split_once("://")
        .map(|(_, value)| value)
        .unwrap_or(base_url);
    without_scheme
        .split(['/', ':'])
        .next()
        .unwrap_or("unknown-upstream")
        .to_string()
}

fn copy_response_headers(
    builder: &mut axum::http::response::Builder,
    headers: &HeaderMap,
    converted: bool,
) {
    for (name, value) in headers {
        if should_copy_header(name, converted) {
            if let Some(map) = builder.headers_mut() {
                map.insert(name.clone(), value.clone());
            }
        }
    }
}

fn should_copy_header(name: &HeaderName, converted: bool) -> bool {
    let name = name.as_str();
    !matches!(
        name,
        "content-length" | "transfer-encoding" | "connection" | "content-encoding"
    ) && !(converted && name == "content-type")
}

fn raw_response(status: StatusCode, body: Bytes, headers: &HeaderMap) -> Response {
    let mut builder = Response::builder().status(status);
    copy_response_headers(&mut builder, headers, false);
    builder.body(Body::from(body)).unwrap_or_else(|_| {
        error_response(StatusCode::INTERNAL_SERVER_ERROR, "response build failed")
    })
}

fn json_response(status: StatusCode, value: Value) -> Response {
    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(Body::from(value.to_string()))
        .expect("valid JSON response")
}

fn error_response(status: StatusCode, message: impl Into<String>) -> Response {
    json_response(
        status,
        json!({
            "error": {
                "message": message.into(),
                "type": "gateway_error"
            }
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_join_preserves_versioned_bases() {
        assert_eq!(
            upstream_url("https://example.test/v1", WireProtocol::OpenaiResponses),
            "https://example.test/v1/responses"
        );
        assert_eq!(
            upstream_url(
                "https://example.test/v1/messages",
                WireProtocol::AnthropicMessages
            ),
            "https://example.test/v1/messages"
        );
    }

    #[test]
    fn sse_uses_protocol_event_names() {
        let bytes = encode_sse(
            WireProtocol::OpenaiResponses,
            &json!({"type": "response.output_text.delta", "delta": "hi"}),
        )
        .unwrap();
        let text = String::from_utf8(bytes.to_vec()).unwrap();
        assert!(text.starts_with("event: response.output_text.delta\n"));
    }
}
