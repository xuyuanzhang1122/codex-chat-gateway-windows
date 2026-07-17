from __future__ import annotations

import json
import subprocess
import sys
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts" / "claude_desktop_config.py"
PROFILE_ID = "3b6a62c4-e961-55b4-8e65-661d52f99e0d"


def load(path: Path) -> dict[str, object]:
    return json.loads(path.read_text(encoding="utf-8"))


def run(action: str, local: Path) -> None:
    subprocess.run(
        [
            sys.executable,
            str(SCRIPT),
            action,
            "--local-app-data",
            str(local),
            "--model-label",
            "DeepSeek Chat",
        ],
        check=True,
    )


def test_apply_and_restore() -> None:
    with tempfile.TemporaryDirectory() as temp:
        local = Path(temp)
        normal = local / "Claude" / "claude_desktop_config.json"
        threep = local / "Claude-3p" / "claude_desktop_config.json"
        library = local / "Claude-3p" / "configLibrary"
        meta = library / "_meta.json"
        profile = library / f"{PROFILE_ID}.json"

        normal.parent.mkdir(parents=True)
        normal.write_text('{"keepNormal": true, "deploymentMode": "1p"}', encoding="utf-8")
        threep.parent.mkdir(parents=True)
        threep.write_text('{"keepThreep": 7}', encoding="utf-8")
        library.mkdir(parents=True)
        meta.write_text(
            json.dumps(
                {
                    "appliedId": "other-profile",
                    "entries": [{"id": "other-profile", "name": "Other"}],
                    "keepMeta": True,
                }
            ),
            encoding="utf-8",
        )

        run("apply", local)
        assert load(normal)["deploymentMode"] == "3p"
        assert load(normal)["keepNormal"] is True
        assert load(threep)["deploymentMode"] == "3p"
        assert load(threep)["keepThreep"] == 7
        profile_json = load(profile)
        assert profile_json["inferenceProvider"] == "gateway"
        assert profile_json["inferenceGatewayBaseUrl"] == "http://127.0.0.1:4000"
        assert [item["name"] for item in profile_json["inferenceModels"]] == [
            "claude-sonnet-5",
            "claude-opus-4-8",
            "claude-haiku-4-5",
        ]
        assert load(meta)["appliedId"] == PROFILE_ID
        assert load(meta)["keepMeta"] is True

        run("restore", local)
        assert load(normal)["deploymentMode"] == "1p"
        assert load(normal)["keepNormal"] is True
        assert load(threep)["deploymentMode"] == "1p"
        assert load(threep)["keepThreep"] == 7
        assert not profile.exists()
        restored_meta = load(meta)
        assert restored_meta["appliedId"] == "other-profile"
        assert restored_meta["entries"] == [{"id": "other-profile", "name": "Other"}]
        assert restored_meta["keepMeta"] is True


def test_apply_rolls_back_on_invalid_existing_json() -> None:
    with tempfile.TemporaryDirectory() as temp:
        local = Path(temp)
        normal = local / "Claude" / "claude_desktop_config.json"
        threep = local / "Claude-3p" / "claude_desktop_config.json"
        normal.parent.mkdir(parents=True)
        threep.parent.mkdir(parents=True)
        original = b'{"deploymentMode":"1p","keep":true}'
        normal.write_bytes(original)
        threep.write_text("not-json", encoding="utf-8")

        result = subprocess.run(
            [sys.executable, str(SCRIPT), "apply", "--local-app-data", str(local)],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        assert result.returncode != 0
        assert normal.read_bytes() == original
        assert threep.read_text(encoding="utf-8") == "not-json"
        assert not (local / "Claude-3p" / "configLibrary" / f"{PROFILE_ID}.json").exists()


if __name__ == "__main__":
    test_apply_and_restore()
    test_apply_rolls_back_on_invalid_existing_json()
    print("Claude Desktop config tests passed.")
