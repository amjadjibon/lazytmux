use crate::domain::{PaneId, SessionId, WindowId};
use anyhow::Result;
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TmuxEnvironment {
    OutsideTmux,
    InsideTmux { current_pane: String },
    PopupMode,
}

pub fn detect_environment() -> TmuxEnvironment {
    if std::env::var("TMUX_POPUP").is_ok() {
        TmuxEnvironment::PopupMode
    } else if std::env::var("TMUX").is_ok() {
        TmuxEnvironment::InsideTmux {
            current_pane: std::env::var("TMUX_PANE").unwrap_or_default(),
        }
    } else {
        TmuxEnvironment::OutsideTmux
    }
}

pub fn execute_handoff(
    session_id: &SessionId,
    session_name: &str,
    window_id: &WindowId,
    pane_id: &PaneId,
    is_mock: bool,
) -> Result<()> {
    if is_mock {
        return Ok(());
    }

    let env = detect_environment();

    match env {
        TmuxEnvironment::PopupMode => {
            let _ = Command::new("tmux")
                .args(["select-window", "-t", &window_id.0])
                .status();
            let _ = Command::new("tmux")
                .args(["select-pane", "-t", &pane_id.0])
                .status();
            let _ = Command::new("tmux")
                .args(["switch-client", "-t", &session_id.0])
                .status();
            // Exit immediately; popup will close automatically
            std::process::exit(0);
        }
        TmuxEnvironment::InsideTmux { .. } => {
            let _ = Command::new("tmux")
                .args(["select-window", "-t", &window_id.0])
                .status();
            let _ = Command::new("tmux")
                .args(["select-pane", "-t", &pane_id.0])
                .status();
            let _ = Command::new("tmux")
                .args(["switch-client", "-t", &session_id.0])
                .status();
            // Exit so the user returns to tmux
            std::process::exit(0);
        }
        TmuxEnvironment::OutsideTmux => {
            // Restore terminal before exec
            crossterm::terminal::disable_raw_mode()?;
            crossterm::execute!(
                std::io::stdout(),
                crossterm::terminal::LeaveAlternateScreen,
                crossterm::cursor::Show
            )?;

            // On Unix platforms, exec directly into tmux attach
            #[cfg(unix)]
            {
                use std::os::unix::process::CommandExt;
                let _ = Command::new("tmux")
                    .args([
                        "attach-session",
                        "-t",
                        session_name,
                        ";",
                        "select-window",
                        "-t",
                        &window_id.0,
                        ";",
                        "select-pane",
                        "-t",
                        &pane_id.0,
                    ])
                    .exec();
            }

            #[cfg(not(unix))]
            {
                let _ = Command::new("tmux")
                    .args(["attach-session", "-t", session_name])
                    .status();
            }

            std::process::exit(0);
        }
    }
}
