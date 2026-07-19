## Codex Chat Gateway Studio v1.4.0

让同一个模型安全地分流到多家 API 平台或多个账号，同时尽量保住上游提示词缓存。本机桥接，密钥不出你的电脑。

### 本版亮点

- **同模型多账号分流**：同一个模型可绑定多家平台或多个账号，按权重接收新会话；支持按模型总开关、逐条上游开关和首选默认线路。
- **缓存友好的会话亲和**：不会每次请求随机换平台。同一会话会尽量固定在同一家上游，保留该平台的提示词缓存；只有失败或冷却时才故障切换。
- **失败冷却与自动切换**：某条线路请求失败后会暂时降温，后续请求自动尝试同模型的其他可用线路，避免持续撞向异常账号。
- **实时分流预览**：新增模型到上游网站的动态流量图；线路首次命中后常驻，可查看累计次数和最近时间。
- **隐私最小化**：流量图只保存模型名、上游域名和聚合计数，不记录提示词、响应正文、API Key 或请求 ID。
- **界面细节优化**：默认窗口下底部信息贴合窗口边缘，并精简项目与许可信息。

> 不同 API 平台之间无法共享彼此的缓存。v1.4.0 的策略是通过会话亲和尽量避免无意义切换，而不是声称把一家平台的缓存搬到另一家。

### 下载

| 文件 | 说明 |
|------|------|
| **`CodexChatGateway-Studio-Setup-v1.4.0.exe`** | 推荐：Studio 安装包（含嵌入式 Python 运行时，开箱即用） |
| `CodexChatGateway-Studio-Updater-*.exe` | 自动更新包，供应用内升级使用，一般无需手动下载 |
| `latest.json` | 自动更新清单 |
| `*.sha256` / `*.sig` | 校验与签名文件 |

### 快速上手

1. 安装后打开 **Codex Chat Gateway**。
2. 在「模型」中添加多个使用相同模型 ID 的上游配置。
3. 在模型分流管理区开启该模型，选择要参与分流的上游。
4. 启动网关；实际请求经过后，可在「分流预览」查看流量走向。

Codex 中使用：模型名 `codex-chat`，地址 `http://127.0.0.1:4000/v1`。

详细说明见 [README](https://github.com/xuyuanzhang1122/codex-chat-gateway-windows#readme)、[分流与缓存说明](https://github.com/xuyuanzhang1122/codex-chat-gateway-windows/blob/main/docs/MODEL_ROUTING.md) 与 [更新记录](https://github.com/xuyuanzhang1122/codex-chat-gateway-windows/blob/main/CHANGELOG.md)。

---

### English

- **Multi-account routing** for one model, with per-model and per-upstream switches plus a preferred default route.
- **Cache-aware session affinity** keeps a conversation on the same provider whenever possible; cooldown and failover move traffic only when needed.
- **Live Routing Preview** shows persistent animated model → upstream connections, hit counts, and last-used time.
- **Privacy-minimal telemetry** stores only model name, upstream domain, and aggregate counts—never prompts, responses, API keys, or request IDs.

Download **`CodexChatGateway-Studio-Setup-v1.4.0.exe`** — it bundles an embedded Python runtime and works out of the box.
