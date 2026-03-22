use serde::Deserialize;
use std::collections::BTreeMap;
use zellij_tile::prelude::*;

#[derive(Default)]
struct State {
    sessions: BTreeMap<u32, ClaudeSession>,
    selected: usize,
    sorted_pids: Vec<u32>,
    initial_scan_done: bool,
    zellij_sessions: Vec<SessionInfo>,
    filter: String,
}

#[derive(Clone, Debug, Deserialize)]
struct ClaudeSession {
    pid: u32,
    #[serde(default)]
    cwd: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    zellij_session: String,
    #[serde(default)]
    zellij_pane_id: Option<u32>,
    #[serde(skip)]
    dir_name: String,
}

register_plugin!(State);

impl ZellijPlugin for State {
    fn load(&mut self, _configuration: BTreeMap<String, String>) {
        request_permission(&[
            PermissionType::ReadApplicationState,
            PermissionType::ChangeApplicationState,
            PermissionType::RunCommands,
            PermissionType::ReadCliPipes,
        ]);

        subscribe(&[
            EventType::Key,
            EventType::Timer,
            EventType::RunCommandResult,
            EventType::SessionUpdate,
            EventType::PermissionRequestResult,
        ]);

        self.scan_status_files();
        set_timeout(2.0);
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::PermissionRequestResult(PermissionStatus::Granted) => {
                self.scan_status_files();
                false
            }
            Event::RunCommandResult(exit_code, stdout, _stderr, context) => {
                if context.get("type").map(|s| s.as_str()) == Some("scan") {
                    self.handle_scan_result(exit_code, &stdout);
                    return true;
                }
                false
            }
            Event::SessionUpdate(sessions, _) => {
                self.zellij_sessions = sessions;
                true
            }
            Event::Timer(_) => {
                self.scan_status_files();
                set_timeout(2.0);
                false
            }
            Event::Key(key) => self.handle_key(key),
            _ => false,
        }
    }

    fn pipe(&mut self, pipe_message: PipeMessage) -> bool {
        if let PipeSource::Cli(ref pipe_id) = pipe_message.source {
            unblock_cli_pipe_input(pipe_id);
        }

        if pipe_message.name == "claude_status" {
            if let Some(ref payload) = pipe_message.payload {
                if let Ok(mut session) = serde_json::from_str::<ClaudeSession>(payload) {
                    if session.status == "ended" {
                        self.sessions.remove(&session.pid);
                    } else {
                        session.dir_name = dir_name(&session.cwd);
                        self.sessions.insert(session.pid, session);
                    }
                    self.rebuild_sorted();
                    return true;
                }
            }
        }
        false
    }

    fn render(&mut self, rows: usize, cols: usize) {
        let width = cols.min(70);

        // Header
        println!("\x1b[1;36m Claude Code Sessions\x1b[0m");
        println!("{}", "\u{2500}".repeat(width));

        if self.sessions.is_empty() {
            println!();
            if self.initial_scan_done {
                println!("  No active Claude Code sessions");
            } else {
                println!("  Scanning...");
            }
            println!();
            println!("\x1b[2m  Esc to close\x1b[0m");
            return;
        }

        if !self.filter.is_empty() {
            println!("\x1b[2m Filter: {}\x1b[0m", self.filter);
        }

        let visible = self.visible_sessions();
        let max_rows = rows.saturating_sub(5);

        for (i, session) in visible.iter().enumerate().take(max_rows) {
            let icon = status_icon(&session.status);
            let is_current = self.is_current_session(&session.zellij_session);

            let session_label = if session.zellij_session.is_empty() {
                "[detached]".to_string()
            } else if is_current {
                format!("{} *", session.zellij_session)
            } else {
                session.zellij_session.clone()
            };

            let status_text = status_label(&session.status);
            let line = format!(
                " {} {:<20} [{:<16}] {}",
                icon, session.dir_name, session_label, status_text
            );

            if i == self.selected {
                println!("\x1b[7m{:<width$}\x1b[0m", line, width = width);
            } else {
                println!("{}", line);
            }
        }

        // Fill remaining space
        let used = visible.len().min(max_rows) + 3 + if self.filter.is_empty() { 0 } else { 1 };
        for _ in used..rows.saturating_sub(1) {
            println!();
        }

        println!(
            "\x1b[2m j/k: navigate  Enter: switch  Esc: close  Type to filter\x1b[0m"
        );
    }
}

