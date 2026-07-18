# Release 构建与自动更新

## 当前产物

| 产物 | 命令 | 说明 |
|---|---|---|
| **Studio 安装包**（推荐） | `.\scripts\build-tauri-installer.ps1` | Tauri 控制台 + LiteLLM runtime，Inno 深色安装器，可选卸载旧 C# 版 |
| 旧便携包 / 旧安装包 | `build-portable.ps1` / `build-installer.ps1` | 仍面向 WPF 控制台 |

安装器输出：

```text
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

## GitHub Release 建议流程

1. 更新 `VERSION`（与 `desktop-tauri/src-tauri/tauri.conf.json` 一致）
2. `git tag vX.Y.Z && git push origin vX.Y.Z`
3. CI 或本机：

```powershell
.\scripts\build-tauri-installer.ps1
```

4. 在 [Releases](https://github.com/xuyuanzhang1122/codex-chat-gateway-windows/releases) 上传：
   - `CodexChatGateway-Studio-Setup-vX.Y.Z.exe`
   - 同名 `.sha256`

## 自动更新（下一阶段）

计划使用 **Tauri Updater**：

1. `cargo tauri signer generate` 生成密钥对  
2. 在 `tauri.conf.json` → `plugins.updater` 填入 `pubkey` 与 endpoints  
3. Release 附带 `latest.json`（version、platforms、url、signature）  
4. 控制台「检查更新」：下载 → 校验签名 → 静默替换 → 提示重启控制台  

**约束（与 AGENTS.md 一致）**：

- 更新通道仅 HTTPS GitHub Releases  
- 不在更新日志中写入 API Key  
- 更新后默认不改 `.gateway/models.json`  

当前仓库已预留文档位置；`pubkey` 未写入配置，避免空公钥导致运行失败。
