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
    upstream: subprocess.Popen[str],
    gateway_log_path: Path,
    upstream_log_path: Path,
) -> dict[str, object]:
    deadline = time.monotonic() + 120
    last_error: Exception | None = None
    while time.monotonic() < deadline:
        for name, process, log_path in (
            ("gateway", gateway, gateway_log_path),
            ("mock upstream", upstream, upstream_log_path),
        ):
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
        f"gateway_running={gateway.poll() is None}; upstream_running={upstream.poll() is None}; "
        f"gateway_log={log_tail(gateway_log_path)}; upstream_log={log_tail(upstream_log_path)}"
    )


def main() -> None:
    upstream_port, gateway_port = free_ports(2)
    env = os.environ.copy()
    env.pop("PYTHONUTF8", None)
    env.update(
        {
            "PYTHONIOENCODING": "cp1252",
            "UPSTREAM_MODEL": "openai/mock-model",
            "CLAUDE_UPSTREAM_MODEL": "custom_openai/mock-model",
            "UPSTREAM_BASE_URL": f"http://127.0.0.1:{upstream_port}/v1",
            "UPSTREAM_API_KEY": "test-key",
        }
    )
    startup = subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0
    temp_directory = tempfile.TemporaryDirectory(ignore_cleanup_errors=True)
    log_root = Path(temp_directory.name)
    upstream_log_path = log_root / "upstream.stderr.log"
    gateway_log_path = log_root / "gateway.stderr.log"
    upstream_log = upstream_log_path.open("w", encoding="utf-8", errors="replace")
    gateway_log = gateway_log_path.open("w", encoding="utf-8", errors="replace")
    upstream = subprocess.Popen(
        [sys.executable, str(ROOT / "tests" / "mock_chat_upstream.py"), "--port", str(upstream_port)],
        cwd=ROOT,
        env=env,
        stdout=subprocess.DEVNULL,
        stderr=upstream_log,
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
        stderr=gateway_log,
        creationflags=startup,
    )
    try:
        base = f"http://127.0.0.1:{gateway_port}"
        models = wait_for_gateway(base, gateway, upstream, gateway_log_path, upstream_log_path)
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
        gateway_log.close()
        upstream_log.close()
        temp_directory.cleanup()


if __name__ == "__main__":
    main()
    print("ANTHROPIC_GATEWAY_OK")
