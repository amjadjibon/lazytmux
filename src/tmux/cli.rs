use super::client::TmuxClient;
use super::parser::{assemble_tree, parse_panes, parse_sessions, parse_windows};
use crate::domain::{Pane, PaneId, Session, SessionId, Window, WindowId};
use anyhow::{Context, Result, anyhow};
use std::process::Command;

#[derive(Debug, Clone, Default)]
pub struct CliTmuxClient;

impl CliTmuxClient {
    pub fn new() -> Self {
        Self
    }

    fn run_cmd(&self, args: &[&str]) -> Result<String> {
        let output = Command::new("tmux")
            .args(args)
            .output()
            .with_context(|| format!("Failed to execute tmux command: tmux {}", args.join(" ")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("tmux error: {}", stderr.trim()));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    fn run_cmd_raw(&self, args: &[&str]) -> Result<Vec<u8>> {
        let output = Command::new("tmux")
            .args(args)
            .output()
            .with_context(|| format!("Failed to execute tmux command: tmux {}", args.join(" ")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("tmux error: {}", stderr.trim()));
        }

        Ok(output.stdout)
    }
}

impl TmuxClient for CliTmuxClient {
    fn list_sessions(&self) -> Result<Vec<Session>> {
        let fmt = "#{session_id}\t#{session_name}\t#{session_windows}\t#{session_attached}";
        let output = self.run_cmd(&["list-sessions", "-F", fmt])?;
        Ok(parse_sessions(&output))
    }

    fn list_windows(&self, session: &SessionId) -> Result<Vec<Window>> {
        let fmt = "#{session_id}\t#{window_id}\t#{window_index}\t#{window_name}\t#{window_active}\t#{window_panes}\t#{window_layout}";
        let output = self.run_cmd(&["list-windows", "-t", &session.0, "-F", fmt])?;
        let windows = parse_windows(&output).into_iter().map(|(_, w)| w).collect();
        Ok(windows)
    }

    fn list_panes(&self, window: &WindowId) -> Result<Vec<Pane>> {
        let fmt = "#{session_id}\t#{window_id}\t#{pane_id}\t#{pane_index}\t#{pane_active}\t#{pane_current_command}\t#{pane_current_path}\t#{pane_width}\t#{pane_height}";
        let output = self.run_cmd(&["list-panes", "-t", &window.0, "-F", fmt])?;
        let panes = parse_panes(&output)
            .into_iter()
            .map(|(_, _, p)| p)
            .collect();
        Ok(panes)
    }

    fn fetch_full_tree(&self) -> Result<Vec<Session>> {
        let sess_fmt = "#{session_id}\t#{session_name}\t#{session_windows}\t#{session_attached}";
        let sess_out = match self.run_cmd(&["list-sessions", "-F", sess_fmt]) {
            Ok(out) => out,
            Err(e) => {
                // If no server running, return empty list
                if e.to_string().contains("no server running")
                    || e.to_string().contains("error connecting to")
                {
                    return Ok(Vec::new());
                }
                return Err(e);
            }
        };
        let sessions = parse_sessions(&sess_out);
        if sessions.is_empty() {
            return Ok(Vec::new());
        }

        let win_fmt = "#{session_id}\t#{window_id}\t#{window_index}\t#{window_name}\t#{window_active}\t#{window_panes}\t#{window_layout}";
        let win_out = self.run_cmd(&["list-windows", "-a", "-F", win_fmt])?;
        let windows = parse_windows(&win_out);

        let pane_fmt = "#{session_id}\t#{window_id}\t#{pane_id}\t#{pane_index}\t#{pane_active}\t#{pane_current_command}\t#{pane_current_path}\t#{pane_width}\t#{pane_height}";
        let pane_out = self.run_cmd(&["list-panes", "-a", "-F", pane_fmt])?;
        let panes = parse_panes(&pane_out);

        Ok(assemble_tree(sessions, windows, panes))
    }

    fn capture_pane(&self, pane: &PaneId, lines: usize, preserve_ansi: bool) -> Result<Vec<u8>> {
        let lines_arg = format!("-{}", lines);
        let mut args = vec!["capture-pane", "-p", "-t", &pane.0, "-S", &lines_arg];
        if preserve_ansi {
            args.insert(1, "-e");
        }
        self.run_cmd_raw(&args)
    }

    fn create_session(&mut self, name: &str) -> Result<SessionId> {
        let out = self.run_cmd(&["new-session", "-d", "-s", name, "-P", "-F", "#{session_id}"])?;
        Ok(SessionId::from(out.trim()))
    }

    fn rename_session(&mut self, session: &SessionId, new_name: &str) -> Result<()> {
        self.run_cmd(&["rename-session", "-t", &session.0, new_name])?;
        Ok(())
    }

    fn kill_session(&mut self, session: &SessionId) -> Result<()> {
        self.run_cmd(&["kill-session", "-t", &session.0])?;
        Ok(())
    }

    fn create_window(&mut self, session: &SessionId, name: &str) -> Result<WindowId> {
        let out = self.run_cmd(&[
            "new-window",
            "-t",
            &session.0,
            "-n",
            name,
            "-P",
            "-F",
            "#{window_id}",
        ])?;
        Ok(WindowId::from(out.trim()))
    }

    fn rename_window(&mut self, window: &WindowId, new_name: &str) -> Result<()> {
        self.run_cmd(&["rename-window", "-t", &window.0, new_name])?;
        Ok(())
    }

    fn kill_window(&mut self, window: &WindowId) -> Result<()> {
        self.run_cmd(&["kill-window", "-t", &window.0])?;
        Ok(())
    }

    fn kill_pane(&mut self, pane: &PaneId) -> Result<()> {
        self.run_cmd(&["kill-pane", "-t", &pane.0])?;
        Ok(())
    }

    fn zoom_pane(&mut self, pane: &PaneId) -> Result<()> {
        self.run_cmd(&["resize-pane", "-Z", "-t", &pane.0])?;
        Ok(())
    }

    fn split_pane(&mut self, pane: &PaneId, vertical: bool) -> Result<PaneId> {
        let flag = if vertical { "-h" } else { "-v" };
        let out = self.run_cmd(&[
            "split-window",
            flag,
            "-t",
            &pane.0,
            "-P",
            "-F",
            "#{pane_id}",
        ])?;
        Ok(PaneId::from(out.trim()))
    }

    fn select_layout(&mut self, window: &WindowId, layout: &str) -> Result<()> {
        self.run_cmd(&["select-layout", "-t", &window.0, layout])?;
        Ok(())
    }

    fn toggle_sync_panes(&mut self, window: &WindowId) -> Result<bool> {
        let current = self
            .run_cmd(&[
                "show-window-options",
                "-t",
                &window.0,
                "-v",
                "synchronize-panes",
            ])
            .unwrap_or_default();
        let new_state = if current.trim() == "on" { "off" } else { "on" };
        self.run_cmd(&[
            "set-window-option",
            "-t",
            &window.0,
            "synchronize-panes",
            new_state,
        ])?;
        Ok(new_state == "on")
    }

    fn swap_pane(&mut self, pane: &PaneId, up: bool) -> Result<()> {
        let flag = if up { "-U" } else { "-D" };
        self.run_cmd(&["swap-pane", flag, "-t", &pane.0])?;
        Ok(())
    }

    fn swap_window(&mut self, window: &WindowId, left: bool) -> Result<()> {
        let target = if left { "-1" } else { "+1" };
        self.run_cmd(&["swap-window", "-d", "-s", &window.0, "-t", target])?;
        Ok(())
    }

    fn respawn_pane(&mut self, pane: &PaneId) -> Result<()> {
        self.run_cmd(&["respawn-pane", "-k", "-t", &pane.0])?;
        Ok(())
    }

    fn send_keys(&mut self, pane: &PaneId, keys: &str) -> Result<()> {
        if !keys.is_empty() {
            self.run_cmd(&["send-keys", "-t", &pane.0, "-l", "--", keys])?;
        }
        self.run_cmd(&["send-keys", "-t", &pane.0, "Enter"])?;
        Ok(())
    }

    fn break_pane(&mut self, pane: &PaneId) -> Result<()> {
        self.run_cmd(&["break-pane", "-d", "-t", &pane.0])?;
        Ok(())
    }

    fn resize_pane(
        &mut self,
        pane: &PaneId,
        direction: crate::tmux::client::ResizeDirection,
        amount: usize,
    ) -> Result<()> {
        let dir_flag = match direction {
            crate::tmux::client::ResizeDirection::Up => "-U",
            crate::tmux::client::ResizeDirection::Down => "-D",
            crate::tmux::client::ResizeDirection::Left => "-L",
            crate::tmux::client::ResizeDirection::Right => "-R",
        };
        let amt_str = amount.to_string();
        self.run_cmd(&["resize-pane", "-t", &pane.0, dir_flag, &amt_str])?;
        Ok(())
    }

    fn focus_pane(&self, session: &SessionId, window: &WindowId, pane: &PaneId) -> Result<()> {
        // Run select commands
        let _ = self.run_cmd(&["select-window", "-t", &window.0]);
        let _ = self.run_cmd(&["select-pane", "-t", &pane.0]);
        if std::env::var("TMUX").is_ok() {
            let _ = self.run_cmd(&["switch-client", "-t", &session.0]);
        }
        Ok(())
    }
}
