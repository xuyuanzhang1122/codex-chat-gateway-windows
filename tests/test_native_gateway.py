from __future__ import annotations

import json
import os
import socket
import subprocess
import sys
import tempfile
import time
import urllib.request
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
NATIVE = ROOT / "native-gateway" / "target" / "debug" / (
    "ccg-native-gateway.exe" if os.name == "nt" else "ccg-native-gateway"
)


def free_port() -> int:
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        return int(sock.getsockname()[1])


def request(url: str, payload: dict[str, object] | None = None) -> bytes:
    body = None if payload is None else json.dumps(payload).encode()
    req = urllib.request.Request(
        url,
        data=body,
        headers={"content-type": "application/json", "anthropic-version": "2023-06-01"},
    )
    with urllib.request.urlopen(req, timeout=20) as response:
        return response.read()


def wait_ready(base: str, process: subprocess.Popen[bytes]) -> float:
    started = time.monotonic()
    for _ in range(80):
        if process.poll() is not None:
            raise RuntimeError(f"native gateway exited with {process.returncode}")
        try:
            request(f"{base}/health/liveliness")
            return time.monotonic() - started
        except Exception:
            time.sleep(0.05)
    raise RuntimeError("native gateway did not become ready")


def main() -> None:
    if not NATIVE.is_file():
        raise RuntimeError(f"build native gateway first: {NATIVE}")
    upstream_port = free_port()
    gateway_port = free_port()
    creationflags = subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0
    with tempfile.TemporaryDirectory(ignore_cleanup_errors=True) as directory:
        temp = Path(directory)
        models = temp / "models.json"
        profile = {
            "id": "mock",
            "name": "Mock Chat",
            "base_url": f"http://127.0.0.1:{upstream_port}/v1",
            "api_key": "test-only",
            "model_id": "mock-model",
            "protocol": "openai_chat",
            "auth_mode": "auto",
            "routing_enabled": True,
            "routing_weight": 1,
        }
        models.write_text(
            json.dumps(
                {
                    "version": 4,
                    "default_id": "mock",
                    "profiles": [profile],
                    "routing": {"enabled": False, "model_rules": []},
                }
            ),
            encoding="utf-8",
        )
        env = os.environ.copy()
        env.update(
            {
                "CCG_ROOT": str(temp),
                "CCG_MODELS_PATH": str(models),
                "CCG_ROUTING_TRAFFIC_PATH": str(temp / "routing-traffic.json"),
                "CCG_STATE_PATH": str(temp / "state.json"),
                "CCG_PORT": str(gateway_port),
            }
        )
        upstream = subprocess.Popen(
            [
                sys.executable,
                str(ROOT / "tests" / "mock_chat_upstream.py"),
                "--port",
                str(upstream_port),
                "--response-text",
                "native response",
                "--stream-text",
                "native stream",
            ],
            cwd=ROOT,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.PIPE,
            creationflags=creationflags,
        )
        gateway = subprocess.Popen(
            [str(NATIVE)],
            cwd=ROOT,
            env=env,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            creationflags=creationflags,
        )
        try:
            base = f"http://127.0.0.1:{gateway_port}"
            ready_seconds = wait_ready(base, gateway)
            models_result = json.loads(request(f"{base}/v1/models"))
            assert {item["id"] for item in models_result["data"]} >= {
                "codex-chat",
                "claude-sonnet-5",
            }

            response = json.loads(
                request(
                    f"{base}/v1/responses",
                    {"model": "codex-chat", "input": "ping", "max_output_tokens": 32},
                )
            )
            assert response["output"][0]["content"][0]["text"] == "native response"

            tool_response = json.loads(
                request(
                    f"{base}/v1/responses",
                    {
                        "model": "codex-chat",
                        "input": "use the tool",
                        "tools": [
                            {
                                "type": "function",
                                "name": "echo",
                                "description": "Echo text",
                                "parameters": {"type": "object"},
                            }
                        ],
                    },
                )
            )
            assert any(item.get("type") == "function_call" for item in tool_response["output"])

            response_stream = request(
                f"{base}/v1/responses",
                {"model": "codex-chat", "input": "stream", "stream": True},
            ).decode()
            assert "response.output_text.delta" in response_stream
            assert "native stream" in response_stream

            message = json.loads(
                request(
                    f"{base}/v1/messages",
                    {
                        "model": "claude-sonnet-5",
                        "max_tokens": 32,
                        "messages": [{"role": "user", "content": "ping"}],
                    },
                )
            )
            assert message["content"][0]["text"] == "native response"

            message_stream = request(
                f"{base}/v1/messages",
                {
                    "model": "claude-sonnet-5",
                    "max_tokens": 32,
                    "stream": True,
                    "messages": [{"role": "user", "content": "stream"}],
                },
            ).decode()
            assert "content_block_delta" in message_stream
            assert "native stream" in message_stream

            reload_result = json.loads(request(f"{base}/internal/ccg/reload", {}))
            assert reload_result["ok"] is True
            assert (temp / "routing-traffic.json").is_file()
            print(f"NATIVE_GATEWAY_READY_SECONDS={ready_seconds:.3f}")
        finally:
            for process in (gateway, upstream):
                if process.poll() is None:
                    process.terminate()
            for process in (gateway, upstream):
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    process.kill()


if __name__ == "__main__":
    main()
