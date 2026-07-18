# Release 构建与自动更新

## 当前产物

| 产物 | 命令 | 说明 |
|---|---|---|
| **Studio 安装包**（推荐） | `.\构建Studio安装器.bat` 或 `.\scripts\build-tauri-installer.ps1` | **Tauri + LobeHub** 控制台 + LiteLLM runtime，可选卸载旧 C# 版 |
| **Studio 自动更新包** | `.\scripts\build-updater-artifacts.ps1` | 签名 NSIS zip + `latest.json`（应用内检查更新） |
| 旧便携包 / 旧安装包 | `build-portable.ps1` / `build-installer.ps1` | **仅遗留 C#/WPF**，不要当 Studio 用 |

### 如何确认是 Studio 而不是旧版

| 检查 | Studio（正确） | 旧 WPF（错误） |
|---|---|---|
| 安装包文件名 | `CodexChatGateway-**Studio**-Setup-v1.3.0.exe` | `CodexChatGateway-Setup-v1.2.0.exe` |
| 主程序体积 | 约 **10MB+** | 约 **100–200KB** |
| 启动界面 | 深色 LobeHub / 侧栏 / 进入控制台 | 旧粒子 WPF 控制台 |
| 目录标记 | 安装目录有 `STUDIO` 文件 | 无 |

**不要**运行 / 安装：

- `dist-installer\portable-bootstrap\...`（仅为中间产物；其中的 exe 可能是旧 WPF）
- `dist-installer\portable\...`
- 根目录旧的 `CodexChatGateway.exe`（若由 `build-desktop.ps1` 生成）

安装器输出：

```text
dist-installer/studio-payload-vX.Y.Z/     ← 解包内容（主程序=Tauri）
dist-installer/CodexChatGateway-Studio-Setup-vX.Y.Z.exe
dist-installer/CodexChatGateway-Studio-Setup-vX.Y.Z.exe.sha256
```

### 安装器能力

- 用户级安装（`PrivilegesRequired=lowest`），默认目录 `%LOCALAPPDATA%\Programs\Codex Chat Gateway`
- 深色 modern Wizard（与 Studio 深色品牌一致；Inno 无法做真正 Acrylic 毛玻璃）
- **任务勾选：安装前卸载 / 删除旧版 C# 桌面程序**
  - 停止 `CodexChatGateway.exe` / `run_gateway.py`
  - 调用旧 Inno 卸载字符串（同一 AppId）
  - 清理遗留 exe
- **保留** `.gateway`（模型密钥）与日志，除非卸载时选择清除

### 应用行为

- 关闭窗口 / 点 X → **隐藏到托盘**，**不停止网关**
- 托盘菜单：显示控制台 / 隐藏 / 退出控制台（网关继续）
- 停止网关：控制台内「停止」或卸载脚本

## GitHub Actions（推荐）

流水线：`.github/workflows/release.yml`

| 触发 | 结果 |
|------|------|
| `push` / `PR` → `main` | 构建 **Studio 安装包**，上传 Actions Artifact；遗留便携包 job 失败不阻断 |
| 推送标签 `vX.Y.Z`（须与 `VERSION` 一致） | 创建/更新 GitHub Release 并上传 Studio 产物 |
| 仓库 Secret `TAURI_SIGNING_PRIVATE_KEY` 已配置 | 额外构建签名更新包 + `latest.json` 并挂到 Release |

发布者只需：

```powershell
# 1) VERSION / tauri.conf.json / package.json 已对齐
git push origin main
git tag v1.3.0
git push origin v1.3.0
# 2) 在 Actions 页查看 "Build Studio release"
```

可选 Secret（Settings → Secrets and variables → Actions）：

| Secret | 说明 |
|--------|------|
| `TAURI_SIGNING_PRIVATE_KEY` | minisign 私钥全文（与 `tauri.conf.json` 公钥配对） |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | 私钥密码（若有） |

## 本机构建（可选，非必须）

```powershell
.\scripts\build-tauri-installer.ps1
$env:TAURI_SIGNING_PRIVATE_KEY_PATH = "$env:USERPROFILE\.codex-chat-gateway\tauri-updater.key"
.\scripts\build-updater-artifacts.ps1
```

Release 附件：

| 文件 | 用途 |
|---|---|
| `CodexChatGateway-Studio-Setup-vX.Y.Z.exe` | 完整安装 |
| 同名 `.sha256` | 校验 |
| `CodexChatGateway-Studio-Updater-vX.Y.Z-windows-x86_64.nsis.zip` | 应用内更新载荷 |
| 同名 `.sig` / `.sha256` | 签名与校验 |
| **`latest.json`** | 更新清单（文件名固定，挂在 latest Release） |

`latest.json` 由构建脚本生成，示意：

```json
{
  "version": "1.3.1",
  "notes": "…",
  "pub_date": "2026-07-18T12:00:00Z",
  "platforms": {
    "windows-x86_64": {
      "signature": "<minisign signature>",
      "url": "https://github.com/xuyuanzhang1122/codex-chat-gateway-windows/releases/download/v1.3.1/CodexChatGateway-Studio-Updater-v1.3.1-windows-x86_64.nsis.zip"
    }
  }
}
```

应用固定读取：

```text
https://github.com/xuyuanzhang1122/codex-chat-gateway-windows/releases/latest/download/latest.json
```

因此 **每个最新 Release 都必须包含名为 `latest.json` 的附件**（覆盖上传即可）。

## 自动更新（已实现）

基于 **Tauri Updater 2** + minisign：

| 环节 | 说明 |
|---|---|
| 通道 | 仅 HTTPS GitHub Releases |
| 公钥 | `desktop-tauri/src-tauri/tauri.conf.json` → `plugins.updater.pubkey`（可提交） |
| 私钥 | 本机 `%USERPROFILE%\.codex-chat-gateway\tauri-updater.key` 或 CI Secret `TAURI_SIGNING_PRIVATE_KEY`（**禁止提交**） |
| 控制台入口 | 「客户端」页 → **检查更新** |
| 启动 | 静默检查一次；有新版本仅写日志，不自动下载 |
| 用户数据 | **不修改** `.gateway/models.json` 与 API Key |
| 网关进程 | 更新控制台不会默认停止网关 |

### 首次生成密钥

```powershell
.\scripts\generate-updater-keys.ps1
```

将打印的 **公钥** 写入 `tauri.conf.json` 的 `plugins.updater.pubkey`（若与仓库内已有公钥不同，则所有已发布客户端将无法验证新签名，一般不要轮换）。

### 约束（与 AGENTS.md 一致）

- 更新通道仅 HTTPS GitHub Releases  
- 不在更新日志 / notes 中写入 API Key  
- 更新后默认不改 `.gateway/models.json`  
- 签名私钥不得进入仓库、示例或日志  

### 开发机注意

`tauri dev` 通常无法完整走安装式更新；请用 **安装版 / NSIS 包** 验证「检查更新」。无网络或尚未上传 `latest.json` 时，检查会提示不可用或已是最新。
