# Codex Chat Gateway

本项目是 Codex Responses API 到第三方模型接口的本地适配层。

- 优先复用 LiteLLM，不自行实现 Responses/Chat Completions 协议转换。
- 网关只能监听 `127.0.0.1`，除非用户明确要求并确认网络暴露风险。
- API Key 只能来自进程环境或未提交的 `.env`，不得写入代码、示例、日志或 Codex TOML。
- 修改 Codex 配置前必须备份；保留既有 MCP、插件及其他设置。
- 默认模型别名为 `codex-chat`，上游模型和地址由本机 `.gateway/models.json` 控制。
- 恢复官方 Codex 配置时只撤销网关相关字段，保留 MCP、插件及其他设置。
- 升级 LiteLLM 前核对 `/responses`、流式输出及工具调用兼容性并完成冒烟测试。
