#!/usr/bin/env bash
# Claude Code hook handler — writes status files and sends pipe messages to the Zellij plugin.

SESSIONS_DIR="$HOME/.claude/sessions"
PLUGIN_URL="file:$HOME/.config/zellij/plugins/zellij-claude.wasm"
input=$(cat)

hook_event=$(echo "$input" | jq -r '.hook_event_name // empty')
[ -z "$hook_event" ] && exit 0

# Find the Claude Code PID by walking up from our parent.
pid=""
cpid=$PPID
while [ "$cpid" -gt 1 ] 2>/dev/null; do
    if [ -f "$SESSIONS_DIR/${cpid}.json" ]; then
        pid="$cpid"
        break
    fi
    pname=$(ps -o comm= -p "$cpid" 2>/dev/null | tr -d ' ')
    if [ "$pname" = "claude" ]; then
        pid="$cpid"
        break
    fi
    cpid=$(ps -o ppid= -p "$cpid" 2>/dev/null | tr -d ' ')
done

[ -z "$pid" ] && exit 0

mkdir -p "$SESSIONS_DIR"

# Ensure the session .json file exists
session_file="$SESSIONS_DIR/${pid}.json"
if [ ! -f "$session_file" ]; then
    cwd=$(lsof -a -p "$pid" -d cwd -Fn 2>/dev/null | grep '^n' | cut -c2-)
    [ -z "$cwd" ] && cwd="$PWD"
    jq -n -c \
        --argjson pid "$pid" \
        --arg cwd "$cwd" \
        --arg zellij_session "${ZELLIJ_SESSION_NAME:-}" \
        '{pid: $pid, cwd: $cwd, zellij_session: $zellij_session}' > "$session_file"
fi

status_file="$SESSIONS_DIR/${pid}.status"

case "$hook_event" in
    SessionStart)     echo "starting"   > "$status_file" ;;
    UserPromptSubmit) echo "thinking"   > "$status_file" ;;
    PreToolUse)
        tool_name=$(echo "$input" | jq -r '.tool_name // empty')
        if [ "$tool_name" = "AskUserQuestion" ]; then
            echo "input" > "$status_file"
        else
            echo "tool" > "$status_file"
        fi
        ;;
    Stop)             echo "idle"       > "$status_file" ;;
    StopFailure)      echo "error"      > "$status_file" ;;
    Notification)
        current=$(cat "$status_file" 2>/dev/null)
        [ "$current" != "input" ] && echo "permission" > "$status_file"
        ;;
    SubagentStart)    echo "subagent"   > "$status_file" ;;
    PreCompact)       echo "compacting" > "$status_file" ;;
    Elicitation)      echo "input"      > "$status_file" ;;
    SessionEnd)
        # Send removal pipe message before cleanup
        if [ -n "$ZELLIJ" ]; then
            payload=$(jq -n -c --argjson pid "$pid" '{"pid": $pid, "status": "ended"}')
            zellij action pipe --plugin "$PLUGIN_URL" --name "claude_status" -- "$payload" 2>/dev/null &
        fi
        rm -f "$status_file" "$session_file"
        exit 0
        ;;
esac

# Send pipe message to Zellij plugin for real-time updates
if [ -n "$ZELLIJ" ]; then
    status=$(cat "$status_file" 2>/dev/null)
    cwd_val=$(jq -r '.cwd // empty' "$session_file" 2>/dev/null)
    payload=$(jq -n -c \
        --argjson pid "$pid" \
        --arg status "$status" \
        --arg cwd "$cwd_val" \
        --arg zellij_session "${ZELLIJ_SESSION_NAME:-}" \
        '{pid: $pid, status: $status, cwd: $cwd, zellij_session: $zellij_session}')
    zellij action pipe --plugin "$PLUGIN_URL" --name "claude_status" -- "$payload" 2>/dev/null &
fi
