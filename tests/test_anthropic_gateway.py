from __future__ import annotations

import json
import os
import socket
import subprocess
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def free_port() -> int:
    with socket.socket() as listener:
        listener.bind(("127.0.0.1", 0))
        return int(listener.getsockname()[1])


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


def wait_for_gateway(base: str) -> dict[str, object]:
    deadline = time.monotonic() + 45
    last_error: Exception | None = None
    while time.monotonic() < deadline:
        try:
            return json.loads(request(f"{base}/v1/models"))
        except (OSError, urllib.error.URLError, json.JSONDecodeError) as exc:
            last_error = exc
            time.sleep(0.25)
    raise RuntimeError(f"Gateway did not become ready: {last_error}")


def main() -> None:
    upstream_port = free_port()
    gateway_port = free_port()
    env = os.environ.copy()
    env.update(
        {
            "UPSTREAM_MODEL": "openai/mock-model",
            "CLAUDE_UPSTREAM_MODEL": "custom_openai/mock-model",
            "UPSTREAM_BASE_URL": f"http://127.0.0.1:{upstream_port}/v1",
            "UPSTREAM_API_KEY": "test-key",
        }
    )
    startup = subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0
    upstream = subprocess.Popen(
        [sys.executable, str(ROOT / "tests" / "mock_chat_upstream.py"), "--port", str(upstream_port)],
        cwd=ROOT,
        env=env,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
        creationflags=startup,
    )
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
        stderr=subprocess.PIPE,
        creationflags=startup,
    )
    try:
        base = f"http://127.0.0.1:{gateway_port}"
        models = wait_for_gateway(base)
        ids = {item["id"] for item in models["data"]}
        required = {"codex-chat", "claude-sonnet-5", "claude-opus-4-8", "claude-haiku-4-5"}
        assert required <= ids

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
        assert basic["content"] == [{"type": "text", "text": "mock response ok"}]

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
        assert "mock stream ok" in stream
    finally:
        for process in (gateway, upstream):
            if process.poll() is None:
                process.terminate()
        for process in (gateway, upstream):
            try:
                process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=5)


if __name__ == "__main__":
    main()
    print("ANTHROPIC_GATEWAY_OK")
