from __future__ import annotations

import copy
import hashlib
import json
import os
import sys
import threading
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Any
from urllib.parse import urlsplit

import yaml


PROJECT_ROOT = Path(__file__).resolve().parent
MODELS_PATH = PROJECT_ROOT / ".gateway" / "models.json"
RUNTIME_CONFIG_PATH = PROJECT_ROOT / ".gateway" / "runtime-config.yaml"
ROUTING_TRAFFIC_PATH = PROJECT_ROOT / ".gateway" / "routing-traffic.json"
_AFFINITY_SALT = os.urandom(32)
_TELEMETRY_LOCK = threading.Lock()
_RECENT_TELEMETRY_CALLS: dict[str, float] = {}


def _prompt_cache_affinity_id(prompt_cache_key: Any) -> str:
    digest = hashlib.sha256(
        _AFFINITY_SALT + str(prompt_cache_key).encode("utf-8")
    ).hexdigest()
    return f"ccg-cache-{digest}"


def _normalized_model_id(value: str) -> str:
    value = value.strip().lower()
    if "/" in value:
        provider, rest = value.split("/", 1)
        if provider in {"openai", "custom_openai", "deepseek"}:
            return rest
    return value


def _claude_litellm_model(value: str) -> str:
    if value.lower().startswith("openai/"):
        return f"custom_openai/{value.split('/', 1)[1]}"
    return value


def _model_routing_enabled(routing: dict[str, Any], model_id: str) -> bool:
    rules = routing.get("model_rules") or []
    if rules:
        normalized = _normalized_model_id(model_id)
        rule = next(
            (
                item
                for item in rules
                if _normalized_model_id(str(item.get("model_id") or "")) == normalized
            ),
            None,
        )
        return bool(rule and rule.get("enabled", False))
    return bool(routing.get("enabled", False))


def _read_routing_pool() -> tuple[dict[str, Any], list[dict[str, Any]]] | None:
    if os.environ.get("CCG_DISABLE_MULTI_ACCOUNT_ROUTING") == "1":
        return None
    if not MODELS_PATH.exists():
        return None

    import json

    with MODELS_PATH.open("r", encoding="utf-8") as handle:
        store = json.load(handle)

    routing = store.get("routing") or {}
    profiles = store.get("profiles") or []
    if not profiles:
        return None

    default_id = store.get("default_id") or ""
    default = next((item for item in profiles if item.get("id") == default_id), profiles[0])
    target_model = _normalized_model_id(str(default.get("model_id") or ""))
    if not _model_routing_enabled(routing, target_model):
        return None
    pool = [
        item
        for item in profiles
        if item.get("routing_enabled", True)
        and _normalized_model_id(str(item.get("model_id") or "")) == target_model
    ]
    if not pool:
        # A malformed/manual edit must not make an otherwise valid gateway unstartable.
        pool = [default]
    return routing, pool


def _config_argument() -> tuple[int | None, Path]:
    for index, value in enumerate(sys.argv[:-1]):
        if value == "--config":
            return index + 1, Path(sys.argv[index + 1]).resolve()
        if value.startswith("--config="):
            return index, Path(value.split("=", 1)[1]).resolve()
    return None, PROJECT_ROOT / "config.yaml"


def _deployment_id(profile_id: str, model_name: str) -> str:
    suffix = hashlib.sha256(model_name.encode("utf-8")).hexdigest()[:10]
    return f"ccg-{profile_id[:16]}-{suffix}"


def _routing_metadata(kwargs: dict[str, Any]) -> tuple[dict[str, Any], dict[str, Any]]:
    litellm_params = kwargs.get("litellm_params")
    if not isinstance(litellm_params, dict):
        litellm_params = {}
    metadata = kwargs.get("metadata") or litellm_params.get("metadata") or {}
    if not isinstance(metadata, dict):
        metadata = {}
    model_info = (
        kwargs.get("model_info")
        or metadata.get("model_info")
        or litellm_params.get("model_info")
        or {}
    )
    if not isinstance(model_info, dict):
        model_info = {}
    return metadata, model_info


