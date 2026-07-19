from __future__ import annotations

import sys


def configure_utf8_stdio() -> None:
    for stream in (sys.stdout, sys.stderr):
        if stream is not None and hasattr(stream, "reconfigure"):
            stream.reconfigure(encoding="utf-8", errors="backslashreplace")


if __name__ == "__main__":
    configure_utf8_stdio()
    from gateway_runtime import install_prompt_cache_affinity, prepare_gateway_runtime

    prepare_gateway_runtime()
    install_prompt_cache_affinity()
    from litellm import run_server

    run_server()
