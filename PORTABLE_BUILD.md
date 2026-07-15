# Portable build provenance

The Windows x64 portable distribution is built from:

- CPython 3.11.9 official embedded x64 distribution.
- LiteLLM pinned to upstream commit `dfe91303a72792bce0c790ab8615b779c1c4730a`
  from PR #32995, with proxy dependencies.
- tomlkit 0.13.3 for safe Codex TOML updates.

The package contains its own `runtime/python.exe`. Runtime launchers never search for or invoke a system `python`, `py`, Docker, Git, or Codex CLI executable.

Before release, the portable runtime is tested against a local Chat Completions mock for:

1. `/v1/responses` non-streaming text conversion.
2. Function tool-call conversion.
3. SSE `response.output_text.delta` and `response.completed` events.
4. Windows PowerShell 5.1 parsing of every `.ps1` file.
5. Hidden API-key capture, multi-profile add/delete/default selection, and `/models` browsing.
6. Codex TOML parsing, backup creation, and preservation of existing MCP configuration.
7. Background start/status/stop behavior after the launcher exits.
8. The LiteLLM PR #32995 tool-call adjacency regression test.
9. One-click Codex official-configuration restore while preserving unrelated settings.

`scripts/build-portable.ps1` is the single local and CI build entrypoint. A matching
`vX.Y.Z` tag causes GitHub Actions to publish the generated `.7z` and `.sha256` files.
