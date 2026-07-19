from __future__ import annotations

import json
import os
import sys
import tempfile
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

import gateway_runtime


BASE_CONFIG = {
    "model_list": [
        {
            "model_name": "codex-chat",
            "litellm_params": {
                "model": "os.environ/UPSTREAM_MODEL",
                "api_base": "os.environ/UPSTREAM_BASE_URL",
                "api_key": "os.environ/UPSTREAM_API_KEY",
                "use_chat_completions_api": True,
            },
        },
        {
            "model_name": "claude-sonnet-5",
            "litellm_params": {
                "model": "os.environ/CLAUDE_UPSTREAM_MODEL",
                "api_base": "os.environ/UPSTREAM_BASE_URL",
                "api_key": "os.environ/UPSTREAM_API_KEY",
            },
        },
    ],
    "litellm_settings": {"drop_params": True},
}


def profile(profile_id: str, key: str, weight: int) -> dict:
    return {
        "id": profile_id,
        "name": profile_id,
        "base_url": f"https://{profile_id}.example/v1",
        "api_key": key,
        "model_id": "gpt-5.6-sol",
        "litellm_model": "openai/gpt-5.6-sol",
        "routing_enabled": True,
        "routing_weight": weight,
    }


def main() -> None:
    first = profile("account-a", "sk-secret-a", 3)
    second = profile("account-b", "sk-secret-b", 1)
    runtime = gateway_runtime._build_runtime_config(
        BASE_CONFIG,
        {"enabled": True, "affinity_ttl_seconds": 7200},
        [first, second],
    )
    assert len(runtime["model_list"]) == 4
    codex = [item for item in runtime["model_list"] if item["model_name"] == "codex-chat"]
    assert [item["litellm_params"]["weight"] for item in codex] == [3, 1]
    assert codex[0]["litellm_params"]["model"] == "os.environ/CCG_ROUTE_0_MODEL"
    assert os.environ["CCG_ROUTE_1_CLAUDE_MODEL"] == "custom_openai/gpt-5.6-sol"
    assert runtime["router_settings"]["max_fallbacks"] == 1
    assert runtime["router_settings"]["deployment_affinity_ttl_seconds"] == 7200
    serialized = yaml.safe_dump(runtime)
    assert "sk-secret-a" not in serialized
    assert "sk-secret-b" not in serialized

    with tempfile.TemporaryDirectory() as temporary:
        models_path = Path(temporary) / "models.json"
        old_path = gateway_runtime.MODELS_PATH
        gateway_runtime.MODELS_PATH = models_path
        try:
            # A v1 store has no global opt-in, so an upgrade keeps single-account behavior.
            models_path.write_text(
                json.dumps({"version": 1, "default_id": "account-a", "profiles": [first]}),
                encoding="utf-8",
            )
            assert gateway_runtime._read_routing_pool() is None

            models_path.write_text(
                json.dumps(
                    {
                        "version": 2,
                        "default_id": "account-a",
                        "profiles": [
                            first,
                            second,
                            {**profile("other-model", "sk-other", 9), "model_id": "gpt-5.4"},
                        ],
                        "routing": {"enabled": True, "affinity_ttl_seconds": 3600},
                    }
                ),
                encoding="utf-8",
            )
            result = gateway_runtime._read_routing_pool()
            assert result is not None
            assert [item["id"] for item in result[1]] == ["account-a", "account-b"]

            models_path.write_text(
                json.dumps(
                    {
                        "version": 3,
                        "default_id": "account-a",
                        "profiles": [first, second],
                        "routing": {
                            "enabled": True,
                            "affinity_ttl_seconds": 3600,
                            "model_rules": [
                                {"model_id": "gpt-5.6-sol", "enabled": False}
                            ],
                        },
                    }
                ),
                encoding="utf-8",
            )
            assert gateway_runtime._read_routing_pool() is None
        finally:
            gateway_runtime.MODELS_PATH = old_path

    gateway_runtime.install_prompt_cache_affinity()
    from litellm.responses.litellm_completion_transformation.transformation import (
        LiteLLMCompletionResponsesConfig,
    )
    from litellm.router_utils.pre_call_checks.deployment_affinity_check import (
        DeploymentAffinityCheck,
    )

    assert "prompt_cache_key" in LiteLLMCompletionResponsesConfig.get_supported_openai_params("x")
    transformed = LiteLLMCompletionResponsesConfig.transform_responses_api_request_to_chat_completion_request(
        model="openai/gpt-5.6-sol",
        input="hello",
        responses_api_request={"prompt_cache_key": "do-not-forward"},
    )
    affinity = DeploymentAffinityCheck._get_session_id_from_request_kwargs(
        {"prompt_cache_key": "do-not-forward"}
    )
    assert affinity is not None and affinity.startswith("ccg-cache-")
    assert "metadata" not in transformed
    assert "do-not-forward" not in json.dumps(transformed)

    print("GATEWAY_ROUTING_OK")


if __name__ == "__main__":
    main()
