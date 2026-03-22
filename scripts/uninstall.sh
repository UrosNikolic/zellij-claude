#!/usr/bin/env bash
# Remove zellij-claude hooks and plugin.
set -euo pipefail

SETTINGS="${CLAUDE_CONFIG_DIR:-$HOME/.claude}/settings.json"
PLUGIN="$HOME/.config/zellij/plugins/zellij-claude.wasm"

# Remove hooks
if command -v jq &>/dev/null && [ -f "$SETTINGS" ]; then
    tmp=$(mktemp)
    jq '
      if .hooks then
        .hooks |= (
          to_entries | map(
            .value |= map(
              select(
                ((.hooks // []) | any(.command | test("zellij-claude"))) | not
              )
            )
          ) | map(select(.value | length > 0)) | from_entries
        ) |
        if (.hooks | length) == 0 then del(.hooks) else . end
      else .
      end
    ' "$SETTINGS" > "$tmp" && mv "$tmp" "$SETTINGS"
    echo "Hooks removed from $SETTINGS"
fi

# Remove plugin
if [ -f "$PLUGIN" ]; then
    rm -f "$PLUGIN"
    echo "Plugin removed: $PLUGIN"
fi

# Clean up session files
rm -rf ~/.claude/sessions/
echo "Session files cleaned up."
echo "Remember to remove the keybinding from ~/.config/zellij/config.kdl"
