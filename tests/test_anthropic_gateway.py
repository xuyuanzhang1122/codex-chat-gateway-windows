from __future__ import annotations

import json
import os
import socket
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def free_ports(count: int) -> list[int]:
    listeners: list[socket.socket] = []
    try:
        for _ in range(count):
            listener = socket.socket()
            listener.bind(("127.0.0.1", 0))
            listeners.append(listener)
        return [int(listener.getsockname()[1]) for listener in listeners]
    finally:
        for listener in listeners:
            listener.close()


def request(url: str, payload: dict[str, object] | None = None) -> bytes:
    headers = {
        "content-type": "application/json",
        "x-api-key": "local-gateway",
        "anthropic-version": "2023-06-01",
    }
    body = json.dumps(payload).encode("utf-8") if payload is not None else None
    method = "POST" if payload is not None else "GET"
    with urllib.request.urlopen(
        urllib.request.Request(url, data=body, headers=headers, method=method), timeout=30
    ) as response:
        return response.read()


def log_tail(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8", errors="replace")[-4000:].strip()
    except OSError as exc:
        return f"Could not read {path}: {exc}"


def exited_process_error(name: str, process: subprocess.Popen[str], log_path: Path) -> str | None:
    return_code = process.poll()
    if return_code is None:
        return None
    return f"{name} exited with code {return_code}: {log_tail(log_path)}"


def wait_for_gateway(
    base: str,
    gateway: subprocess.Popen[str],
    upstreams: list[subprocess.Popen[str]],
    gateway_log_path: Path,
    upstream_log_paths: list[Path],
) -> dict[str, object]:
    deadline = time.monotonic() + 120
    last_error: Exception | None = None
    while time.monotonic() < deadline:
        processes = [("gateway", gateway, gateway_log_path)] + [
            (f"mock upstream {index + 1}", process, upstream_log_paths[index])
            for index, process in enumerate(upstreams)
        ]
        for name, process, log_path in processes:
            process_error = exited_process_error(name, process, log_path)
            if process_error is not None:
                raise RuntimeError(process_error)
        try:
            return json.loads(request(f"{base}/v1/models"))
        except (OSError, urllib.error.URLError, json.JSONDecodeError) as exc:
            last_error = exc
            time.sleep(0.25)
    raise RuntimeError(
        f"Gateway did not become ready after 120 seconds: {last_error}; "
        f"gateway_running={gateway.poll() is None}; "
        f"upstreams_running={[process.poll() is None for process in upstreams]}; "
        f"gateway_log={log_tail(gateway_log_path)}; "
        f"upstream_logs={[log_tail(path) for path in upstream_log_paths]}"
    )


def main() -> None:
    upstream_port, second_upstream_port, gateway_port = free_ports(3)
    env = os.environ.copy()
    env.pop("PYTHONUTF8", None)
    env.update(
        {
            "PYTHONIOENCODING": "cp1252",
            "UPSTREAM_MODEL": "openai/mock-model",
            "CLAUDE_UPSTREAM_MODEL": "custom_openai/mock-model",
            "UPSTREAM_BASE_URL": f"http://127.0.0.1:{upstream_port}/v1",
            "UPSTREAM_API_KEY": "test-key",
            "LITELLM_LOCAL_MODEL_COST_MAP": "True",
        }
    )
    startup = subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0
    temp_directory = tempfile.TemporaryDirectory(ignore_cleanup_errors=True)
    log_root = Path(temp_directory.name)
    upstream_log_path = log_root / "upstream-a.stderr.log"
    second_upstream_log_path = log_root / "upstream-b.stderr.log"
    gateway_log_path = log_root / "gateway.stderr.log"
    upstream_log = upstream_log_path.open("w", encoding="utf-8", errors="replace")
    second_upstream_log = second_upstream_log_path.open("w", encoding="utf-8", errors="replace")
    gateway_log = gateway_log_path.open("w", encoding="utf-8", errors="replace")
    models_path = log_root / "models.json"
    runtime_config_path = log_root / "runtime-config.yaml"
    routing_traffic_path = log_root / "routing-traffic.json"
    profile = {
        "id": "mock-account",
        "name": "Mock A",
        "base_url": f"http://127.0.0.1:{upstream_port}/v1",
        "api_key": "test-key-a",
        "model_id": "mock-model",
        "litellm_model": "openai/mock-model",
        "routing_enabled": True,
        "routing_weight": 1,
    }
    models_path.write_text(
        json.dumps(
            {
                "version": 3,
                "default_id": profile["id"],
                "profiles": [profile],
                "routing": {
                    "enabled": False,
                    "affinity_ttl_seconds": 3600,
                    "model_rules": [],
                },
            }
        ),
        encoding="utf-8",
    )
    env.update(
        {
            "CCG_MODELS_PATH": str(models_path),
            "CCG_RUNTIME_CONFIG_PATH": str(runtime_config_path),
            "CCG_ROUTING_TRAFFIC_PATH": str(routing_traffic_path),
        }
    )
    upstream = subprocess.Popen(
        [
            sys.executable,
            str(ROOT / "tests" / "mock_chat_upstream.py"),
            "--port",
            str(upstream_port),
            "--response-text",
            "mock response a",
            "--stream-text",
            "mock stream a",
        ],
        cwd=ROOT,
        env=env,
        stdout=subprocess.DEVNULL,
        stderr=upstream_log,
        creationflags=startup,
    )
    second_upstream = subprocess.Popen(
        [
            sys.executable,
            str(ROOT / "tests" / "mock_chat_upstream.py"),
            "--port",
            str(second_upstream_port),
            "--response-text",
            "mock response b",
            "--stream-text",
            "mock stream b",
        ],
        cwd=ROOT,
        env=env,
        stdout=subprocess.DEVNULL,
        stderr=second_upstream_log,
        creationflags=startup,
    )
    gateway_started_at = time.monotonic()
    gateway = subprocess.Popen(
        [
            sys.executable,
            str(ROOT / "run_gateway.py"),
            "--config",
            str(ROOT / "config.yaml"),
            "--host",
            "127.0.0.1",
            "--port",
            str(gateway_port),
        ],
        cwd=ROOT,
        env=env,
        stdout=subprocess.DEVNULL,
        stderr=gateway_log,
        creationflags=startup,
    )
    try:
        base = f"http://127.0.0.1:{gateway_port}"
        models = wait_for_gateway(
            base,
            gateway,
            [upstream, second_upstream],
            gateway_log_path,
            [upstream_log_path, second_upstream_log_path],
        )
        print(f"GATEWAY_READY_SECONDS={time.monotonic() - gateway_started_at:.2f}")
        ids = {item["id"] for item in models["data"]}
        required = {"codex-chat", "claude-sonnet-5", "claude-opus-4-8", "claude-haiku-4-5"}
        assert required <= ids

        responses_basic = json.loads(
            request(
                f"{base}/v1/responses",
                {
                    "model": "codex-chat",
                    "input": "ping",
                    "max_output_tokens": 32,
                },
            )
        )
        assert responses_basic["object"] == "response"
        assert responses_basic["output"][0]["content"][0]["text"] == "mock response a"

        responses_tool = json.loads(
            request(
                f"{base}/v1/responses",
                {
                    "model": "codex-chat",
                    "input": "use the tool",
                    "max_output_tokens": 32,
                    "tools": [
                        {
                            "type": "function",
                            "name": "echo",
                            "description": "Echo text",
                            "parameters": {
                                "type": "object",
                                "properties": {"text": {"type": "string"}},
                                "required": ["text"],
                            },
                        }
                    ],
                },
            )
        )
        function_calls = [
            item for item in responses_tool["output"] if item["type"] == "function_call"
        ]
        assert len(function_calls) == 1, responses_tool
        assert function_calls[0]["name"] == "echo", responses_tool

        responses_stream = request(
            f"{base}/v1/responses",
            {
                "model": "codex-chat",
                "input": "stream",
                "max_output_tokens": 32,
                "stream": True,
            },
        ).decode("utf-8")
        assert "response.output_text.delta" in responses_stream
        assert "mock stream a" in responses_stream

        basic = json.loads(
            request(
                f"{base}/v1/messages",
                {
                    "model": "claude-sonnet-5",
                    "max_tokens": 32,
                    "messages": [{"role": "user", "content": "ping"}],
                },
            )
        )
        assert basic["type"] == "message"
        assert basic["content"] == [{"type": "text", "text": "mock response a"}]

        tool = json.loads(
            request(
                f"{base}/v1/messages",
                {
                    "model": "claude-sonnet-5",
                    "max_tokens": 32,
                    "messages": [{"role": "user", "content": "use the tool"}],
                    "tools": [
                        {
                            "name": "echo",
                            "description": "Echo text",
                            "input_schema": {
                                "type": "object",
                                "properties": {"text": {"type": "string"}},
                                "required": ["text"],
                            },
                        }
                    ],
                },
            )
        )
        assert tool["stop_reason"] == "tool_use"
        assert tool["content"][0]["type"] == "tool_use"
        assert tool["content"][0]["name"] == "echo"

        stream = request(
            f"{base}/v1/messages",
            {
                "model": "claude-sonnet-5",
                "max_tokens": 32,
                "stream": True,
                "messages": [{"role": "user", "content": "stream"}],
            },
        ).decode("utf-8")
        assert "content_block_delta" in stream
        assert "mock stream a" in stream

        # Change only the gateway's upstream. The public aliases and listening
        # socket stay in place while the running LiteLLM Router is updated.
        profile["name"] = "Mock B"
        profile["base_url"] = f"http://127.0.0.1:{second_upstream_port}/v1"
        profile["api_key"] = "test-key-b"
        models_path.write_text(
            json.dumps(
                {
                    "version": 3,
                    "default_id": profile["id"],
                    "profiles": [profile],
                    "routing": {
                        "enabled": False,
                        "affinity_ttl_seconds": 3600,
                        "model_rules": [],
                    },
                }
            ),
            encoding="utf-8",
        )
        reload_result = json.loads(request(f"{base}/internal/ccg/reload", {}))
        assert reload_result["ok"] is True
        assert reload_result["runtime_config_persisted"] is True
        assert "codex-chat" in reload_result["routes"]
        assert gateway.poll() is None

        switched = json.loads(
            request(
                f"{base}/v1/responses",
                {
                    "model": "codex-chat",
                    "input": "after reload",
                    "max_output_tokens": 32,
                },
            )
        )
        assert switched["output"][0]["content"][0]["text"] == "mock response b"
    except Exception:
        gateway_log.flush()
        upstream_log.flush()
        second_upstream_log.flush()
        print(f"GATEWAY_LOG:\n{log_tail(gateway_log_path)}", file=sys.stderr)
        print(f"UPSTREAM_LOG:\n{log_tail(upstream_log_path)}", file=sys.stderr)
        print(f"SECOND_UPSTREAM_LOG:\n{log_tail(second_upstream_log_path)}", file=sys.stderr)
        raise
    finally:
        for process in (gateway, upstream, second_upstream):
            if process.poll() is None:
                process.terminate()
        for process in (gateway, upstream, second_upstream):
            try:
                process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=5)
        gateway_log.close()
        upstream_log.close()
        second_upstream_log.close()
        temp_directory.cleanup()


if __name__ == "__main__":
    main()
    print("ANTHROPIC_GATEWAY_OK")
