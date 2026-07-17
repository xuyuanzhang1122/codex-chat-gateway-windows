# Codex Chat Gateway

[![Build portable release](https://github.com/xuyuanzhang1122/codex-chat-gateway-windows/actions/workflows/release.yml/badge.svg)](https://github.com/xuyuanzhang1122/codex-chat-gateway-windows/actions/workflows/release.yml)
[![GitHub Release](https://img.shields.io/github/v/release/xuyuanzhang1122/codex-chat-gateway-windows)](https://github.com/xuyuanzhang1122/codex-chat-gateway-windows/releases)

把只支持 Chat Completions 的第三方模型接入 Codex，以及 Claude Desktop 内嵌的 Code 模式。Codex 请求本机的 `/v1/responses`，Claude Desktop 请求 Anthropic Messages 兼容接口，LiteLLM 负责转换成上游模型商支持的协议。

> 本项目是社区兼容工具，不是 OpenAI 官方项目。

## 为什么不自己实现转换器

Responses API 不只是字段改名，还涉及 SSE 流式事件、工具调用、错误映射、参数兼容和多轮上下文。本项目直接复用 LiteLLM，只负责 Windows 一键安装、密钥隔离、Codex TOML 安全写入和健康检查。

## 下载与安装

普通用户请前往 [GitHub Releases](https://github.com/xuyuanzhang1122/codex-chat-gateway-windows/releases/latest) 下载最新版：

- `CodexChatGateway-Setup-vX.Y.Z.exe`：推荐，品牌化图形安装程序。
- `codex-chat-gateway-portable-vX.Y.Z-windows-x64.7z`：免安装便携版。
- 同名 `.sha256`：用于校验下载文件完整性。

## 便携分发版：空白 Windows 直接使用

便携版目录中包含 `runtime/python.exe` 和全部已验证依赖，不要求目标电脑安装 Python、Docker、Git、Codex CLI 或联网下载运行库。目标系统为 Windows x64，并需要能够访问上游模型 API。

### 美化安装包

发布给普通用户时优先使用 `CodexChatGateway-Setup-vX.Y.Z.exe`。安装程序采用品牌化深色界面，支持中文/英文、用户级免管理员安装、开始菜单入口，以及可选的桌面快捷方式和登录自启；安装完成后可以直接打开桌面控制台。升级会保留模型配置，卸载时可选择是否同时删除模型密钥、日志和本地设置。

开发者可安装 64 位 Inno Setup 7，然后执行 `./scripts/build-installer.ps1`。脚本会先构建完整便携载荷，再输出安装程序和对应的 SHA-256 文件到 `dist-installer/`；64 位编译器可处理 LiteLLM 依赖中的超长路径。如已有刚构建的便携目录，可通过 `-PayloadDirectory` 复用，避免重复下载和打包依赖。

### 桌面版（推荐）

解压便携包后双击 `CodexChatGateway.exe` 或 `桌面版.bat`。桌面控制台提供网关启动、停止和重启，实时显示 PID、运行时长与当前模型，并集成接口检查、日志入口、Codex/Claude Desktop 配置及安全恢复。模型可手动填写，也可从兼容接口在线获取列表后选择；默认模型切换后可直接重启生效。

“登录自启”按钮会显示当前状态，并可随时启用或关闭。关闭或最小化窗口会隐藏到系统托盘，不会停止已在后台运行的网关；双击托盘图标可恢复窗口，右键菜单可启动、停止或退出桌面控制台。桌面程序采用单实例运行，重复启动只会唤起已有窗口。

源码目录可执行 `./scripts/build-desktop.ps1` 单独构建桌面程序。桌面程序使用 Windows 自带的 .NET Framework，不需要额外安装 Electron 或 WebView 运行时；WPF 界面支持高 DPI、窗口缩放和动态粒子背景。

1. 解压完整目录，不能只复制几个 `.bat` 文件。
2. 双击 `model-config.bat`（或 `模型配置.bat`）添加模型。按 API URL、Key、模型选择方式的顺序配置。
3. 双击 `start-gateway.bat`（或 `启动网关.bat`）。网关进入隐藏后台，关闭启动窗口不会停止服务。
4. 双击 `gateway-status.bat` 查看状态，或用 `check-gateway.bat` 做接口检查。
5. 双击 `configure-codex.bat`。脚本会先备份并保留现有 MCP 配置。
6. 完全退出并重新启动 Codex。

如需退出第三方网关，双击 `恢复Codex官方配置.bat`（或 `restore-official-codex.bat`），然后完全退出并重新启动 Codex。恢复脚本只撤销模型和本地 provider 设置，不会删除 MCP、插件、功能开关或其他配置。

分发包不会包含密钥。模型配置保存在本机 `.gateway/models.json`；旧版 `.env` 会在首次运行时自动迁移。

Codex 中使用的模型名是 `codex-chat`，本地地址是 `http://127.0.0.1:4000/v1`。

## Claude Desktop 的 Code 模式

这里配置的是 Claude Desktop 应用内的 Code 模式，不是普通聊天、MCP 配置，也不是独立的 Claude Code CLI。

1. 先按上面的步骤配置模型并启动网关。
2. 双击 `configure-claude-desktop.bat`（或 `配置Claude Desktop Code模式.bat`）。
3. 完全退出 Claude Desktop（包括托盘进程），再重新打开并进入 Code 模式。

脚本按 Claude Desktop 的 3P Profile 结构写入 `%LOCALAPPDATA%\Claude` 与 `%LOCALAPPDATA%\Claude-3p`，将当前默认模型映射成 Desktop 可识别的 Sonnet、Opus、Haiku 路由。上游 Key 不会写进 Claude Desktop 配置文件，Profile 中只有无权限的本地占位 Token。

恢复时双击 `restore-official-claude-desktop.bat`（或 `恢复Claude Desktop官方配置.bat`），脚本会移除本项目自己的 Profile、切回官方 `1p` 模式，并保留其他 Profile 及无关字段。之后同样需要完全重启 Claude Desktop。

模型切换仍统一通过 `model-config.bat` 完成；切换默认模型后重启网关即可，不需要重新生成 Claude Desktop Profile。Claude Desktop 只接受 `claude-sonnet-*`、`claude-opus-*`、`claude-haiku-*` 等角色路由，因此真实的 DeepSeek、Kimi 或其他 OpenAI 兼容模型 ID 只保留在本地网关中。普通 OpenAI 兼容上游会使用 LiteLLM 的 `custom_openai` Chat 适配器，DeepSeek 则使用 LiteLLM 已有的 DeepSeek Messages 适配器。

所有 `.ps1` 执行脚本均保持纯 ASCII，以兼容会把无 BOM UTF-8 误读为 ANSI 的 Windows PowerShell 5.1；中文只保留在说明文档和可选启动器文件名中。

## 模型配置与后台管理

`model-config.bat` 支持新增、删除、设置当前默认模型，以及两种模型选择方式：手动输入模型 ID，或调用标准 `GET {API URL}/models` 在线列出后选择。

DeepSeek URL 自动使用 `deepseek/模型名` 适配器，其他 OpenAI 兼容 URL 自动使用 `openai/模型名`。部分模型商不开放 `/models`，此时选择 Manual model 即可。Key 以明文保存在当前 Windows 用户可访问的本地配置文件中，请勿打包或分享 `.gateway` 目录。

`stop-gateway.bat` 停止后台服务，`enable-autostart.bat`/`disable-autostart.bat` 控制当前用户登录后自启动，不需要管理员权限。切换默认模型后需重启网关。

首次执行 `配置Codex.bat` 时会在 Codex 配置目录记录恢复状态；老版本没有恢复状态时，会从历史备份提取原模型设置。每次写入前仍会创建时间戳备份。

## 源码开发版

源码开发目录使用 `install.bat` 创建 `.venv`。精简后的可分发成品不包含安装器、测试文件或开发脚本，也不检测系统 `python` 命令；它始终运行包内的 `runtime/python.exe`。

## CI/CD 与发布

GitHub Actions 会在 Windows x64 环境中完成完整构建：下载并校验官方 CPython 3.11.9 嵌入式运行时、安装锁定到提交哈希的 LiteLLM 上游修复、运行回归测试、生成 7-Zip 便携包和品牌化 Inno Setup 安装程序，并为两种成品生成 SHA-256。

- 推送到 `main` 或创建 Pull Request：构建并上传 Actions Artifact，不创建 Release。
- 手动运行工作流：生成可下载的测试构建。
- 推送与 `VERSION` 一致的标签，例如 `v1.2.0`：自动创建或更新 GitHub Release，并上传安装版 `.exe`、便携版 `.7z` 与对应的 `.sha256`。

本地执行同一构建流程：

```powershell
.\scripts\build-portable.ps1
.\scripts\build-installer.ps1
```

贡献规范见 [CONTRIBUTING.md](CONTRIBUTING.md)，版本变化见 [CHANGELOG.md](CHANGELOG.md)。

## 安全边界

- 网关固定监听 `127.0.0.1`，不会暴露到局域网。
- `.env` 和 `.gateway` 已加入 `.gitignore`。
- Codex 只访问本地无密钥地址；上游密钥只存在于网关进程环境中。
- 配置脚本使用 TOML 解析器修改配置，写入前创建带时间戳的备份。
- Claude Desktop 配置采用独立 Profile ID、原子写入和失败回滚，不覆盖 CC Switch 或其他工具的 Profile。
- 之前已经发到聊天、日志或截图里的 Key 应立即撤销，不能复制到 `.env` 继续使用。

## 已知兼容边界

- LiteLLM 会丢弃上游不支持的可选参数，但模型本身仍需支持可靠的工具调用，Codex 才能正常完成代理任务。
- 分发版将 LiteLLM 锁定到 PR #32995 的上游提交，修复 Codex/DeepSeek 多轮工具调用中工具消息不相邻的问题；上游正式发布后可改回稳定版依赖。
- 第三方模型的工具调用格式、上下文长度和指令遵循能力可能弱于 Codex 默认模型。
- `previous_response_id` 等有状态能力由具体 LiteLLM 版本和上游能力决定；本项目主要保障 Codex 的常规流式文本和函数工具调用路径。
- 实际模型调用会消耗上游额度，健康检查不会主动生成内容。

## 依赖与来源

- [LiteLLM](https://github.com/BerriAI/litellm)，固定提交 `dfe91303a72792bce0c790ab8615b779c1c4730a`。
- [LiteLLM Responses API 文档](https://docs.litellm.ai/docs/response_api)。
- [LiteLLM DeepSeek 文档](https://docs.litellm.ai/docs/providers/deepseek)。
- [DeepSeek 官方 Anthropic API 兼容说明](https://api-docs.deepseek.com/guides/anthropic_api)。
- [LiteLLM PR #32995](https://github.com/BerriAI/litellm/pull/32995)。
- [Codex 自定义模型提供商](https://developers.openai.com/codex/config-advanced#custom-model-providers)。
- [CC Switch 的 Claude Desktop 实现](https://github.com/farion1231/cc-switch/blob/main/src-tauri/src/claude_desktop_config.rs)，用于核对 3P Profile 文件结构、模型角色和官方模式恢复行为。

本项目不复制 LiteLLM 源码，Release 构建从固定的上游 GitHub 提交安装。