def _record_routing_event(
    kwargs: dict[str, Any], traffic_path: Path | None = None
) -> bool:
    """Persist one privacy-safe model -> upstream hit.

    The callback intentionally stores no prompt, response, token, key, headers, or
    request identifier. Repeated callback delivery for the same call is deduplicated.
    """

    metadata, model_info = _routing_metadata(kwargs)
    profile_id = str(model_info.get("ccg_profile_id") or "").strip()
    model_id = str(model_info.get("ccg_model_id") or "").strip()
    if not profile_id or not model_id:
        return False

    profile_name = str(model_info.get("ccg_profile_name") or profile_id).strip()
    api_base = str(metadata.get("api_base") or kwargs.get("api_base") or "").strip()
    upstream_host = str(model_info.get("ccg_upstream_host") or "").strip()
    if not upstream_host and api_base:
        upstream_host = urlsplit(api_base).hostname or ""
    if not upstream_host:
        upstream_host = "unknown-upstream"

    call_id = str(
        kwargs.get("litellm_call_id")
        or kwargs.get("litellm_trace_id")
        or metadata.get("litellm_call_id")
        or ""
    )
    route_key = f"{_normalized_model_id(model_id)}::{profile_id}"
    dedupe_key = f"{route_key}::{call_id}" if call_id else ""
    path = traffic_path or ROUTING_TRAFFIC_PATH
    now_epoch = time.time()
    now_text = datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")

    with _TELEMETRY_LOCK:
        expired = [key for key, seen in _RECENT_TELEMETRY_CALLS.items() if now_epoch - seen > 120]
        for key in expired:
            _RECENT_TELEMETRY_CALLS.pop(key, None)
        if dedupe_key and dedupe_key in _RECENT_TELEMETRY_CALLS:
            return False
        if dedupe_key:
            _RECENT_TELEMETRY_CALLS[dedupe_key] = now_epoch

        store: dict[str, Any] = {"version": 1, "routes": []}
        try:
            if path.exists():
                loaded = json.loads(path.read_text(encoding="utf-8"))
                if isinstance(loaded, dict) and isinstance(loaded.get("routes"), list):
                    store = loaded
        except (OSError, ValueError):
            # Telemetry must never interfere with an upstream request.
            store = {"version": 1, "routes": []}

        routes = store.setdefault("routes", [])
        existing = next(
            (
                item
                for item in routes
                if isinstance(item, dict)
                and _normalized_model_id(str(item.get("model_id") or ""))
                == _normalized_model_id(model_id)
                and str(item.get("profile_id") or "") == profile_id
            ),
            None,
        )
        if existing is None:
            routes.append(
                {
                    "model_id": model_id,
                    "profile_id": profile_id,
                    "profile_name": profile_name,
                    "upstream_host": upstream_host,
                    "hit_count": 1,
                    "first_seen_at": now_text,
                    "last_seen_at": now_text,
                }
            )
        else:
            existing["profile_name"] = profile_name
            existing["upstream_host"] = upstream_host
            existing["hit_count"] = max(0, int(existing.get("hit_count") or 0)) + 1
            existing["last_seen_at"] = now_text

        store["version"] = 1
        path.parent.mkdir(parents=True, exist_ok=True)
        temp_path = path.with_suffix(".json.tmp")
        try:
            temp_path.write_text(
                json.dumps(store, ensure_ascii=False, indent=2) + "\n",
                encoding="utf-8",
            )
            os.replace(temp_path, path)
        except OSError:
            return False
    return True


def install_routing_telemetry() -> None:
    """Attach a LiteLLM callback after deployment selection and before the API call."""

    import litellm
    from litellm.integrations.custom_logger import CustomLogger

    if getattr(litellm, "_ccg_routing_telemetry", False):
        return

    class RoutingTelemetryCallback(CustomLogger):
        def log_pre_api_call(self, model: str, messages: Any, kwargs: dict[str, Any]):
            _record_routing_event(kwargs)

        async def async_log_pre_api_call(
            self, model: str, messages: Any, kwargs: dict[str, Any]
        ):
            _record_routing_event(kwargs)

    callback = RoutingTelemetryCallback(turn_off_message_logging=True)
    litellm.logging_callback_manager.add_litellm_callback(callback)
    litellm._ccg_routing_telemetry = True
    litellm._ccg_routing_telemetry_callback = callback


def _build_runtime_config(
    base_config: dict[str, Any], routing: dict[str, Any], pool: list[dict[str, Any]]
) -> dict[str, Any]:
    templates = base_config.get("model_list") or []
    if not templates:
        raise ValueError("config.yaml 中没有 model_list")

    deployments: list[dict[str, Any]] = []
    for profile_index, profile in enumerate(pool):
        prefix = f"CCG_ROUTE_{profile_index}"
        litellm_model = str(profile.get("litellm_model") or "")
        if not litellm_model:
            raise ValueError(f"模型配置 {profile.get('name') or profile_index} 缺少 litellm_model")

        os.environ[f"{prefix}_MODEL"] = litellm_model
        os.environ[f"{prefix}_CLAUDE_MODEL"] = _claude_litellm_model(litellm_model)
        os.environ[f"{prefix}_BASE_URL"] = str(profile.get("base_url") or "")
        os.environ[f"{prefix}_API_KEY"] = str(profile.get("api_key") or "")

        for template in templates:
            deployment = copy.deepcopy(template)
            model_name = str(deployment.get("model_name") or "")
            params = deployment.setdefault("litellm_params", {})
            is_codex_route = model_name == "codex-chat" or bool(params.get("use_chat_completions_api"))
            model_env = f"{prefix}_MODEL" if is_codex_route else f"{prefix}_CLAUDE_MODEL"
            params["model"] = f"os.environ/{model_env}"
            params["api_base"] = f"os.environ/{prefix}_BASE_URL"
            params["api_key"] = f"os.environ/{prefix}_API_KEY"
            params["weight"] = max(1, min(100, int(profile.get("routing_weight", 1))))
            model_info = deployment.setdefault("model_info", {})
            model_info["id"] = _deployment_id(str(profile.get("id") or profile_index), model_name)
            model_info["ccg_profile_id"] = str(profile.get("id") or profile_index)
            model_info["ccg_profile_name"] = str(profile.get("name") or profile_index)
            model_info["ccg_model_id"] = str(profile.get("model_id") or litellm_model)
            model_info["ccg_upstream_host"] = (
                urlsplit(str(profile.get("base_url") or "")).hostname or "unknown-upstream"
            )
            deployments.append(deployment)

    runtime = copy.deepcopy(base_config)
    runtime["model_list"] = deployments
    router_settings = runtime.setdefault("router_settings", {})
    router_settings.update(
        {
            "routing_strategy": "simple-shuffle",
            "enable_pre_call_checks": True,
            "optional_pre_call_checks": [
                "responses_api_deployment_check",
                "session_affinity",
            ],
            "deployment_affinity_ttl_seconds": max(
                300, min(86_400, int(routing.get("affinity_ttl_seconds", 3600)))
            ),
            "enable_weighted_failover": True,
            "max_fallbacks": max(0, len(pool) - 1),
            "allowed_fails": 1,
            "cooldown_time": 30,
        }
    )
    return runtime


