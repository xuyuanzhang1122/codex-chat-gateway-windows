# Contributing

Issues and PRs welcome.

## Before you commit

1. Never commit API keys, `.env`, `.gateway`, logs, or personal Codex config.
2. Keep PowerShell under `scripts/` pure ASCII (Windows PowerShell 5.1).
3. Do not commit updater **private** keys (`TAURI_SIGNING_PRIVATE_KEY*`).
4. Prefer editing `desktop-tauri/` for the console; leave legacy `desktop/` unless you intend to touch WPF.
5. Launchers live in `bin/` — keep paths relative to the repo root (`..\scripts\`).

## Local checks

```powershell
.\.venv\Scripts\python.exe .\tests\test_tool_output_adjacency.py
.\.venv\Scripts\python.exe .\tests\test_codex_restore.py
```

```powershell
cd desktop-tauri
npm run build
npm run tauri build -- --no-bundle
```

Bump `VERSION` (and Studio package versions) before tagging `vX.Y.Z`.  
See [docs/STRUCTURE.md](docs/STRUCTURE.md) and [docs/RELEASE_AND_UPDATES.md](docs/RELEASE_AND_UPDATES.md).
