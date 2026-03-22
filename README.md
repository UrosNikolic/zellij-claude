# zellij-claude

A [Zellij](https://zellij.dev) plugin for managing and switching between [Claude Code](https://docs.anthropic.com/en/docs/claude-code) sessions.

Quickly see all your running Claude Code sessions across Zellij sessions, check their status at a glance, and jump to any one with a keystroke.

## Features

- 🔍 **Live session discovery** — automatically detects all running Claude Code instances
- 🔄 **Real-time status updates** — polls every 2 seconds and accepts pipe updates
- 🔀 **Cross-session switching** — jump to any Claude session, even in other Zellij sessions
- 🔎 **Filtering** — type to filter sessions by name, directory, or status
- 🪶 **Lightweight** — single WASM binary, no external dependencies at runtime

## Status Icons

| Icon | Status | Description |
|------|--------|-------------|
| 🚀 | Starting | Claude Code is initializing |
| ⏳ | Thinking | Claude is processing your request |
| 🔧 | Running tool | Claude is executing a tool |
| 💤 | Idle | Session is waiting for input |
| ⌨️ | Needs input | Claude is waiting for user input |
| 🔐 | Needs permission | Claude needs permission to proceed |
| 🔧 | Subagent | A subagent is running |
| 📦 | Compacting | Context is being compacted |
| ❌ | Error | An error has occurred |
| ❓ | Unknown | Unrecognized status |

## Installation

### From Release

1. Download the latest `zellij-claude.wasm` from [Releases](https://github.com/firefightlabs/zellij-claude/releases)
2. Place it somewhere accessible (e.g., `~/.config/zellij/plugins/`)
3. Add a keybinding to your Zellij config (see [Configuration](#configuration))

### Build from Source

Requires [Rust](https://rustup.rs/) with the `wasm32-wasip1` target:

```bash
rustup target add wasm32-wasip1
cargo build --release
```

The compiled plugin will be at `target/wasm32-wasip1/release/zellij-claude.wasm`.

## Configuration

Add a keybinding to your Zellij config (`~/.config/zellij/config.kdl`):

```kdl
keybinds {
    shared {
        bind "Ctrl y" {
            LaunchOrFocusPlugin "file:~/.config/zellij/plugins/zellij-claude.wasm" {
                floating true
                move_to_focused_tab true
            }
        }
    }
}
```

## Keybindings

| Key | Action |
|-----|--------|
| `j` / `↓` | Move selection down |
| `k` / `↑` | Move selection up |
| `Enter` | Switch to selected session |
| `Esc` / `Ctrl+c` | Close the plugin |
| Any letter | Filter sessions |
| `Backspace` | Clear filter |

## How It Works

The plugin discovers Claude Code sessions by scanning `~/.claude/sessions/` for status files. It matches sessions to Zellij panes by inspecting environment variables of running processes. When you select a session, it switches to the correct Zellij session, tab, and pane.

Sessions can also push status updates to the plugin via Zellij's pipe mechanism:

```bash
echo '{"pid":1234,"cwd":"/path","status":"thinking"}' | zellij pipe --name claude_status
```

## Inspiration

This plugin was inspired by [tmux-claude](https://github.com/smilovanovic/tmux-claude), the tmux equivalent for managing Claude Code sessions.

## License

MIT
