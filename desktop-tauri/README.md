# Codex Chat Gateway Studio

The only supported desktop console, built with Tauri 2 and React. It manages models, launches the standalone native Rust gateway, configures supported clients, and handles full Studio installer updates.

```powershell
npm install
npm run tauri dev
```

Production builds are created from the repository root with `scripts/build-tauri-installer.ps1`. The installer bundles the release gateway executable; the desktop application does not start or depend on Python, LiteLLM, BAT, or C#/WPF components.

The gateway listens only on `127.0.0.1`. API keys stay in the local model store and must never be embedded in frontend assets or command-line arguments.
