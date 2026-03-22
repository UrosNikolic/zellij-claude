# zellij-claude

Zellij plugin for managing and switching between Claude Code sessions.

## Build

```bash
cargo build --release
```

Target is `wasm32-wasip1` (configured in `.cargo/config.toml`). Output: `target/wasm32-wasip1/release/zellij-claude.wasm`

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
