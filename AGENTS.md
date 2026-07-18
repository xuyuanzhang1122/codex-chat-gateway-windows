# Codex Chat Gateway

本项目是 Codex Responses API 到第三方模型接口的本地适配层。

- 优先复用 LiteLLM，不自行实现 Responses/Chat Completions 协议转换。
- 网关只能监听 `127.0.0.1`，除非用户明确要求并确认网络暴露风险。
- API Key 只能来自进程环境或未提交的 `.env`，不得写入代码、示例、日志或 Codex TOML。
- 修改 Codex 配置前必须备份；保留既有 MCP、插件及其他设置。
- 默认模型别名为 `codex-chat`，上游模型和地址由本机 `.gateway/models.json` 控制。
- 恢复官方 Codex 配置时只撤销网关相关字段，保留 MCP、插件及其他设置。
- Claude Desktop Code 模式只能通过其 3P Profile 文件配置；不得把普通聊天或 MCP 配置冒充模型配置，也不得修改应用程序、`app.asar` 或注入插件来扩大控制权。
- Claude Desktop 配置必须使用本项目独立 Profile ID，保留其他 Profile 和无关字段；恢复时只移除本项目 Profile 并切回官方 `1p` 模式。
- 如果当前 Claude Desktop 版本不再支持已验证的 3P Profile 结构，应暂停并明确报告暂不支持，不得绕过应用校验。
- 升级 LiteLLM 前核对 `/responses`、流式输出及工具调用兼容性并完成冒烟测试。
- 新桌面控制台优先维护 `desktop-tauri/`（Tauri 2 + React）；网关仍只监听 `127.0.0.1`，密钥不得写入前端静态资源或客户端配置。
- 旧版 WPF（`desktop/GatewayDesktop.cs` / `CodexChatGateway.exe`）在 Tauri 版稳定前可并存，勿在未确认分发策略前删除。
- Studio 安装包由 `scripts/build-tauri-installer.ps1` 构建；关闭控制台不得默认杀掉网关进程。
- 自动更新仅允许 HTTPS GitHub Release 通道（`latest.json` + 签名 NSIS 包）；公钥可进仓库，**私钥不得提交**（用 `TAURI_SIGNING_PRIVATE_KEY` / `_PATH`）。
- 更新不得改写 `.gateway/models.json` 或日志中的密钥；更新控制台不得默认杀掉网关进程。
