<p align="center">
  <img src="desktop/assets/gateway-logo.png" width="96" alt="Codex Chat Gateway">
</p>

<h1 align="center">Codex Chat Gateway</h1>

<p align="center">
  Local bridge from third-party Chat Completions APIs to<br>
  <strong>Codex</strong> (<code>/v1/responses</code>) and <strong>Claude Desktop</strong> Code mode.
</p>

<p align="center">
  <a href="README.md">English</a> ·
  <a href="README.zh-CN.md">简体中文</a>
</p>

<p align="center">
  <a href="https://github.com/xuyuanzhang1122/codex-chat-gateway-windows/actions/workflows/release.yml"><img src="https://github.com/xuyuanzhang1122/codex-chat-gateway-windows/actions/workflows/release.yml/badge.svg" alt="build"></a>
  <a href="https://github.com/xuyuanzhang1122/codex-chat-gateway-windows/releases"><img src="https://img.shields.io/github/v/release/xuyuanzhang1122/codex-chat-gateway-windows" alt="release"></a>
  <img src="https://img.shields.io/badge/platform-Windows%20x64-0078d7" alt="platform">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-green" alt="license"></a>
</p>

<p align="center">
  <a href="https://github.com/xuyuanzhang1122/codex-chat-gateway-windows/releases/latest"><b>Download Studio for Windows</b></a>
  ·
  <a href="docs/RELEASE_AND_UPDATES.md">Release & updates</a>
  ·
  <a href="docs/STRUCTURE.md">Repo layout</a>
</p>

---

<p align="center">
  <img src="docs/assets/studio-gateway.png" width="920" alt="Studio console">
</p>

Community tool — **not** an official OpenAI product.  
The gateway always binds to `127.0.0.1`. Keys stay on your machine.

## Why

Codex talks Responses API. Claude Desktop Code mode talks Anthropic Messages. Most providers only expose OpenAI-style Chat Completions.

This project runs [LiteLLM](https://github.com/BerriAI/litellm) on `http://127.0.0.1:4000` and ships a Windows Studio console for models, process lifecycle, and client wiring. Protocol conversion is delegated to LiteLLM; we focus on packaging, safe config writes, and a local control surface.

```text
  Codex ──/v1/responses──┐
                         ├──► 127.0.0.1:4000 (LiteLLM) ──► DeepSeek / Kimi / OpenAI-compatible
  Claude Desktop Code ───┘
```

## Features

| | |
|---|---|
| **Studio console** | Tauri 2 + React + [LobeHub UI](https://ui.lobehub.com/). Frameless window, tray (close does not kill the gateway). |
| **Models** | CRUD, default selection, remote `/models` list, import `baseurl` / `key` / `model` text files. |
| **Clients** | One-click Codex provider write + Claude Desktop 3P Profile (Code mode only). Restore keeps MCP / other profiles. |
| **Updates** | Signed updates over HTTPS GitHub Releases (`latest.json` + minisign). Does not rewrite `.gateway`. |
| **Installer** | User-level Studio setup with optional removal of the legacy C# desktop. |

## Install

1. Grab **`CodexChatGateway-Studio-Setup-v*.exe`** from [Releases](https://github.com/xuyuanzhang1122/codex-chat-gateway-windows/releases/latest).
2. Open **Codex Chat Gateway**.
3. **Models** → add a profile (or **Import txt**).
4. **Gateway** → Start.
5. **Clients** → configure Codex and/or Claude Desktop, then fully restart those apps.

Codex should use:

| | |
|---|---|
| Model | `codex-chat` |
| Base URL | `http://127.0.0.1:4000/v1` |

### Import text format

```text
baseurl：https://api.deepseek.com
key:sk-xxxxxxxx
model:deepseek-v4-flash,deepseek-v4-pro
```

`model` may be empty; the console will offer an online list fetch.  
`：` / `:` / `=` and common key aliases (`base_url`, `api_key`, …) are accepted.

## Source layout

| Path | Purpose |
|------|---------|
| `desktop-tauri/` | Studio UI + Rust gateway manager |
| `bin/` | Launchers for a source checkout |
| `scripts/` | Configure / start / build automation (ASCII PowerShell) |
| `desktop/` | Legacy WPF console (kept until Studio fully replaces it) |
| `docs/` | Release, portable, structure notes |
| `examples/` | Sample Codex provider TOML |

Common entry points from a clone:

```powershell
.\Studio.bat                 # or .\bin\desktop-tauri.bat
.\bin\start-gateway.bat
.\bin\build-tauri-installer.ps1   # via 构建Studio安装器.bat
```

```powershell
cd desktop-tauri
npm install
npm run tauri dev
```

## Auto-update

Users: **Clients → Check for updates**. Startup only logs availability; nothing downloads without consent.

Publishers sign artifacts with a private key that **never** enters the repo:

```powershell
$env:TAURI_SIGNING_PRIVATE_KEY_PATH = "$env:USERPROFILE\.codex-chat-gateway\tauri-updater.key"
.\scripts\build-updater-artifacts.ps1
```

Upload the updater zip and a root-level **`latest.json`** on the GitHub Release. Details: [docs/RELEASE_AND_UPDATES.md](docs/RELEASE_AND_UPDATES.md).

## Security

- Listen address is fixed to loopback.
- Upstream keys live in process env / `.gateway/models.json` only — not in Codex TOML, Claude profiles, logs, or the webview bundle.
- Codex / Claude restore scripts reverse **only** this project's fields.
- Signing private keys and `.env` / `.gateway` must not be committed.

## Limits

- Upstream models still need solid tool-calling for Codex agent work.
- Optional params LiteLLM cannot map are dropped.
- LiteLLM is pinned to a known commit that includes tool-message adjacency fixes (see `CHANGELOG` / requirements).

## Credits

- [LiteLLM](https://github.com/BerriAI/litellm) — protocol bridge  
- [LobeHub UI](https://ui.lobehub.com/) — Studio components  
- [Tauri](https://tauri.app/) — desktop shell & updater  
- Claude Desktop 3P Profile shape cross-checked against [cc-switch](https://github.com/farion1231/cc-switch)

Maintainer: [xuyuanzhang1122](https://github.com/xuyuanzhang1122)

## License

[MIT](LICENSE) · [Changelog](CHANGELOG.md) · [Contributing](CONTRIBUTING.md) · [Third-party notices](THIRD_PARTY_NOTICES.md)
