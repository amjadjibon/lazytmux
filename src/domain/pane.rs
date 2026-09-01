use super::id::{PaneId, SessionId, WindowId};
use ansi_to_tui::IntoText;
use ratatui::text::Text;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Pane {
    pub id: PaneId,
    pub window_id: WindowId,
    pub session_id: SessionId,
    pub index: u32,
    pub active: bool,
    pub current_command: String,
    pub current_path: PathBuf,
    pub width: u16,
    pub height: u16,
    pub preview_lines: Vec<String>,
    pub preview_raw: Vec<u8>,
}

impl Pane {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: PaneId,
        window_id: WindowId,
        session_id: SessionId,
        index: u32,
        active: bool,
        current_command: String,
        current_path: PathBuf,
        width: u16,
        height: u16,
    ) -> Self {
        Self {
            id,
            window_id,
            session_id,
            index,
            active,
            current_command,
            current_path,
            width,
            height,
            preview_lines: Vec::new(),
            preview_raw: Vec::new(),
        }
    }

    pub fn set_preview(&mut self, raw: Vec<u8>) {
        let text_lossy = String::from_utf8_lossy(&raw);
        self.preview_lines = text_lossy.lines().map(|s| s.to_string()).collect();
        self.preview_raw = raw;
    }

    pub fn preview_text(&self) -> Text<'static> {
        if self.preview_raw.is_empty() {
            if self.preview_lines.is_empty() {
                return Text::raw("No output captured");
            }
            return Text::from(self.preview_lines.join("\n"));
        }
        self.preview_raw
            .as_slice()
            .into_text()
            .unwrap_or_else(|_| Text::from(String::from_utf8_lossy(&self.preview_raw).to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pane_preview_empty() {
        let pane = Pane::new(
            PaneId::from("%1"),
            WindowId::from("@1"),
            SessionId::from("$1"),
            1,
            true,
            "zsh".to_string(),
            PathBuf::from("/tmp"),
            80,
            24,
        );
        assert_eq!(pane.preview_text(), Text::raw("No output captured"));
    }

    #[test]
    fn test_pane_preview_with_ansi() {
        let mut pane = Pane::new(
            PaneId::from("%1"),
            WindowId::from("@1"),
            SessionId::from("$1"),
            1,
            true,
            "zsh".to_string(),
            PathBuf::from("/tmp"),
            80,
            24,
        );
        let ansi_bytes = b"\x1b[32mHello\x1b[0m \x1b[1;34mWorld\x1b[0m\n".to_vec();
        pane.set_preview(ansi_bytes);
        assert!(!pane.preview_lines.is_empty());
        let text = pane.preview_text();
        assert_eq!(text.lines.len(), 1);
    }
}
