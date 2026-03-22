#!/usr/bin/env bash
# Build and install the zellij-claude plugin + Claude Code hooks.
set -euo pipefail

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
HOOK_CMD="$PROJECT_DIR/scripts/hook.sh"
INSTALL_DIR="$HOME/.config/zellij/plugins"
SETTINGS="${CLAUDE_CONFIG_DIR:-$HOME/.claude}/settings.json"

# --- Build plugin ---

if ! command -v cargo &>/dev/null; then
    echo "Rust is required. Install via: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

if ! rustup target list --installed 2>/dev/null | grep -q wasm32-wasip1; then
    echo "Adding wasm32-wasip1 target..."
    rustup target add wasm32-wasip1
fi

echo "Building zellij-claude plugin..."
cd "$PROJECT_DIR"
cargo build --release 2>&1

mkdir -p "$INSTALL_DIR"
cp "$PROJECT_DIR/target/wasm32-wasip1/release/zellij-claude.wasm" "$INSTALL_DIR/zellij-claude.wasm"
echo "Plugin installed to $INSTALL_DIR/zellij-claude.wasm"

# --- Install Claude Code hooks ---

if ! command -v jq &>/dev/null; then
    echo "error: jq is required but not found" >&2
    exit 1
fi

mkdir -p "$(dirname "$SETTINGS")"
[ -f "$SETTINGS" ] || echo '{}' > "$SETTINGS"

hooks_json=$(cat <<EOF
{
  "SessionStart":     [{"hooks": [{"type": "command", "command": "$HOOK_CMD", "async": true}]}],
  "UserPromptSubmit": [{"hooks": [{"type": "command", "command": "$HOOK_CMD", "async": true}]}],
  "PreToolUse":       [{"hooks": [{"type": "command", "command": "$HOOK_CMD", "async": true}]}],
  "Stop":             [{"hooks": [{"type": "command", "command": "$HOOK_CMD", "async": true}]}],
  "StopFailure":      [{"hooks": [{"type": "command", "command": "$HOOK_CMD", "async": true}]}],
  "Notification":     [{"matcher": "permission_prompt", "hooks": [{"type": "command", "command": "$HOOK_CMD", "async": true}]}],
  "SubagentStart":    [{"hooks": [{"type": "command", "command": "$HOOK_CMD", "async": true}]}],
  "PreCompact":       [{"hooks": [{"type": "command", "command": "$HOOK_CMD", "async": true}]}],
  "Elicitation":      [{"hooks": [{"type": "command", "command": "$HOOK_CMD", "async": true}]}],
  "SessionEnd":       [{"hooks": [{"type": "command", "command": "$HOOK_CMD"}]}]
}
EOF
)

tmp=$(mktemp)
jq --argjson new_hooks "$hooks_json" '
  .hooks //= {} |
  .hooks |= (
    to_entries | map(
      .value |= map(
        if (.hooks // []) | any(.command | test("zellij-claude")) then empty else . end
      )
    ) | from_entries
  ) |
  .hooks |= (. as $existing |
    ($new_hooks | to_entries | reduce .[] as $entry ($existing;
      .[$entry.key] = ((.[$entry.key] // []) + $entry.value)
    ))
  )
' "$SETTINGS" > "$tmp" && mv "$tmp" "$SETTINGS"

echo "Claude Code hooks installed."
echo ""
echo "Add this keybinding to your Zellij config (~/.config/zellij/config.kdl):"
echo ""
echo '    shared_among "normal" "locked" {'
echo '        bind "Ctrl Shift m" {'
echo "            LaunchOrFocusPlugin \"file:$INSTALL_DIR/zellij-claude.wasm\" {"
echo '                floating true'
echo '            }'
echo '        }'
echo '    }'
