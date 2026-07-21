# Codex Chat Gateway

本项目是 Codex Responses、OpenAI Chat Completions 与 Anthropic Messages 的本地协议路由层。

- 唯一受支持的产品形态是 `desktop-tauri/`（Tauri 2 + React）与 `native-gateway/`（Rust）。不得重新引入 BAT 启动器、C#/WPF 客户端、Python runtime 或 LiteLLM。
- 同协议请求必须原生透传；只有客户端协议与上游协议不一致时才调用可复用的 Rust 协议转换库。不要复制或移植第三方项目的协议实现。
- 网关只能监听 `127.0.0.1`，除非用户明确要求并确认网络暴露风险。
- API Key 只能来自进程环境或未提交的 `.gateway/models.json`；不得写入代码、示例、日志、前端静态资源或 Codex TOML。
- 默认模型别名为 `codex-chat`，上游模型、地址、协议与认证方式由本机 `.gateway/models.json` 控制。
- 修改 Codex 配置前必须备份；恢复时只撤销网关相关字段，保留 MCP、插件及其他设置。
- Claude Desktop Code 模式只能通过已验证的 3P Profile 文件配置；使用本项目独立 Profile ID，保留其他 Profile 和无关字段。不得修改应用程序、`app.asar` 或注入插件。
- 如果当前 Claude Desktop 版本不再支持已验证的 3P Profile 结构，应暂停并明确报告暂不支持，不得绕过应用校验。
- 升级协议转换依赖前，必须核对 Responses、Chat Completions、Anthropic Messages、SSE 流式输出及工具调用兼容性并完成冒烟测试。
- Studio 关闭时不得默认杀掉网关进程。
- Studio 安装包由 `scripts/build-tauri-installer.ps1` 构建，载荷只能包含 Tauri Studio、原生 Rust 网关和必要的 PowerShell 维护脚本。
- 应用内更新通过 HTTPS GitHub Release 的 `latest.json` 检测；下载完整 Inno Studio 安装包并校验 SHA-256 后覆盖安装。updater 私钥不得提交，使用 `TAURI_SIGNING_PRIVATE_KEY` 或 `_PATH`。
- 更新不得改写 `.gateway/models.json` 或日志中的密钥。安装器可清理历史 BAT、C#、Python 和 LiteLLM 文件，但必须保留用户配置与日志。
- 发版前更新根目录 `VERSION` 和 `CHANGELOG.md`，再推送完全匹配的 `v*.*.*` tag。`scripts/sync-versions.ps1` 负责同步两个 Rust crate 与 Tauri 配置。
- `.github/workflows/release.yml` 只发布 Studio 安装包，不再构建或发布旧版/便携版制品。
- 构建和发布脚本必须兼容 Windows PowerShell 5.1：纯 ASCII，不依赖模块自动加载，哈希等基础能力直接用 .NET 实现。
