from __future__ import annotations

import json
import os
import sys
import tempfile
import copy
from pathlib import Path
from types import SimpleNamespace

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
    assert codex[0]["model_info"]["ccg_profile_id"] == "account-a"
    assert codex[0]["model_info"]["ccg_model_id"] == "gpt-5.6-sol"
    assert codex[0]["model_info"]["ccg_upstream_host"] == "account-a.example"
    assert os.environ["CCG_ROUTE_1_CLAUDE_MODEL"] == "custom_openai/gpt-5.6-sol"
    assert runtime["router_settings"]["max_fallbacks"] == 1
    assert runtime["router_settings"]["deployment_affinity_ttl_seconds"] == 7200
    serialized = yaml.safe_dump(runtime)
    assert "sk-secret-a" not in serialized
    assert "sk-secret-b" not in serialized

    class FakeRouter:
        def __init__(self, model_list: list[dict]) -> None:
            self.model_list = copy.deepcopy(model_list)
            self.max_fallbacks = 0
            self.enable_weighted_failover = False
            self.allowed_fails = 0
            self.deployment_affinity_ttl_seconds = 300

        def get_model_list(self) -> list[dict]:
            return copy.deepcopy(self.model_list)

        def get_model_names(self) -> list[str]:
            return [item["model_name"] for item in self.model_list]

        def set_model_list(self, model_list: list[dict]) -> None:
            self.model_list = copy.deepcopy(model_list)

    fake_router = FakeRouter(BASE_CONFIG["model_list"])
    hot_reload = gateway_runtime._apply_runtime_to_router(fake_router, runtime)
    assert hot_reload["deployments"] == 4
    assert hot_reload["routes"] == ["claude-sonnet-5", "codex-chat"]
    assert fake_router.max_fallbacks == 1
    assert fake_router.enable_weighted_failover is True
    assert fake_router.deployment_affinity_ttl_seconds == 7200

    class FailOnceRouter(FakeRouter):
        def __init__(self, model_list: list[dict]) -> None:
            super().__init__(model_list)
            self.fail_next = True

        def set_model_list(self, model_list: list[dict]) -> None:
            if self.fail_next:
                self.fail_next = False
                self.model_list = []
                raise ValueError("synthetic reload failure")
            super().set_model_list(model_list)

    fail_router = FailOnceRouter(BASE_CONFIG["model_list"])
    original_models = fail_router.get_model_list()
    try:
        gateway_runtime._apply_runtime_to_router(fail_router, runtime)
        raise AssertionError("reload failure should propagate")
    except ValueError as exc:
        assert "synthetic" in str(exc)
    assert fail_router.get_model_list() == original_models

    with tempfile.TemporaryDirectory() as temporary:
        temporary_path = Path(temporary)
        models_path = temporary_path / "models.json"
        runtime_path = temporary_path / "runtime-config.yaml"
        base_path = temporary_path / "config.yaml"
        base_path.write_text(yaml.safe_dump(BASE_CONFIG), encoding="utf-8")
        models_path.write_text(
            json.dumps(
                {
                    "version": 3,
                    "default_id": "account-a",
                    "profiles": [first],
                    "routing": {
                        "enabled": False,
                        "affinity_ttl_seconds": 3600,
                        "model_rules": [],
                    },
                }
            ),
            encoding="utf-8",
        )
        old_models_path = gateway_runtime.MODELS_PATH
        old_runtime_path = gateway_runtime.RUNTIME_CONFIG_PATH
        old_base_path = gateway_runtime._BASE_CONFIG_PATH
        old_route_environment = gateway_runtime._route_environment_snapshot()
        try:
            gateway_runtime.MODELS_PATH = models_path
            gateway_runtime.RUNTIME_CONFIG_PATH = runtime_path
            gateway_runtime._BASE_CONFIG_PATH = base_path
            os.environ["CCG_ROUTE_9_API_KEY"] = "stale-secret"

            live_router = FakeRouter(BASE_CONFIG["model_list"])
            proxy_server = SimpleNamespace(llm_router=live_router, llm_model_list=[])
            applied = gateway_runtime.reload_live_config(proxy_server)
            assert applied["ok"] is True
            assert applied["runtime_config_persisted"] is True
            assert "CCG_ROUTE_9_API_KEY" not in os.environ
            assert os.environ["CCG_ROUTE_0_API_KEY"] == "sk-secret-a"

            route_environment_before_failure = gateway_runtime._route_environment_snapshot()
            models_path.write_text(
                json.dumps(
                    {
                        "version": 3,
                        "default_id": "account-b",
                        "profiles": [second, first],
                        "routing": {
                            "enabled": True,
                            "affinity_ttl_seconds": 3600,
                            "model_rules": [],
                        },
                    }
                ),
                encoding="utf-8",
            )
            failing_live_router = FailOnceRouter(live_router.get_model_list())
            failing_proxy = SimpleNamespace(
                llm_router=failing_live_router,
                llm_model_list=live_router.get_model_list(),
            )
            try:
                gateway_runtime.reload_live_config(failing_proxy)
                raise AssertionError("failed live reload should propagate")
            except ValueError as exc:
                assert "synthetic" in str(exc)
            assert (
                gateway_runtime._route_environment_snapshot()
                == route_environment_before_failure
            )
        finally:
            gateway_runtime.MODELS_PATH = old_models_path
            gateway_runtime.RUNTIME_CONFIG_PATH = old_runtime_path
            gateway_runtime._BASE_CONFIG_PATH = old_base_path
            gateway_runtime._restore_route_environment(old_route_environment)

    gateway_runtime.install_live_config_reload()
    gateway_runtime.install_live_config_reload()
    from litellm.proxy import proxy_server

    assert sum(
        1
        for route in proxy_server.app.routes
        if getattr(route, "path", None) == gateway_runtime.LIVE_RELOAD_PATH
    ) == 1

    with tempfile.TemporaryDirectory() as temporary:
        models_path = Path(temporary) / "models.json"
        old_path = gateway_runtime.MODELS_PATH
        gateway_runtime.MODELS_PATH = models_path
        try:
            os.environ["CCG_DISABLE_MULTI_ACCOUNT_ROUTING"] = "1"
            assert gateway_runtime._read_routing_pool() is None
            assert gateway_runtime._read_runtime_pool() is None
            os.environ.pop("CCG_DISABLE_MULTI_ACCOUNT_ROUTING")

            # A v1 store has no global opt-in, so an upgrade keeps single-account behavior.
            models_path.write_text(
                json.dumps({"version": 1, "default_id": "account-a", "profiles": [first]}),
                encoding="utf-8",
            )
            assert gateway_runtime._read_routing_pool() is None
            single = gateway_runtime._read_runtime_pool()
            assert single is not None
            assert single[0]["enabled"] is False
            assert [item["id"] for item in single[1]] == ["account-a"]

            annotated = gateway_runtime._build_runtime_config(
                BASE_CONFIG,
                single[0],
                single[1],
            )
            annotated_codex = next(
                item for item in annotated["model_list"] if item["model_name"] == "codex-chat"
            )
            assert annotated_codex["model_info"]["ccg_profile_id"] == "account-a"
            assert annotated["router_settings"]["max_fallbacks"] == 0

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
            os.environ.pop("CCG_DISABLE_MULTI_ACCOUNT_ROUTING", None)
            gateway_runtime.MODELS_PATH = old_path

    with tempfile.TemporaryDirectory() as temporary:
        traffic_path = Path(temporary) / "routing-traffic.json"
        callback_kwargs = {
            "litellm_call_id": "call-1",
            "metadata": {
                "api_base": "https://account-a.example/v1",
                "model_info": {
                    "ccg_profile_id": "account-a",
                    "ccg_profile_name": "Account A",
                    "ccg_model_id": "gpt-5.6-sol",
                    "ccg_upstream_host": "account-a.example",
                },
            },
        }
        assert gateway_runtime._record_routing_event(callback_kwargs, traffic_path)
        # Duplicate callback delivery for one LiteLLM call must not double count.
        assert not gateway_runtime._record_routing_event(callback_kwargs, traffic_path)
        callback_kwargs["litellm_call_id"] = "call-2"
        assert gateway_runtime._record_routing_event(callback_kwargs, traffic_path)
        traffic = json.loads(traffic_path.read_text(encoding="utf-8"))
        assert traffic["routes"][0]["hit_count"] == 2
        serialized_traffic = json.dumps(traffic)
        assert "sk-secret" not in serialized_traffic
        assert "messages" not in serialized_traffic

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

    gateway_runtime.install_routing_telemetry()
    import litellm

    assert any(
        type(callback).__name__ == "RoutingTelemetryCallback"
        for callback in litellm.callbacks
    )

    print("GATEWAY_ROUTING_OK")


if __name__ == "__main__":
    main()
