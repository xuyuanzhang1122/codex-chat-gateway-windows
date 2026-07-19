from __future__ import annotations

import os
import sys

# Embeddable Python (python311._pth) does not put the script directory on
# sys.path; add it explicitly so gateway_runtime is importable.
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))


def configure_utf8_stdio() -> None:
    for stream in (sys.stdout, sys.stderr):
        if stream is not None and hasattr(stream, "reconfigure"):
            stream.reconfigure(encoding="utf-8", errors="backslashreplace")


if __name__ == "__main__":
    configure_utf8_stdio()
    from gateway_runtime import (
        install_prompt_cache_affinity,
        install_routing_telemetry,
        prepare_gateway_runtime,
    )

    prepare_gateway_runtime()
    install_prompt_cache_affinity()
    install_routing_telemetry()
    from litellm import run_server

    run_server()
