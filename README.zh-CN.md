# Codex Chat Gateway

面向 Windows 的本地多协议 AI 网关。它把 Codex、Claude Code 和 OpenAI 兼容客户端统一接入你配置的第三方模型，同时保持网关只监听 `127.0.0.1`。

## 当前架构

```text
Codex (Responses) ─────────────┐
OpenAI 客户端 (Chat) ─────────┼─► Rust 原生网关 ─► Responses / Chat / Anthropic 上游
Claude Code (Anthropic) ──────┘
```

- 上下游协议相同：请求和响应直接透传，不做冗余转换。
- 上下游协议不同：使用 `linguafranca` Rust 库转换 Responses、Chat Completions 和 Anthropic Messages，包括 SSE 与工具调用。
- 同一模型可配置多个不同协议的上游账号，支持权重、会话亲和、重试和故障转移。
- 网关是独立的预编译 Rust 进程；关闭 Studio 不会默认停止网关。
- 不包含 LiteLLM、Python runtime、BAT 启动器或 C#/WPF 客户端。

## 使用

发布版只提供 `CodexChatGateway-Studio-Setup-v*.exe`。安装并打开 Studio 后：

1. 新建模型和上游账号，选择上游原生协议及认证方式。
2. 启动本地网关。
3. 由 Studio 配置 Codex 或 Claude Desktop Code 模式。

模型与密钥保存在未提交的 `.gateway/models.json`。密钥不会写进 Codex 配置、前端资源或日志。

## 开发

```powershell
cd desktop-tauri
npm install
npm run tauri dev
```

原生网关测试：

```powershell
cargo test --manifest-path native-gateway/Cargo.toml
python tests/test_native_gateway.py
```

构建唯一受支持的安装包：

```powershell
.\scripts\build-tauri-installer.ps1
```

## 目录

| 路径 | 用途 |
| --- | --- |
| `desktop-tauri/` | Tauri 2 + React Studio |
| `native-gateway/` | Rust 网关与协议路由 |
| `scripts/` | 构建、更新和后台进程维护 |
| `installer/` | Studio Inno 安装器 |
| `.gateway/models.json` | 本机模型配置，不提交 |

BAT、C#/WPF、Python/LiteLLM 与旧便携版已终止支持，不再构建或发布。
