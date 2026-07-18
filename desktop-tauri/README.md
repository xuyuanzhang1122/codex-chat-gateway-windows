# Codex Chat Gateway · Tauri Desktop

新一代 **Studio** 控制台：基于 **Tauri 2 + React + [LobeHub UI](https://ui.lobehub.com/)**，不是旧 WPF 界面的移植。

## 署名 / Credits

| | |
|---|---|
| **项目仓库** | [xuyuanzhang1122/codex-chat-gateway-windows](https://github.com/xuyuanzhang1122/codex-chat-gateway-windows) |
| **Owner** | [xuyuanzhang1122](https://github.com/xuyuanzhang1122) |
| **UI 组件库** | [LobeHub UI](https://ui.lobehub.com/) · [lobehub/lobe-ui](https://github.com/lobehub/lobe-ui) |
| **协议转换** | [LiteLLM](https://github.com/BerriAI/litellm)（本机进程） |

应用内底部 Credit 栏与「关于」数据同样展示上述署名。

## UI / 窗口

- **无边框原生窗口** + 自定义标题栏（拖拽 / 最小化 / 最大化 / 关闭）
- **LobeHub `SideNav` / `Button` / `Block` / `Modal` / `Form` / `Snippet` / `Tag` / `Alert` / `Empty` 等组件**，非原生丑陋控件
- 深色 ThemeProvider（`primaryColor: purple`）
- 左侧导航 + 分页：运行时 / 模型 / 客户端 / 日志

## 后端架构（相对 bat 堆叠）

- Rust `GatewayManager`：进程生命周期、state 持久化、健康探测、内存缓存
- **事件推送** `gateway://status|log|action`，前端不轮询阻塞
- **启动/停止异步 worker 线程**，不卡 UI
- Codex / Claude 配置仍走既有脚本（白名单），网关启停不再依赖 bat

## 性能

- 轻量 liveliness + 单 PID；全量路由校验仅在检查/启动就绪
- 切页无重挂载动画风暴；日志上限 250 行

## 开发

前置：Node 20+、Rust stable、Windows x64。

```powershell
cd desktop-tauri
npm install
npm run tauri dev
```

项目根目录由下列规则解析（含 `config.yaml` + `scripts/`）：

1. 环境变量 `CODEX_CHAT_GATEWAY_ROOT`
2. 可执行文件向上查找
3. 当前工作目录向上查找
4. `CARGO_MANIFEST_DIR` 向上（dev）

## 构建

```powershell
cd desktop-tauri
npm run tauri build
```

产物在 `desktop-tauri/src-tauri/target/release/` 与 bundle 目录。

也可从仓库根目录双击 `桌面版-Tauri.bat`（开发机需已 `npm install`）。

## 与旧桌面版关系

| | WPF (`CodexChatGateway.exe`) | Tauri（本目录） |
|---|---|---|
| 状态 | 现有分发版 | 新架构预览 |
| 网关启停 | PowerShell 脚本 | Rust 原生 + 回退扫描 |
| UI | WPF | WebView2 + React |
| 配置 Codex/Claude | 脚本 | 同脚本（白名单调用） |

旧版 bat 入口仍可用；Tauri 版推荐用 `桌面版-Tauri.bat`。
