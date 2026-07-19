# 更新记录

## Unreleased

## 1.4.1 - 2026-07-19

- 修复安装版启动网关即退：`run_gateway.py` 现在显式把脚本目录加入 `sys.path`，不再依赖嵌入式 Python `._pth` 的搜索路径。
- 修复控制台「打开日志目录」等按钮报 `plugin:opener|open_path not allowed by ACL`：capabilities 补 `opener:allow-open-path`。

## 1.4.0 - 2026-07-19

- 新增同模型多账号加权分流：`models.json` v3、会话/缓存亲和、失败冷却与同模型账号故障切换。
- Studio 模型页按模型分组管理分流规则，可分别开关每个模型及其绑定上游；旧 PowerShell/WPF 入口可无损保留 v3 配置。
- 新增「分流预览」动态图：基于 React Flow 展示模型到上游网站的真实选路，首次命中后线路常驻，并持续累计命中次数与最近时间。
- LiteLLM 路由回调只持久化模型、上游域名和聚合次数，不记录提示词、响应正文、API Key 或请求标识。
- 优化默认窗口布局并精简底部项目/许可信息，内容始终贴合窗口底边。

## 1.3.0

- **Studio 控制台**（`desktop-tauri/`）：Tauri 2 + React + LobeHub；无边框、托盘关窗不杀网关；去掉多余启动闪屏。
- Rust `GatewayManager`：进程生命周期、多进程停止（taskkill 树）、事件推送。
- **Studio 安装器**与 **自动更新**（GitHub `latest.json` + minisign）。
- 模型 **api.txt 导入**；托盘状态与窗口可见性同步。
- 仓库整理：启动器迁入 `bin/`，示例进 `examples/`，文档进 `docs/`；README 与 Studio 截图更新。
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
