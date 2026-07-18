# 更新记录

## Unreleased

- 新增 **Tauri 2 + React + LobeHub UI** 桌面控制台（`desktop-tauri/`）：无边框窗口、SideNav、Block/Modal/Form/Snippet 等组件，弃用旧 WPF 与原生丑控件。
- Rust `GatewayManager`：原生进程生命周期、状态缓存、异步启停 worker、`gateway://*` 事件推送，避免切页/启动卡顿。
- **关闭到托盘**：点 X / 关窗仅隐藏控制台，**不停止网关**；托盘可显示/隐藏/退出控制台。
- **Studio 安装器** `scripts/build-tauri-installer.ps1` + `installer/CodexChatGateway-Studio.iss`：深色 Inno 安装包，可选**安装前卸载旧 C# 版**。
- 应用内与 README 署名：GitHub `xuyuanzhang1122/codex-chat-gateway-windows`、LobeHub UI、LiteLLM。
- 网关启停修复：无 state 回退杀进程、路由身份校验、端口占用拒绝、运行中补写 state；恢复配置确认、改模型提示重启。
- **自动更新**：Tauri Updater + minisign；HTTPS GitHub `latest.json`；客户端页「检查更新」；启动静默探测；`scripts/build-updater-artifacts.ps1` / `generate-updater-keys.ps1`。
- 文档 `docs/RELEASE_AND_UPDATES.md`：Release 与签名更新通道说明。
- 新增品牌化深色 Windows 安装包：中英文界面、用户级安装、快捷方式/登录自启选项及可选择的数据清理卸载流程。
- 桌面控制台改为自适应 WPF 界面，新增高 DPI 支持、玻璃卡片和可交互粒子背景。
- 新增网关重启、PID/运行时长/当前模型状态、实时脚本输出与登录自启状态开关。
- 模型管理新增在线获取模型列表，并支持在桌面端完成增删改、设为默认及旧 `.env` 自动迁移。
- 增加单实例唤起、版本资源信息和桌面端存储/迁移/命名/自启冒烟测试。
- 修复模型配置窗口因 `PasswordBox` 模板类型不匹配而闪退的问题。
- 桌面端启动/重启在缺少默认模型时改为打开原生配置窗口，不再回退到旧脚本菜单。

## 1.2.0

- 新增原生 Windows 桌面控制台，统一管理网关、模型、日志及 Codex/Claude Desktop 接入。
- 新增专属桌面 Logo 与系统托盘模式，关闭或最小化窗口时继续在后台管理网关。
- 新增 Claude Desktop 内嵌 Code 模式的 3P Profile 配置入口，不修改普通聊天或 MCP 配置。
- 同一个默认模型同时提供 Codex 与 Claude Desktop 的 Sonnet/Opus/Haiku 兼容路由。
- 新增一键恢复 Claude Desktop 官方 1P 模式，保留其他 Profile 和无关字段。
- 增加 Claude Desktop 配置回归测试，并纳入便携包 CI 构建。
- 网关启动入口强制使用 UTF-8 输出，避免英文/西文 Windows 代码页因 LiteLLM 横幅而启动失败。

## 1.1.1

- 新增一键恢复 Codex 官方配置，保留 MCP、插件和无关设置。
- 增加多模型配置、模型浏览、删除和默认模型切换。
- 网关改为隐藏后台进程，增加状态、停止和登录自启动入口。
- 强制将 Codex Responses 请求桥接至上游 Chat Completions。
- 引入 LiteLLM PR #32995 工具调用相邻性补丁和回归测试。
- 精简便携包，移除安装器、测试文件和开发脚本。
- 使用 7-Zip 保留中文启动器文件名。
