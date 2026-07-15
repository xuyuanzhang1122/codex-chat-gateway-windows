from __future__ import annotations

from pathlib import Path
import subprocess
import sys
import tempfile

import tomlkit


SCRIPTS = Path(__file__).resolve().parents[1] / "scripts"


def run(script: str, config: Path, state: Path) -> None:
    command = [sys.executable, str(SCRIPTS / script), "--config", str(config), "--state", str(state)]
    if script == "configure_codex.py":
        command.extend(["--port", "4000"])
    subprocess.run(command, check=True, capture_output=True, text=True)


def parse(path: Path):
    return tomlkit.parse(path.read_text(encoding="utf-8"))


def main() -> None:
    with tempfile.TemporaryDirectory() as temporary:
        root = Path(temporary)
        config = root / "config.toml"
        state = root / "restore.json"
        config.write_text(
            'model = "gpt-5.4"\nmodel_provider = "openai"\n\n'
            '[features]\nkeep_feature = true\n\n'
            '[mcp_servers.keep]\ncommand = "node"\n',
            encoding="utf-8",
        )

        run("configure_codex.py", config, state)
        configured = parse(config)
        assert configured["model"] == "codex-chat"
        assert configured["mcp_servers"]["keep"]["command"] == "node"
        configured["plugins"] = {"added_later": {"enabled": True}}
        config.write_text(tomlkit.dumps(configured), encoding="utf-8")

        run("restore_codex.py", config, state)
        restored = parse(config)
        assert restored["model"] == "gpt-5.4"
        assert restored["model_provider"] == "openai"
        assert "local-chat-gateway" not in restored.get("model_providers", {})
        assert restored["mcp_servers"]["keep"]["command"] == "node"
        assert restored["plugins"]["added_later"]["enabled"] is True
        assert not state.exists()

    with tempfile.TemporaryDirectory() as temporary:
        root = Path(temporary)
        config = root / "config.toml"
        state = root / "missing-state.json"
        clean_backup = root / "config.toml.bak-20260715-120000-chat-gateway"
        clean_backup.write_text('model = "gpt-5.4"\nmodel_provider = "openai"\n', encoding="utf-8")
        config.write_text(
            'model = "codex-chat"\nmodel_provider = "local-chat-gateway"\n\n'
            '[model_providers.local-chat-gateway]\nbase_url = "http://127.0.0.1:4000/v1"\nwire_api = "responses"\n\n'
            '[mcp_servers.keep]\ncommand = "node"\n',
            encoding="utf-8",
        )
        run("restore_codex.py", config, state)
        restored = parse(config)
        assert restored["model"] == "gpt-5.4"
        assert restored["model_provider"] == "openai"
        assert restored["mcp_servers"]["keep"]["command"] == "node"

    print("CODEX_RESTORE_OK")


if __name__ == "__main__":
    main()
