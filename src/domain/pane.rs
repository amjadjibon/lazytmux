use super::id::{PaneId, SessionId, WindowId};
use ansi_to_tui::IntoText;
use ratatui::text::Text;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

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
    /// tmux `pane_synchronized`: input sent to any pane in this window is
    /// broadcast to all of them.
    pub synchronized: bool,
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
            synchronized: false,
        }
    }

    pub fn set_preview(&mut self, raw: Vec<u8>) {
        // A pane that produced no new output returns byte-identical bytes on
        // every capture, which is the common case at the refresh interval.
        // Rebuilding the line vector for that costs far more than the compare.
        if raw == self.preview_raw && !raw.is_empty() {
            return;
        }

        let text_lossy = String::from_utf8_lossy(&raw);
        let mut lines: Vec<String> = text_lossy.lines().map(|s| s.to_string()).collect();
        while lines.last().is_some_and(|l| line_is_blank(l.as_bytes())) {
            lines.pop();
        }
        self.preview_lines = lines;
        self.preview_raw = raw;
    }

    pub fn preview_text(&self) -> Text<'static> {
        self.preview_text_tail(usize::MAX)
    }

    /// The preview as plain text, with terminal escape sequences removed.
    /// Suitable for the clipboard, where SGR codes would be pasted verbatim.
    pub fn plain_preview(&self) -> String {
        self.preview_lines
            .iter()
            .map(|l| String::from_utf8_lossy(&visible_bytes(l.as_bytes())).into_owned())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The parsed preview, limited to the last `max_lines` non-blank lines.
    ///
    /// Callers render the tail of the buffer and scroll everything above it out
    /// of view, so parsing the whole thing is wasted work — and Inspect mode
    /// keeps 2000 lines, which would be re-parsed on every frame. tmux
    /// `capture-pane -e` re-emits SGR state at the start of every line, so
    /// cutting on a line boundary keeps colours intact.
    pub fn preview_text_tail(&self, max_lines: usize) -> Text<'static> {
        if self.preview_raw.is_empty() {
            if self.preview_lines.is_empty() {
                return Text::raw("No output captured");
            }
            return Text::from(self.preview_lines.join("\n"));
        }

        let tail = tail_lines(&self.preview_raw, max_lines);
        let mut text = tail
            .into_text()
            .unwrap_or_else(|_| Text::from(String::from_utf8_lossy(tail).to_string()));

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

/// True when a line renders as blank: nothing but whitespace once terminal
/// escape sequences are removed.
fn line_is_blank(line: &[u8]) -> bool {
    !visible_bytes(line).iter().any(|b| !b.is_ascii_whitespace())
}

/// `line` with ANSI escape sequences removed.
fn visible_bytes(line: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(line.len());
    let mut i = 0;
    while i < line.len() {
        if line[i] == 0x1b {
            i += 1;
            match line.get(i) {
                // CSI: parameters and intermediates, then a final byte 0x40..=0x7e.
                Some(b'[') => {
                    i += 1;
                    while i < line.len() && !(0x40..=0x7e).contains(&line[i]) {
                        i += 1;
                    }
                    i += 1;
                }
                // OSC: runs to BEL or ST (ESC \).
                Some(b']') => {
                    i += 1;
                    while i < line.len() && line[i] != 0x07 {
                        if line[i] == 0x1b && line.get(i + 1) == Some(&b'\\') {
                            i += 1;
                            break;
                        }
                        i += 1;
                    }
                    i += 1;
                }
                // Any other two-byte escape.
                Some(_) => i += 1,
                None => {}
            }
        } else {
            out.push(line[i]);
            i += 1;
        }
    }
    out
}

/// `raw` with trailing blank lines dropped, then limited to its last
/// `max_lines` lines. Returns a slice of `raw`, so no copying happens.
fn tail_lines(raw: &[u8], max_lines: usize) -> &[u8] {
    let mut starts = vec![0usize];
    for (i, b) in raw.iter().enumerate() {
        if *b == b'\n' {
            starts.push(i + 1);
        }
    }

    // Line `i` spans starts[i]..(starts[i + 1] - 1), the last one runs to the end.
    let line = |i: usize| -> &[u8] {
        let start = starts[i];
        let end = starts.get(i + 1).map_or(raw.len(), |n| n.saturating_sub(1));
        &raw[start..end.max(start)]
    };

    let mut end = starts.len();
    while end > 0 && line_is_blank(line(end - 1)) {
        end -= 1;
    }
    if end == 0 {
        return &[];
    }

    let start = end.saturating_sub(max_lines);
    let from = starts[start];
    let to = starts
        .get(end)
        .map_or(raw.len(), |next| next.saturating_sub(1));
    &raw[from..to.max(from)]
}

/// How long a resolved branch is reused before the working tree is walked again.
const GIT_BRANCH_TTL: Duration = Duration::from_secs(5);

type GitBranchCache = Mutex<HashMap<PathBuf, (Option<String>, Instant)>>;

fn git_branch_cache() -> &'static GitBranchCache {
    static CACHE: OnceLock<GitBranchCache> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Resolve the git branch for a pane's working directory.
///
/// `Pane::new` runs for every pane on every refresh, so the uncached walk (up to
/// 12 `stat`+`open` pairs per pane) would run several times a second on the UI
/// thread. Results are memoised per path for `GIT_BRANCH_TTL`.
pub fn detect_git_branch(path: &std::path::Path) -> Option<String> {
    let mut cache = match git_branch_cache().lock() {
        Ok(cache) => cache,
        // A poisoned lock only means some other thread panicked mid-lookup;
        // fall back to walking rather than propagating the panic.
        Err(_) => return detect_git_branch_uncached(path),
    };

    if let Some((branch, fetched_at)) = cache.get(path)
        && fetched_at.elapsed() < GIT_BRANCH_TTL
    {
        return branch.clone();
    }

    let branch = detect_git_branch_uncached(path);
    if cache.len() > 512 {
        cache.retain(|_, (_, fetched_at)| fetched_at.elapsed() < GIT_BRANCH_TTL);
    }
    cache.insert(path.to_path_buf(), (branch.clone(), Instant::now()));
    branch
}

fn detect_git_branch_uncached(path: &std::path::Path) -> Option<String> {
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
                    } else if trimmed.chars().count() >= 7 {
                        // Detached HEAD: show the short SHA. Cap by characters,
                        // never by byte index, so a corrupt HEAD cannot panic.
                        return Some(trimmed.chars().take(7).collect());
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

    fn test_pane() -> Pane {
        Pane::new(
            PaneId::from("%1"),
            WindowId::from("@1"),
            SessionId::from("$1"),
            1,
            true,
            "zsh".to_string(),
            PathBuf::from("/tmp"),
            80,
            24,
        )
    }

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
        let output_with_blanks =
            b"cargo build\nFinished dev target(s)\nprompt$ \n\n\n\n\n".to_vec();
        pane.set_preview(output_with_blanks);
        assert_eq!(pane.preview_lines.len(), 3);
        assert_eq!(pane.preview_lines.last().unwrap(), "prompt$ ");

        let text = pane.preview_text();
        assert_eq!(text.lines.len(), 3);
    }

    #[test]
    fn test_preview_text_tail_windows_to_the_end() {
        let mut pane = test_pane();
        let body: String = (0..500).map(|i| format!("line {i}\n")).collect();
        pane.set_preview(body.into_bytes());

        let tail = pane.preview_text_tail(10);
        assert_eq!(tail.lines.len(), 10);
        let last: String = tail
            .lines
            .last()
            .unwrap()
            .spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect();
        assert_eq!(last, "line 499", "windowing must keep the newest lines");

        // The window is applied after trailing blanks are dropped, so a mostly
        // empty buffer still shows its real content.
        let mut sparse = test_pane();
        sparse.set_preview(b"hello\nworld\n\n\n\n\n\n\n\n\n\n\n".to_vec());
        let tail = sparse.preview_text_tail(3);
        assert_eq!(tail.lines.len(), 2);

        // An unbounded window matches the old whole-buffer behaviour.
        assert_eq!(pane.preview_text().lines.len(), 500);
    }

    #[test]
    fn test_preview_text_tail_preserves_colour() {
        let mut pane = test_pane();
        // tmux capture-pane -e re-emits SGR per line, so a cut line keeps colour.
        let raw = (0..50)
            .map(|i| format!("\x1b[31mred {i}\x1b[39m\n"))
            .collect::<String>();
        pane.set_preview(raw.into_bytes());

        let tail = pane.preview_text_tail(5);
        assert_eq!(tail.lines.len(), 5);
        let styled = tail.lines[0].spans.iter().any(|s| s.style.fg.is_some());
        assert!(styled, "colour was lost when slicing the buffer");
    }

    #[test]
    fn test_set_preview_skips_identical_capture() {
        let mut pane = test_pane();
        pane.set_preview(b"one\ntwo\n".to_vec());
        let before = pane.preview_lines.clone();

        pane.set_preview(b"one\ntwo\n".to_vec());
        assert_eq!(pane.preview_lines, before);

        pane.set_preview(b"one\ntwo\nthree\n".to_vec());
        assert_eq!(pane.preview_lines.len(), 3, "a changed capture must apply");
    }

    #[test]
    fn test_plain_preview_strips_escape_sequences() {
        let mut pane = test_pane();
        pane.set_preview(b"\x1b[32mok\x1b[0m done\n\x1b]0;title\x07plain\n".to_vec());
        let copied = pane.plain_preview();
        assert_eq!(copied, "ok done\nplain");
        assert!(
            !copied.contains('\x1b'),
            "escape codes reached the clipboard"
        );
    }

    #[test]
    fn test_blank_line_detection_ignores_escapes() {
        assert!(line_is_blank(b""));
        assert!(line_is_blank(b"   "));
        assert!(line_is_blank(b"\x1b[31m\x1b[39m"));
        assert!(line_is_blank(b"\x1b[m   \x1b[m"));
        assert!(!line_is_blank(b"\x1b[31mx\x1b[39m"));
        assert!(!line_is_blank(b"x"));
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
