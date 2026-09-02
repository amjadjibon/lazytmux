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
    pub git_branch: Option<String>,
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
        let git_branch = detect_git_branch(&current_path);
        Self {
            id,
            window_id,
            session_id,
            index,
            active,
            current_command,
            current_path,
            git_branch,
            width,
            height,
            preview_lines: Vec::new(),
            preview_raw: Vec::new(),
        }
    }

    pub fn set_preview(&mut self, raw: Vec<u8>) {
        let text_lossy = String::from_utf8_lossy(&raw);
        let mut lines: Vec<String> = text_lossy.lines().map(|s| s.to_string()).collect();
        while let Some(last) = lines.last() {
            if last.trim().is_empty() {
                lines.pop();
            } else {
                break;
            }
        }
        self.preview_lines = lines;
        self.preview_raw = raw;
    }

    pub fn preview_text(&self) -> Text<'static> {
        if self.preview_raw.is_empty() {
            if self.preview_lines.is_empty() {
                return Text::raw("No output captured");
            }
            return Text::from(self.preview_lines.join("\n"));
        }
        let mut text =
            self.preview_raw.as_slice().into_text().unwrap_or_else(|_| {
                Text::from(String::from_utf8_lossy(&self.preview_raw).to_string())
            });

        // Trim trailing empty lines so the widget displays the most recent output at the bottom
        while let Some(last) = text.lines.last() {
            if last.spans.iter().all(|s| s.content.trim().is_empty()) {
                text.lines.pop();
            } else {
                break;
            }
        }

        if text.lines.is_empty() {
            Text::raw("No output captured")
        } else {
            text
        }
    }
}

pub fn detect_git_branch(path: &std::path::Path) -> Option<String> {
    use std::io::Read;
    let mut current = path;
    let mut depth = 0;

    while depth < 12 {
        depth += 1;
        let git_dir = current.join(".git");
        if git_dir.is_dir() {
            let head_file = git_dir.join("HEAD");
            if let Ok(mut file) = std::fs::File::open(head_file) {
                let mut buffer = [0u8; 1024];
                if let Ok(n) = file.read(&mut buffer) {
                    let content = String::from_utf8_lossy(&buffer[..n]);
                    let trimmed = content.trim();
                    if let Some(branch) = trimmed.strip_prefix("ref: refs/heads/") {
                        return Some(branch.to_string());
                    } else if !trimmed.is_empty() && trimmed.len() >= 7 {
                        return Some(trimmed[..7].to_string());
                    }
                }
            }
            return None;
        } else if git_dir.is_file() {
            if let Ok(mut file) = std::fs::File::open(git_dir) {
                let mut buffer = [0u8; 1024];
                if let Ok(n) = file.read(&mut buffer) {
                    let content = String::from_utf8_lossy(&buffer[..n]);
                    if let Some(gitdir_path) = content.trim().strip_prefix("gitdir:") {
                        let head_file = std::path::PathBuf::from(gitdir_path.trim()).join("HEAD");
                        if let Ok(mut hfile) = std::fs::File::open(head_file) {
                            let mut hbuf = [0u8; 1024];
                            if let Ok(hn) = hfile.read(&mut hbuf) {
                                let head_content = String::from_utf8_lossy(&hbuf[..hn]);
                                let trimmed = head_content.trim();
                                if let Some(branch) = trimmed.strip_prefix("ref: refs/heads/") {
                                    return Some(branch.to_string());
                                }
                            }
                        }
                    }
                }
            }
            return None;
        }

        match current.parent() {
            Some(parent) if parent != current => current = parent,
            _ => break,
        }
    }
    None
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

    #[test]
    fn test_pane_preview_trims_trailing_blank_lines() {
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
        let output_with_blanks = b"cargo build\nFinished dev target(s)\nprompt$ \n\n\n\n\n".to_vec();
        pane.set_preview(output_with_blanks);
        assert_eq!(pane.preview_lines.len(), 3);
        assert_eq!(pane.preview_lines.last().unwrap(), "prompt$ ");

        let text = pane.preview_text();
        assert_eq!(text.lines.len(), 3);
    }

    #[test]
    fn test_detect_git_branch() {
        let repo_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let branch = detect_git_branch(&repo_path);
        assert!(
            branch.is_some(),
            "Current workspace should be in a git repo"
        );
        assert_eq!(branch.unwrap(), "main");
    }
}
