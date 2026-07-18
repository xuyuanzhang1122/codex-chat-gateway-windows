<p align="center">
  <img src="desktop/assets/gateway-logo.png" width="96" alt="Codex Chat Gateway">
</p>

<h1 align="center">Codex Chat Gateway</h1>

<p align="center">
  把第三方 Chat Completions 接口接到本机<br>
  <strong>Codex</strong>（<code>/v1/responses</code>）与 <strong>Claude Desktop</strong> Code 模式
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
  <a href="https://github.com/xuyuanzhang1122/codex-chat-gateway-windows/releases/latest"><b>下载 Windows Studio 安装包</b></a>
  ·
  <a href="docs/RELEASE_AND_UPDATES.md">发布与更新</a>
  ·
  <a href="docs/STRUCTURE.md">仓库结构</a>
</p>

---

<p align="center">
  <img src="docs/assets/studio-gateway.png" width="920" alt="Studio 控制台">
</p>

社区工具，**不是** OpenAI 官方项目。  
网关只监听 `127.0.0.1`，密钥只留在本机。

## 它解决什么问题

Codex 走 Responses API，Claude Desktop 的 Code 模式走 Anthropic Messages，而 DeepSeek、Kimi 等多数只提供 OpenAI 风格的 Chat Completions。

本项目在 `http://127.0.0.1:4000` 运行 [LiteLLM](https://github.com/BerriAI/litellm)，并提供 Windows Studio 控制台：管理模型、启停网关、接入客户端。协议转换交给 LiteLLM，本仓库负责安装包装、安全写配置、本机控制台。

```text
  Codex ──/v1/responses──┐
                         ├──► 127.0.0.1:4000 (LiteLLM) ──► DeepSeek / Kimi / OpenAI 兼容接口
  Claude Desktop Code ───┘
```

## 功能

| | |
|---|---|
| **Studio 控制台** | Tauri 2 + React + [LobeHub UI](https://ui.lobehub.com/)。无边框窗口；关窗进托盘，**不停止网关**。 |
| **模型管理** | 增删改、默认模型、在线拉取 `/models`，支持导入 `baseurl` / `key` / `model` 文本。 |
| **客户端接入** | 一键写 Codex 提供方与 Claude Desktop Code 模式 3P Profile；恢复时保留 MCP 与其他 Profile。 |
| **自动更新** | HTTPS GitHub Releases（`latest.json` + minisign 验签），**不改** `.gateway`。 |
| **安装包** | 用户级 Studio 安装；可选卸载旧版 C# 桌面程序。 |

## 安装使用

1. 从 [Releases](https://github.com/xuyuanzhang1122/codex-chat-gateway-windows/releases/latest) 下载 **`CodexChatGateway-Studio-Setup-v*.exe`**。
2. 打开 **Codex Chat Gateway**。
3. **模型** → 添加配置（或 **导入 txt**）。
4. **网关** → 启动。
5. **客户端** → 配置 Codex / Claude Desktop，然后**完全退出并重启**对应客户端。

接入后 Codex 中应使用：

| | |
|---|---|
| 模型名 | `codex-chat` |
| 地址 | `http://127.0.0.1:4000/v1` |

### 导入文本格式

```text
baseurl：https://api.deepseek.com
key:sk-xxxxxxxx
model:deepseek-v4-flash,deepseek-v4-pro
```

`model` 可留空，会提示是否在线拉取列表。  
支持 `：` / `:` / `=`，以及 `base_url`、`api_key` 等别名。

## 源码目录

| 路径 | 作用 |
|------|------|
| `desktop-tauri/` | Studio 界面 + Rust 网关管理 |
| `bin/` | 源码目录下的启动脚本 |
| `scripts/` | 配置 / 启停 / 构建（PowerShell 纯 ASCII） |
| `desktop/` | 旧 WPF 控制台（与 Studio 并存） |
| `docs/` | 发布、便携、结构说明 |
| `examples/` | 示例 Codex 提供方 TOML |

克隆后常用入口：

```powershell
.\Studio.bat                 # 或 .\bin\desktop-tauri.bat
.\bin\start-gateway.bat
.\构建Studio安装器.bat
```

```powershell
cd desktop-tauri
npm install
npm run tauri dev
```

## 自动更新

用户：控制台 **客户端 → 检查更新**。启动时只做静默探测，不会自动下载。

发布者用私钥签名（**私钥不得进仓库**）：

```powershell
$env:TAURI_SIGNING_PRIVATE_KEY_PATH = "$env:USERPROFILE\.codex-chat-gateway\tauri-updater.key"
.\scripts\build-updater-artifacts.ps1
```

把更新 zip 与 Release 根目录的 **`latest.json`** 一同上传。详见 [docs/RELEASE_AND_UPDATES.md](docs/RELEASE_AND_UPDATES.md)。

## 安全边界

- 固定监听本机回环地址。
- 上游 Key 只在进程环境 / `.gateway/models.json`，不会写入 Codex TOML、Claude Profile、日志或前端静态资源。
- 恢复脚本只撤销本项目写入的字段。
- 签名私钥与 `.env` / `.gateway` 禁止提交。

## 已知限制

- 上游模型仍需可靠的工具调用，Codex 代理任务才能稳定完成。
- LiteLLM 无法映射的可选参数会被丢弃。
- LiteLLM 锁定在含工具消息相邻性修复的提交（见 `CHANGELOG` / `requirements.txt`）。

## 致谢

- [LiteLLM](https://github.com/BerriAI/litellm) — 协议转换  
- [LobeHub UI](https://ui.lobehub.com/) — Studio 组件  
- [Tauri](https://tauri.app/) — 桌面壳与更新器  
- Claude Desktop 3P Profile 结构参考 [cc-switch](https://github.com/farion1231/cc-switch)

维护者：[xuyuanzhang1122](https://github.com/xuyuanzhang1122)

## 许可

[MIT](LICENSE) · [更新记录](CHANGELOG.md) · [贡献指南](CONTRIBUTING.md) · [第三方声明](THIRD_PARTY_NOTICES.md)
