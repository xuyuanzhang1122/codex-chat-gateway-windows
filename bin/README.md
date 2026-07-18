# Launchers

Thin `.bat` wrappers for a **source checkout**. They call scripts under `../scripts/` (or `../desktop-tauri` for Studio).

| Launcher | Action |
|----------|--------|
| `desktop-tauri.bat` / `桌面版-Tauri.bat` | Dev Studio (`npm run tauri dev`) |
| `start-gateway.bat` / `启动网关.bat` | Start LiteLLM gateway in background |
| `stop-gateway.bat` / `停止网关.bat` | Stop gateway |
| `model-config.bat` / `模型配置.bat` | CLI model manager |
| `configure-codex.bat` / `配置Codex.bat` | Write Codex provider |
| `configure-claude-desktop.bat` | Claude Desktop Code 3P Profile |
| `restore-official-*.bat` | Reverse gateway-only config |
| `构建Studio安装器.bat` | Build Studio installer |

Root aliases such as `Studio.bat` and `start-gateway.bat` simply call into this folder.

Portable packages flatten these next to `scripts/` (see `scripts/build-portable.ps1`).
