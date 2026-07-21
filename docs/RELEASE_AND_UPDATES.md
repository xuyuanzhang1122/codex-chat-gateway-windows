# Release and updates

There is one Windows product and one release artifact:

```powershell
.\scripts\build-tauri-installer.ps1
```

This creates `dist-installer\CodexChatGateway-Studio-Setup-v<VERSION>.exe` containing Tauri Studio and the native Rust gateway.

## Release flow

1. Update root `VERSION` and add the matching `CHANGELOG.md` section.
2. Run `scripts/sync-versions.ps1` or build the installer, which runs it automatically.
3. Push a tag exactly matching `v<VERSION>`.
4. `.github/workflows/release.yml` builds and publishes the Studio installer, checksum, signature, and `latest.json`.

Prerelease identifiers such as `2.1.0-beta.1` are supported.

## Update guarantees

- The updater downloads the complete Inno Studio installer over HTTPS and verifies its signature/checksum.
- `.gateway/models.json`, API keys, `.env`, and logs are preserved.
- Historical BAT, C#/WPF, Python and LiteLLM files may be removed during upgrade.
- Closing or updating Studio does not stop an independently running gateway unless explicitly requested.

No legacy installer, portable package, or bare Tauri updater bundle is produced.
