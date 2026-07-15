# 参与贡献

欢迎通过 Issue 和 Pull Request 改进项目。

## 提交前检查

1. 不得提交 API Key、`.env`、`.gateway`、日志或个人 Codex 配置。
2. PowerShell 执行脚本必须保持纯 ASCII，兼容 Windows PowerShell 5.1。
3. 修改 Responses/Chat 转换路径时，必须运行工具调用和恢复配置回归测试。
4. 修改便携包内容时，确认没有加入安装器、测试目录或开发环境文件。

## 本地验证

```powershell
.\.venv\Scripts\python.exe .\tests\test_tool_output_adjacency.py
.\.venv\Scripts\python.exe .\tests\test_codex_restore.py
```

发布构建由 `scripts/build-portable.ps1` 和 GitHub Actions 完成。创建 `vX.Y.Z`
标签前，必须先同步更新 `VERSION`。
