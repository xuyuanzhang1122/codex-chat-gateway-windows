# Repository layout

```text
codex-chat-gateway/
├── README.md                 Project overview
├── VERSION                   Release version (keep in sync with desktop-tauri)
├── config.yaml               LiteLLM gateway config
├── run_gateway.py            Gateway entry (spawned by Studio / scripts)
├── gateway_runtime.py         Runtime routing config + cache affinity adapter
├── requirements.txt
├── Studio.bat                Thin alias → bin/desktop-tauri.bat
├── bin/                      User-facing launchers (EN + ZH)
├── desktop-tauri/            Studio console (Tauri 2 + React + LobeHub) — primary UI
├── desktop/                  Legacy WPF console (kept until Studio is the only ship path)
├── scripts/                  PowerShell / Python automation (ASCII .ps1)
├── installer/                Inno Setup sources (Studio)
├── patches/                  Vendored LiteLLM pin notes
├── tests/                    Regression tests
├── examples/                 Sample Codex provider TOML, etc.
├── docs/                     Release, portable, structure
└── .github/workflows/        CI
```

## What belongs where

| Path | Role |
|------|------|
| `desktop-tauri/` | New Studio UI and Rust gateway manager |
| `desktop/` | Old WPF UI source; do not delete until distribution policy says so |
| `bin/` | Double-click launchers for source checkouts |
| `scripts/` | Real implementation of configure / start / build |
| `docs/` | Human docs; keep root README short |

Build outputs (`dist/`, `dist-installer/`, `runtime/`, `.venv/`) stay gitignored.
