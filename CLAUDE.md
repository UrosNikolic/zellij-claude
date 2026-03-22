# zellij-claude

Zellij plugin for managing and switching between Claude Code sessions.

## Build & Release

```bash
make build           # build locally
make release         # build, commit, tag (patch bump), push — triggers GitHub Actions
make release-minor   # bump minor version
make release-major   # bump major version
```

Target is `wasm32-wasip1` (configured in `.cargo/config.toml`). Output: `target/wasm32-wasip1/release/zellij-claude.wasm`

GitHub Actions builds the WASM and attaches it to the release on tag push.

## Architecture

Single-file Zellij WASM plugin (`src/main.rs`) using `zellij-tile` 0.43. The plugin:

- Discovers Claude Code sessions by scanning `~/.claude/sessions/*.json`
- Matches sessions to Zellij panes via process environment variables
- Accepts real-time status updates via Zellij pipes (`claude_status`)
- Polls every 2 seconds via timer

## Testing locally

Load the plugin in Zellij with:

```kdl
LaunchOrFocusPlugin "file:target/wasm32-wasip1/release/zellij-claude.wasm" {
    floating true
}
```
