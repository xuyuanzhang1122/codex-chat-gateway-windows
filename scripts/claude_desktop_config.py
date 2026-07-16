from __future__ import annotations

import argparse
import json
import os
import tempfile
from dataclasses import dataclass
from pathlib import Path


PROFILE_ID = "3b6a62c4-e961-55b4-8e65-661d52f99e0d"
PROFILE_NAME = "Codex Chat Gateway"
CONFIG_FILE = "claude_desktop_config.json"
ROUTE_IDS = (
    "claude-sonnet-5",
    "claude-opus-4-8",
    "claude-haiku-4-5",
)


@dataclass(frozen=True)
class Paths:
    normal_config: Path
    threep_config: Path
    profile: Path
    meta: Path


def pick_claude_dir(local_app_data: Path, threep: bool) -> Path:
    exact_name = "Claude-3p" if threep else "Claude"
    exact = local_app_data / exact_name
    if exact.exists():
        return exact

    candidates: list[Path] = []
    if local_app_data.exists():
        for path in local_app_data.iterdir():
            if not path.is_dir():
                continue
            name = path.name
            if name.startswith("Claude") and (("-3p" in name) == threep):
                candidates.append(path)
    return sorted(candidates)[0] if candidates else exact


def get_paths(local_app_data: Path) -> Paths:
    normal_dir = pick_claude_dir(local_app_data, False)
    threep_dir = pick_claude_dir(local_app_data, True)
    library = threep_dir / "configLibrary"
    return Paths(
        normal_config=normal_dir / CONFIG_FILE,
        threep_config=threep_dir / CONFIG_FILE,
        profile=library / f"{PROFILE_ID}.json",
        meta=library / "_meta.json",
    )


def read_object(path: Path) -> dict[str, object]:
    if not path.exists():
        return {}
    try:
        value = json.loads(path.read_text(encoding="utf-8-sig"))
    except (OSError, UnicodeError, json.JSONDecodeError) as exc:
        raise RuntimeError(f"Cannot safely parse existing JSON file: {path}: {exc}") from exc
    if not isinstance(value, dict):
        raise RuntimeError(f"Existing JSON root must be an object: {path}")
    return value


def atomic_write_json(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    payload = json.dumps(value, ensure_ascii=False, indent=2) + "\n"
    handle, temp_name = tempfile.mkstemp(prefix=f".{path.name}.", suffix=".tmp", dir=path.parent)
    try:
        with os.fdopen(handle, "w", encoding="utf-8", newline="\n") as stream:
            stream.write(payload)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temp_name, path)
    except BaseException:
        try:
            os.unlink(temp_name)
        except FileNotFoundError:
            pass
        raise


def set_deployment_mode(path: Path, mode: str) -> None:
    value = read_object(path)
    value["deploymentMode"] = mode
    atomic_write_json(path, value)


def update_meta(path: Path, apply_profile: bool) -> None:
    value = read_object(path)
    raw_entries = value.get("entries", [])
    entries = list(raw_entries) if isinstance(raw_entries, list) else []
    entries = [
        entry
        for entry in entries
        if not (isinstance(entry, dict) and entry.get("id") == PROFILE_ID)
    ]

    if apply_profile:
        entries.append({"id": PROFILE_ID, "name": PROFILE_NAME})
        value["appliedId"] = PROFILE_ID
    elif value.get("appliedId") == PROFILE_ID:
        next_id = next(
            (
                entry.get("id")
                for entry in entries
                if isinstance(entry, dict) and isinstance(entry.get("id"), str)
            ),
            None,
        )
        if next_id:
            value["appliedId"] = next_id
        else:
            value.pop("appliedId", None)

    value["entries"] = entries
    atomic_write_json(path, value)


def build_profile(base_url: str, model_label: str) -> dict[str, object]:
    base_url = base_url.rstrip("/")
    label = model_label.strip() or "Current gateway model"
    return {
        "coworkEgressAllowedHosts": ["*"],
        "disableDeploymentModeChooser": True,
        "inferenceGatewayApiKey": "local-gateway",
        "inferenceGatewayAuthScheme": "bearer",
        "inferenceGatewayBaseUrl": base_url,
        "inferenceProvider": "gateway",
        "inferenceModels": [
            {"name": route, "labelOverride": f"{label} ({role})"}
            for route, role in zip(ROUTE_IDS, ("Sonnet", "Opus", "Haiku"), strict=True)
        ],
    }


def snapshot(paths: Paths) -> dict[Path, bytes | None]:
    return {
        path: path.read_bytes() if path.exists() else None
        for path in (paths.normal_config, paths.threep_config, paths.profile, paths.meta)
    }


def restore_snapshot(files: dict[Path, bytes | None]) -> None:
    for path, content in files.items():
        if content is None:
            path.unlink(missing_ok=True)
            continue
        path.parent.mkdir(parents=True, exist_ok=True)
        temp = path.with_name(f".{path.name}.rollback")
        temp.write_bytes(content)
        os.replace(temp, path)


def apply(paths: Paths, base_url: str, model_label: str) -> None:
    files = snapshot(paths)
    try:
        set_deployment_mode(paths.normal_config, "3p")
        set_deployment_mode(paths.threep_config, "3p")
        atomic_write_json(paths.profile, build_profile(base_url, model_label))
        update_meta(paths.meta, True)
    except BaseException:
        restore_snapshot(files)
        raise


def restore_official(paths: Paths) -> None:
    files = snapshot(paths)
    try:
        set_deployment_mode(paths.normal_config, "1p")
        set_deployment_mode(paths.threep_config, "1p")
        paths.profile.unlink(missing_ok=True)
        update_meta(paths.meta, False)
    except BaseException:
        restore_snapshot(files)
        raise


def main() -> int:
    parser = argparse.ArgumentParser(description="Configure Claude Desktop Code mode gateway profile.")
    parser.add_argument("action", choices=("apply", "restore"))
    parser.add_argument("--local-app-data", type=Path)
    parser.add_argument("--base-url", default="http://127.0.0.1:4000")
    parser.add_argument("--model-label", default="Current gateway model")
    args = parser.parse_args()

    local_app_data = args.local_app_data or Path(
        os.environ.get("LOCALAPPDATA", Path.home() / "AppData" / "Local")
    )
    paths = get_paths(local_app_data)
    if args.action == "apply":
        apply(paths, args.base_url, args.model_label)
        print(f"Claude Desktop Code profile configured: {paths.profile}")
        print("Fully quit and restart Claude Desktop before using Code mode.")
    else:
        restore_official(paths)
        print("Claude Desktop was switched back to official 1P mode.")
        print("Fully quit and restart Claude Desktop to apply the change.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