impl State {
    fn scan_status_files(&self) {
        let mut context = BTreeMap::new();
        context.insert("type".to_string(), "scan".to_string());

        run_command(
            &[
                "bash",
                "-c",
                concat!(
                    "dir=\"$HOME/.claude/sessions\"; ",
                    "[ -d \"$dir\" ] || exit 0; ",
                    "for f in \"$dir\"/*.json; do ",
                    "  [ -f \"$f\" ] || continue; ",
                    "  pid=$(basename \"$f\" .json); ",
                    "  kill -0 \"$pid\" 2>/dev/null || { rm -f \"$f\" \"$dir/${pid}.status\"; continue; }; ",
                    "  status=\"idle\"; ",
                    "  sf=\"$dir/${pid}.status\"; ",
                    "  [ -f \"$sf\" ] && status=$(cat \"$sf\"); ",
                    "  zs=\"\"; zp=\"\"; ",
                    "  env_line=$(ps -E -p \"$pid\" -o command= 2>/dev/null); ",
                    "  zs=$(echo \"$env_line\" | grep -o 'ZELLIJ_SESSION_NAME=[^ ]*' | head -1 | cut -d= -f2); ",
                    "  zp=$(echo \"$env_line\" | grep -o 'ZELLIJ_PANE_ID=[^ ]*' | head -1 | cut -d= -f2); ",
                    "  if [ -z \"$zs\" ]; then ",
                    "    ppid_cur=$pid; ",
                    "    for _ in 1 2 3 4 5 6 7 8; do ",
                    "      ppid_cur=$(ps -o ppid= -p \"$ppid_cur\" 2>/dev/null | tr -d ' '); ",
                    "      [ -z \"$ppid_cur\" ] || [ \"$ppid_cur\" = \"1\" ] && break; ",
                    "      env_line=$(ps -E -p \"$ppid_cur\" -o command= 2>/dev/null); ",
                    "      zs=$(echo \"$env_line\" | grep -o 'ZELLIJ_SESSION_NAME=[^ ]*' | head -1 | cut -d= -f2); ",
                    "      zp=$(echo \"$env_line\" | grep -o 'ZELLIJ_PANE_ID=[^ ]*' | head -1 | cut -d= -f2); ",
                    "      [ -n \"$zs\" ] && break; ",
                    "    done; ",
                    "  fi; ",
                    "  jq -c --arg status \"$status\" --arg zs \"$zs\" --arg zp \"$zp\" '. + {status: $status, zellij_session: $zs} + (if $zp != \"\" then {zellij_pane_id: ($zp | tonumber)} else {} end)' \"$f\"; ",
                    "done",
                ),
            ],
            context,
        );
    }

    fn handle_scan_result(&mut self, _exit_code: Option<i32>, stdout: &[u8]) {
        self.initial_scan_done = true;
        let output = String::from_utf8_lossy(stdout);
        let mut seen_pids = std::collections::HashSet::new();

        for line in output.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Ok(mut session) = serde_json::from_str::<ClaudeSession>(line) {
                session.dir_name = dir_name(&session.cwd);
                seen_pids.insert(session.pid);
                self.sessions.insert(session.pid, session);
            }
        }

