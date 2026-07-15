# Codex Chat Gateway

[![Build portable release](https://github.com/xuyuanzhang1122/codex-chat-gateway-windows/actions/workflows/release.yml/badge.svg)](https://github.com/xuyuanzhang1122/codex-chat-gateway-windows/actions/workflows/release.yml)
[![GitHub Release](https://img.shields.io/github/v/release/xuyuanzhang1122/codex-chat-gateway-windows)](https://github.com/xuyuanzhang1122/codex-chat-gateway-windows/releases)

把只支持 Chat Completions 的第三方模型接入 Codex。Codex 请求本机的 `/v1/responses`，LiteLLM 负责转换成上游模型商支持的协议。

> 本项目是社区兼容工具，不是 OpenAI 官方项目。

## 为什么不自己实现转换器

Responses API 不只是字段改名，还涉及 SSE 流式事件、工具调用、错误映射、参数兼容和多轮上下文。本项目直接复用 LiteLLM，只负责 Windows 一键安装、密钥隔离、Codex TOML 安全写入和健康检查。

## 便携分发版：空白 Windows 直接使用

便携版目录中包含 `runtime/python.exe` 和全部已验证依赖，不要求目标电脑安装 Python、Docker、Git、Codex CLI 或联网下载运行库。目标系统为 Windows x64，并需要能够访问上游模型 API。

1. 解压完整目录，不能只复制几个 `.bat` 文件。
2. 双击 `model-config.bat`（或 `模型配置.bat`）添加模型。按 API URL、Key、模型选择方式的顺序配置。
3. 双击 `start-gateway.bat`（或 `启动网关.bat`）。网关进入隐藏后台，关闭启动窗口不会停止服务。
4. 双击 `gateway-status.bat` 查看状态，或用 `check-gateway.bat` 做接口检查。
5. 双击 `configure-codex.bat`。脚本会先备份并保留现有 MCP 配置。
6. 完全退出并重新启动 Codex。

如需退出第三方网关，双击 `恢复Codex官方配置.bat`（或 `restore-official-codex.bat`），然后完全退出并重新启动 Codex。恢复脚本只撤销模型和本地 provider 设置，不会删除 MCP、插件、功能开关或其他配置。

分发包不会包含密钥。模型配置保存在本机 `.gateway/models.json`；旧版 `.env` 会在首次运行时自动迁移。

Codex 中使用的模型名是 `codex-chat`，本地地址是 `http://127.0.0.1:4000/v1`。

所有 `.ps1` 执行脚本均保持纯 ASCII，以兼容会把无 BOM UTF-8 误读为 ANSI 的 Windows PowerShell 5.1；中文只保留在说明文档和可选启动器文件名中。

## 模型配置与后台管理

`model-config.bat` 支持新增、删除、设置当前默认模型，以及两种模型选择方式：手动输入模型 ID，或调用标准 `GET {API URL}/models` 在线列出后选择。

DeepSeek URL 自动使用 `deepseek/模型名` 适配器，其他 OpenAI 兼容 URL 自动使用 `openai/模型名`。部分模型商不开放 `/models`，此时选择 Manual model 即可。Key 以明文保存在当前 Windows 用户可访问的本地配置文件中，请勿打包或分享 `.gateway` 目录。

`stop-gateway.bat` 停止后台服务，`enable-autostart.bat`/`disable-autostart.bat` 控制当前用户登录后自启动，不需要管理员权限。切换默认模型后需重启网关。

首次执行 `配置Codex.bat` 时会在 Codex 配置目录记录恢复状态；老版本没有恢复状态时，会从历史备份提取原模型设置。每次写入前仍会创建时间戳备份。

## 源码开发版

源码开发目录使用 `install.bat` 创建 `.venv`。精简后的可分发成品不包含安装器、测试文件或开发脚本，也不检测系统 `python` 命令；它始终运行包内的 `runtime/python.exe`。

## CI/CD 与发布

GitHub Actions 会在 Windows x64 环境中完成完整构建：下载并校验官方 CPython 3.11.9 嵌入式运行时、安装锁定到提交哈希的 LiteLLM 上游修复、运行回归测试、用 7-Zip 打包并生成 SHA-256。

- 推送到 `main` 或创建 Pull Request：构建并上传 Actions Artifact，不创建 Release。
- 手动运行工作流：生成可下载的测试构建。
- 推送与 `VERSION` 一致的标签，例如 `v1.1.1`：自动创建或更新 GitHub Release，并上传 `.7z` 与 `.sha256`。

本地执行同一构建流程：

```powershell
.\scripts\build-portable.ps1
```

贡献规范见 [CONTRIBUTING.md](CONTRIBUTING.md)，版本变化见 [CHANGELOG.md](CHANGELOG.md)。

## 安全边界

- 网关固定监听 `127.0.0.1`，不会暴露到局域网。
- `.env` 和 `.gateway` 已加入 `.gitignore`。
- Codex 只访问本地无密钥地址；上游密钥只存在于网关进程环境中。
- 配置脚本使用 TOML 解析器修改配置，写入前创建带时间戳的备份。
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
- [LiteLLM PR #32995](https://github.com/BerriAI/litellm/pull/32995)。
- [Codex 自定义模型提供商](https://developers.openai.com/codex/config-advanced#custom-model-providers)。

本项目不复制 LiteLLM 源码，Release 构建从固定的上游 GitHub 提交安装。
