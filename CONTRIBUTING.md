# Contributing

Only the Tauri Studio and native Rust gateway are supported.

1. Put UI and desktop lifecycle changes in `desktop-tauri/`.
2. Put routing, upstream transport, and protocol work in `native-gateway/`.
3. Keep the listener fixed to `127.0.0.1` and never expose API keys through arguments, logs, frontend assets, or Codex configuration.
4. Preserve direct same-protocol passthrough. Use the shared Rust conversion library only for cross-protocol requests.
5. Do not add BAT launchers, C#/WPF projects, a Python runtime, LiteLLM, or legacy/portable release jobs.

Before submitting:

```powershell
cargo test --manifest-path native-gateway/Cargo.toml
cargo check --manifest-path desktop-tauri/src-tauri/Cargo.toml
cd desktop-tauri
npm run build
```

For release changes, update root `VERSION` and `CHANGELOG.md`; do not edit crate versions independently.
