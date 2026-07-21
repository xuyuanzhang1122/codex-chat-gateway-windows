# Codex Chat Gateway

A native Windows gateway that connects Codex, Claude Code, and OpenAI-compatible clients to third-party model providers while listening only on `127.0.0.1`.

## Architecture

```text
Codex (Responses) ─────────────┐
OpenAI clients (Chat) ─────────┼─► native Rust gateway ─► Responses / Chat / Anthropic upstreams
Claude Code (Anthropic) ──────┘
```

- Matching client and upstream protocols are passed through without redundant conversion.
- Cross-protocol traffic uses the reusable `linguafranca` Rust library for Responses, Chat Completions, Anthropic Messages, SSE, and tool calls.
- One model can use multiple upstream accounts with different native protocols, weights, session affinity, retries, and failover.
- The gateway is a standalone precompiled Rust process and keeps running when Studio closes.
- LiteLLM, the Python runtime, BAT launchers, and the C#/WPF client are not included or supported.

## Use

Releases contain one supported artifact: `CodexChatGateway-Studio-Setup-v*.exe`.

1. Add a model and upstream accounts in Studio, including each upstream's native protocol and authentication mode.
2. Start the local gateway.
3. Let Studio configure Codex or Claude Desktop Code mode.

Models and keys stay in the uncommitted `.gateway/models.json`. Keys are never copied into Codex configuration, frontend assets, or logs.

## Development

```powershell
cd desktop-tauri
npm install
npm run tauri dev
```

Gateway tests:

```powershell
cargo test --manifest-path native-gateway/Cargo.toml
python tests/test_native_gateway.py
```

Build the only supported installer:

```powershell
.\scripts\build-tauri-installer.ps1
```

## Layout

| Path | Purpose |
| --- | --- |
| `desktop-tauri/` | Tauri 2 + React Studio |
| `native-gateway/` | Rust gateway and protocol routing |
| `scripts/` | Build, update, and background process maintenance |
| `installer/` | Studio Inno installer |
| `.gateway/models.json` | Local model configuration; never committed |

BAT, C#/WPF, Python/LiteLLM, and old portable distributions are end-of-life and are no longer built or released.
