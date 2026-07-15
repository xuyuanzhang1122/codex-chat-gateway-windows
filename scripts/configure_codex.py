from __future__ import annotations

import argparse
import datetime as dt
import json
from pathlib import Path
import shutil
import sys

import tomlkit


GATEWAY_PROVIDER = "local-chat-gateway"
GATEWAY_MODEL = "codex-chat"


def _plain(value: object) -> object:
    unwrap = getattr(value, "unwrap", None)
    return unwrap() if callable(unwrap) else value


def capture_relevant(document: object) -> dict[str, object]:
    providers = document.get("model_providers")
    local_provider = providers.get(GATEWAY_PROVIDER) if providers is not None else None
    return {
        "version": 1,
        "model": {"present": "model" in document, "value": _plain(document.get("model"))},
        "model_provider": {
            "present": "model_provider" in document,
            "value": _plain(document.get("model_provider")),
        },
        "local_provider": {
            "present": local_provider is not None,
            "value": _plain(local_provider),
        },
    }


def is_gateway_config(document: object) -> bool:
    return (
        str(document.get("model", "")) == GATEWAY_MODEL
        and str(document.get("model_provider", "")) == GATEWAY_PROVIDER
    )


def find_clean_backup(path: Path) -> dict[str, object] | None:
    backups = sorted(
        path.parent.glob(f"{path.name}.bak-*-chat-gateway"),
        key=lambda item: item.stat().st_mtime,
        reverse=True,
    )
    for backup in backups:
        try:
            candidate = tomlkit.parse(backup.read_text(encoding="utf-8"))
        except Exception:
            continue
        if not is_gateway_config(candidate):
            return capture_relevant(candidate)
    return None


def empty_official_snapshot() -> dict[str, object]:
    return {
        "version": 1,
        "model": {"present": False, "value": None},
        "model_provider": {"present": False, "value": None},
        "local_provider": {"present": False, "value": None},
    }


def backup_config(path: Path) -> Path | None:
    if not path.exists():
        return None
    stamp = dt.datetime.now().strftime("%Y%m%d-%H%M%S-%f")
    backup = path.with_name(f"{path.name}.bak-{stamp}-chat-gateway")
    shutil.copy2(path, backup)
    return backup


def write_document(path: Path, document: object) -> None:
    rendered = tomlkit.dumps(document)
    tomlkit.parse(rendered)
    temporary = path.with_name(f"{path.name}.tmp-chat-gateway")
    temporary.write_text(rendered, encoding="utf-8")
    temporary.replace(path)


def main() -> int:
    parser = argparse.ArgumentParser(description="Safely add the local Responses gateway to Codex config.")
    parser.add_argument("--config", required=True, type=Path)
    parser.add_argument("--state", required=True, type=Path)
    parser.add_argument("--port", default=4000, type=int)
    args = parser.parse_args()

    path: Path = args.config.expanduser().resolve()
    state_path: Path = args.state.expanduser().resolve()
    path.parent.mkdir(parents=True, exist_ok=True)
    state_path.parent.mkdir(parents=True, exist_ok=True)
    original = path.read_text(encoding="utf-8") if path.exists() else ""

    try:
        document = tomlkit.parse(original) if original.strip() else tomlkit.document()
    except Exception as exc:
        print(f"Codex config could not be parsed; no changes were made: {exc}", file=sys.stderr)
        return 2

    if not state_path.exists():
        if is_gateway_config(document):
            snapshot = find_clean_backup(path) or empty_official_snapshot()
        else:
            snapshot = capture_relevant(document)
        state_path.write_text(json.dumps(snapshot, ensure_ascii=True, indent=2), encoding="utf-8")
        print(f"Restore state: {state_path}")

    backup = backup_config(path)
    if backup is not None:
        print(f"Backup: {backup}")

    document["model"] = GATEWAY_MODEL
    document["model_provider"] = GATEWAY_PROVIDER

    providers = document.get("model_providers")
    if providers is None:
        providers = tomlkit.table()
        document["model_providers"] = providers

    provider = tomlkit.table()
    provider["name"] = "Local Chat-to-Responses Gateway"
    provider["base_url"] = f"http://127.0.0.1:{args.port}/v1"
    provider["wire_api"] = "responses"
    providers[GATEWAY_PROVIDER] = provider

    write_document(path, document)
    print(f"Configured: {path}")
    print("Fully exit and restart Codex.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
