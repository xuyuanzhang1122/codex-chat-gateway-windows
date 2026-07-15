from __future__ import annotations

import argparse
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import time


class Handler(BaseHTTPRequestHandler):
    def log_message(self, format: str, *args: object) -> None:
        return

    def do_GET(self) -> None:
        if not self.path.endswith("/models"):
            self.send_error(404)
            return
        payload = {"object": "list", "data": [{"id": "mock-chat"}, {"id": "mock-reasoner"}]}
        body = json.dumps(payload).encode("utf-8")
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_POST(self) -> None:
        if not self.path.endswith("/chat/completions"):
            self.send_error(404)
            return

        length = int(self.headers.get("Content-Length", "0"))
        request = json.loads(self.rfile.read(length) or b"{}")
        if request.get("stream") and not request.get("tools"):
            created = int(time.time())
            model = request.get("model", "mock-model")
            chunks = [
                {
                    "id": "chatcmpl-mock",
                    "object": "chat.completion.chunk",
                    "created": created,
                    "model": model,
                    "choices": [{"index": 0, "delta": {"role": "assistant", "content": ""}, "finish_reason": None}],
                },
                {
                    "id": "chatcmpl-mock",
                    "object": "chat.completion.chunk",
                    "created": created,
                    "model": model,
                    "choices": [{"index": 0, "delta": {"content": "mock stream ok"}, "finish_reason": None}],
                },
                {
                    "id": "chatcmpl-mock",
                    "object": "chat.completion.chunk",
                    "created": created,
                    "model": model,
                    "choices": [{"index": 0, "delta": {}, "finish_reason": "stop"}],
                    "usage": {"prompt_tokens": 5, "completion_tokens": 3, "total_tokens": 8},
                },
            ]
            self.send_response(200)
            self.send_header("Content-Type", "text/event-stream")
            self.send_header("Cache-Control", "no-cache")
            self.end_headers()
            for chunk in chunks:
                self.wfile.write(f"data: {json.dumps(chunk)}\n\n".encode("utf-8"))
                self.wfile.flush()
            self.wfile.write(b"data: [DONE]\n\n")
            self.wfile.flush()
            return

        message: dict[str, object]
        finish_reason = "stop"
        if request.get("tools"):
            message = {
                "role": "assistant",
                "content": None,
                "tool_calls": [
                    {
                        "id": "call_mock_1",
                        "type": "function",
                        "function": {"name": "echo", "arguments": '{"text":"ok"}'},
                    }
                ],
            }
            finish_reason = "tool_calls"
        else:
            message = {"role": "assistant", "content": "mock response ok"}

        payload = {
            "id": "chatcmpl-mock",
            "object": "chat.completion",
            "created": int(time.time()),
            "model": request.get("model", "mock-model"),
            "choices": [{"index": 0, "message": message, "finish_reason": finish_reason}],
            "usage": {"prompt_tokens": 5, "completion_tokens": 3, "total_tokens": 8},
        }
        body = json.dumps(payload).encode("utf-8")
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--port", type=int, default=4012)
    args = parser.parse_args()
    ThreadingHTTPServer(("127.0.0.1", args.port), Handler).serve_forever()


if __name__ == "__main__":
    main()