def prepare_gateway_runtime() -> Path | None:
    pool_result = _read_routing_pool()
    if pool_result is None:
        return None

    routing, pool = pool_result
    argument_index, base_path = _config_argument()
    with base_path.open("r", encoding="utf-8") as handle:
        base_config = yaml.safe_load(handle) or {}
    runtime = _build_runtime_config(base_config, routing, pool)

    RUNTIME_CONFIG_PATH.parent.mkdir(parents=True, exist_ok=True)
    temp_path = RUNTIME_CONFIG_PATH.with_suffix(".yaml.tmp")
    with temp_path.open("w", encoding="utf-8", newline="\n") as handle:
        yaml.safe_dump(runtime, handle, allow_unicode=True, sort_keys=False)
    os.replace(temp_path, RUNTIME_CONFIG_PATH)

    if argument_index is None:
        sys.argv.extend(["--config", str(RUNTIME_CONFIG_PATH)])
    elif sys.argv[argument_index].startswith("--config="):
        sys.argv[argument_index] = f"--config={RUNTIME_CONFIG_PATH}"
    else:
        sys.argv[argument_index] = str(RUNTIME_CONFIG_PATH)
    return RUNTIME_CONFIG_PATH


def install_prompt_cache_affinity() -> None:
    """Use prompt_cache_key as a local affinity hint without forwarding it upstream.

    Codex sends this Responses API field to keep reusable prefixes together. LiteLLM's
    Responses-to-Chat bridge currently omits it, so convert it to a salted session id.
    The raw cache key never enters logs, generated config, or provider metadata.
    """

    from litellm.responses.litellm_completion_transformation.transformation import (
        LiteLLMCompletionResponsesConfig,
    )
    from litellm.router_utils.pre_call_checks.deployment_affinity_check import (
        DeploymentAffinityCheck,
    )

    if getattr(LiteLLMCompletionResponsesConfig, "_ccg_prompt_cache_affinity", False):
        return

    original_supported = LiteLLMCompletionResponsesConfig.get_supported_openai_params
    original_transform = (
        LiteLLMCompletionResponsesConfig.transform_responses_api_request_to_chat_completion_request
    )
    original_session_id = DeploymentAffinityCheck._get_session_id_from_request_kwargs

    def supported(model: str) -> list:
        values = list(original_supported(model))
        if "prompt_cache_key" not in values:
            values.append("prompt_cache_key")
        return values

    def transform(
        model: str,
        input: Any,
        responses_api_request: Any,
        custom_llm_provider: str | None = None,
        stream: bool | None = None,
        extra_headers: dict[str, Any] | None = None,
        **kwargs: Any,
    ) -> dict[str, Any]:
        request = dict(responses_api_request)
        request.pop("prompt_cache_key", None)
        return original_transform(
            model=model,
            input=input,
            responses_api_request=request,
            custom_llm_provider=custom_llm_provider,
            stream=stream,
            extra_headers=extra_headers,
            **kwargs,
        )

    def session_id(request_kwargs: dict[str, Any]) -> str | None:
        existing = original_session_id(request_kwargs)
        if existing is not None:
            return existing
        prompt_cache_key = request_kwargs.get("prompt_cache_key")
        if prompt_cache_key is None:
            nested = request_kwargs.get("kwargs")
            if isinstance(nested, dict):
                prompt_cache_key = nested.get("prompt_cache_key")
        if prompt_cache_key is None:
            return None
        return _prompt_cache_affinity_id(prompt_cache_key)

    LiteLLMCompletionResponsesConfig.get_supported_openai_params = staticmethod(supported)
    LiteLLMCompletionResponsesConfig.transform_responses_api_request_to_chat_completion_request = staticmethod(
        transform
    )
    DeploymentAffinityCheck._get_session_id_from_request_kwargs = staticmethod(session_id)
    LiteLLMCompletionResponsesConfig._ccg_prompt_cache_affinity = True
