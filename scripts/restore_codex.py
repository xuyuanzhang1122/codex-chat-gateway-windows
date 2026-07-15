from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys

import tomlkit

SCRIPT_DIRECTORY = Path(__file__).resolve().parent
if str(SCRIPT_DIRECTORY) not in sys.path:
    sys.path.insert(0, str(SCRIPT_DIRECTORY))

from configure_codex import (
    GATEWAY_PROVIDER,
    backup_config,
    empty_official_snapshot,
    find_clean_backup,
    write_document,
)


def restore_entry(document: object, key: str, entry: dict[str, object]) -> None:
    if entry.get("present"):
        document[key] = entry.get("value")
    elif key in document:
        del document[key]


def main() -> int:
    parser = argparse.ArgumentParser(description="Restore Codex settings used before the local gateway.")
    parser.add_argument("--config", required=True, type=Path)
    parser.add_argument("--state", required=True, type=Path)
    args = parser.parse_args()

    path = args.config.expanduser().resolve()
    state_path = args.state.expanduser().resolve()
    if not path.exists():
        print(f"Codex config does not exist: {path}")
        print("Codex is already using its official defaults.")
        return 0

    try:
        document = tomlkit.parse(path.read_text(encoding="utf-8"))
    except Exception as exc:
        print(f"Codex config could not be parsed; no changes were made: {exc}", file=sys.stderr)
        return 2

    try:
        if state_path.exists():
            snapshot = json.loads(state_path.read_text(encoding="utf-8"))
        else:
            snapshot = find_clean_backup(path) or empty_official_snapshot()
    except Exception as exc:
        print(f"Restore state could not be read; no changes were made: {exc}", file=sys.stderr)
        return 3

    backup = backup_config(path)
    if backup is not None:
        print(f"Backup: {backup}")

    restore_entry(document, "model", snapshot["model"])
    restore_entry(document, "model_provider", snapshot["model_provider"])

    providers = document.get("model_providers")
    local_entry = snapshot["local_provider"]
    if local_entry.get("present"):
        if providers is None:
            providers = tomlkit.table()
            document["model_providers"] = providers
        providers[GATEWAY_PROVIDER] = local_entry.get("value")
    elif providers is not None and GATEWAY_PROVIDER in providers:
        del providers[GATEWAY_PROVIDER]
        if len(providers) == 0:
            del document["model_providers"]

    write_document(path, document)
    if state_path.exists():
        state_path.unlink()
    print(f"Restored official Codex configuration: {path}")
    print("MCP, plugins, features, and unrelated settings were preserved.")
    print("Fully exit and restart Codex.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