        // Remove sessions no longer in scan (dead PIDs cleaned up by the script)
        self.sessions.retain(|pid, _| seen_pids.contains(pid));
        self.rebuild_sorted();
    }

    fn handle_key(&mut self, key: KeyWithModifier) -> bool {
        if key.bare_key == BareKey::Esc
            || (key.bare_key == BareKey::Char('c') && key.has_modifiers(&[KeyModifier::Ctrl]))
        {
            hide_self();
            return false;
        }

        let len = self.visible_sessions().len();
        if len == 0 && key.bare_key != BareKey::Backspace {
            return false;
        }

        match key.bare_key {
            BareKey::Down | BareKey::Char('j') if key.key_modifiers.is_empty() => {
                if len > 0 {
                    self.selected = (self.selected + 1) % len;
                }
                true
            }
            BareKey::Up | BareKey::Char('k') if key.key_modifiers.is_empty() => {
                if len > 0 {
                    self.selected = if self.selected == 0 {
                        len - 1
                    } else {
                        self.selected - 1
                    };
                }
                true
            }
            BareKey::Enter => {
                self.switch_to_selected();
                false
            }
            BareKey::Backspace => {
                self.filter.pop();
                self.selected = 0;
                true
            }
            BareKey::Char(c) if key.key_modifiers.is_empty() && (c.is_alphanumeric() || c == '-' || c == '_') => {
                self.filter.push(c);
                self.selected = 0;
                true
            }
            _ => false,
        }
    }

    fn switch_to_selected(&self) {
        let visible = self.visible_sessions();
        if let Some(session) = visible.get(self.selected) {
            if session.zellij_session.is_empty() {
                return;
            }

            if self.is_current_session(&session.zellij_session) {
                // Same session: find the tab containing this pane and switch to it
                if let Some(pane_id) = session.zellij_pane_id {
                    if let Some(tab_pos) = self.find_tab_for_pane(pane_id) {
                        hide_self();
                        // go_to_tab is 1-indexed
                        go_to_tab(tab_pos as u32 + 1);
                        focus_terminal_pane(pane_id, false);
                        return;
                    }
                }
                hide_self();
            } else {
                // Cross-session switch: focus the correct tab and pane
                let (tab_pos, pane_id_focus) = if let Some(pane_id) = session.zellij_pane_id {
                    let tab = self.find_tab_for_pane_in_session(&session.zellij_session, pane_id);
                    (tab, Some((pane_id, false)))
                } else {
                    (None, None)
                };
                switch_session_with_focus(
                    &session.zellij_session,
                    tab_pos,
                    pane_id_focus,
                );
            }
        }
    }

    fn find_tab_for_pane(&self, pane_id: u32) -> Option<usize> {
        let current_session = self
            .zellij_sessions
            .iter()
            .find(|s| s.is_current_session)?;
        Self::find_pane_tab(&current_session.panes, pane_id)
    }

    fn find_tab_for_pane_in_session(&self, session_name: &str, pane_id: u32) -> Option<usize> {
        let session = self
            .zellij_sessions
            .iter()
            .find(|s| s.name == session_name)?;
        Self::find_pane_tab(&session.panes, pane_id)
    }

    fn find_pane_tab(panes_manifest: &PaneManifest, pane_id: u32) -> Option<usize> {
        for (tab_position, panes) in &panes_manifest.panes {
            for pane in panes {
                if !pane.is_plugin && pane.id == pane_id {
                    return Some(*tab_position);
                }
            }
        }
        None
    }

    fn is_current_session(&self, name: &str) -> bool {
        self.zellij_sessions
            .iter()
            .any(|s| s.is_current_session && s.name == name)
    }

    fn visible_sessions(&self) -> Vec<&ClaudeSession> {
        self.sorted_pids
            .iter()
            .filter_map(|pid| self.sessions.get(pid))
            .filter(|s| {
                if self.filter.is_empty() {
                    return true;
                }
                let f = self.filter.to_lowercase();
                s.dir_name.to_lowercase().contains(&f)
                    || s.zellij_session.to_lowercase().contains(&f)
                    || s.status.to_lowercase().contains(&f)
            })
            .collect()
    }

    fn rebuild_sorted(&mut self) {
        self.sorted_pids = self.sessions.keys().copied().collect();
        // Sort by session name, then directory name
        self.sorted_pids.sort_by(|a, b| {
            let sa = &self.sessions[a];
            let sb = &self.sessions[b];
            sa.zellij_session
                .cmp(&sb.zellij_session)
                .then(sa.dir_name.cmp(&sb.dir_name))
        });
        // Clamp selection
        let len = self.visible_sessions().len();
        if self.selected >= len && len > 0 {
            self.selected = len - 1;
        }
    }
}

fn dir_name(cwd: &str) -> String {
    cwd.rsplit('/').next().unwrap_or(cwd).to_string()
}

fn status_icon(status: &str) -> &'static str {
    match status {
        "starting" => "\u{1F680}",
        "thinking" => "\u{23F3}",
        "tool" => "\u{1F527}",
        "idle" => "\u{1F4A4}",
        "error" => "\u{274C}",
        "permission" => "\u{1F510}",
        "subagent" => "\u{1F527}",
        "compacting" => "\u{1F4E6}",
        "input" => "\u{2328}\u{FE0F}",
        _ => "\u{2753}",
    }
}

fn status_label(status: &str) -> &'static str {
    match status {
        "starting" => "starting",
        "thinking" => "thinking",
        "tool" => "running tool",
        "idle" => "idle",
        "error" => "error",
        "permission" => "needs permission",
        "subagent" => "subagent",
        "compacting" => "compacting",
        "input" => "needs input",
        _ => "unknown",
    }
}
