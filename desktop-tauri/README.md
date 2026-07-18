# Codex Chat Gateway · Tauri Desktop

新一代桌面控制台，基于 **Tauri 2 + React**，视觉风格参考 [Mineradio](https://mineradio.art/)（深色星场粒子、玻璃卡片、薄荷高亮与启动入场）。

相对旧版 WPF（`desktop/GatewayDesktop.cs`），本客户端在 Rust 侧重写了网关生命周期，并修掉旧架构中的若干问题：

- 无 `state.json` 时仍可按进程命令行停止本项目网关
- 健康检查 + `/v1/models` 身份校验（必须含 `codex-chat`）
- 端口被占用但不是本网关时拒绝启动
- 已在运行时补写 / 同步 `state.json`
- 恢复 Codex / Claude 前二次确认
- 修改默认或当前模型后提示是否立即重启
- 剪贴板失败不会崩 UI

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
