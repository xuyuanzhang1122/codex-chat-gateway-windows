# Repository structure

```text
codex-chat-gateway/
├── desktop-tauri/       Tauri 2 + React Studio
├── native-gateway/      Standalone Rust gateway
├── installer/           Inno Studio installer
├── scripts/             Build, update, autostart, status and stop helpers
├── tests/               End-to-end native gateway tests
├── .gateway/            Local model/state data (uncommitted)
├── VERSION              Single release version source
└── CHANGELOG.md
```

The release payload contains `CodexChatGateway.exe`, `ccg-native-gateway.exe`, licenses, version metadata, and the small set of PowerShell maintenance scripts required by Studio. No BAT, C#/WPF, Python runtime, LiteLLM, or portable distribution is supported.
