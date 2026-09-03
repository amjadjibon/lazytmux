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
    /// Captured output as plain text, one entry per line, with terminal escape
    /// sequences removed and trailing blank lines dropped. This is what search,
    /// clipboard copy, and line counts work against; rendering goes through
    /// [`Pane::preview_window`], which colours from `preview_raw`.
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

        let line_count = content_lines(&raw).len();
        self.preview_lines = content_lines(&raw)
            .iter()
            .map(|l| String::from_utf8_lossy(&visible_bytes(l)).into_owned())
            .collect();
        debug_assert_eq!(self.preview_lines.len(), line_count);
        self.preview_raw = raw;
    }

    /// The whole preview, parsed. Prefer the windowed accessors on render paths.
    pub fn preview_text(&self) -> Text<'static> {
        self.preview_window(0, usize::MAX)
    }

    /// The last `max_lines` lines, parsed. For preview cards, which show the
    /// bottom of the buffer.
    pub fn preview_text_tail(&self, max_lines: usize) -> Text<'static> {
        let skip = self.preview_lines.len().saturating_sub(max_lines);
        self.preview_window(skip, max_lines)
    }

    /// Lines `skip..skip + take` of the preview, parsed with their colours.
    ///
    /// Only the visible viewport is ever on screen, so parsing the whole buffer
    /// is wasted work — Inspect mode keeps 2000 lines, which would otherwise be
    /// re-parsed on every frame. tmux `capture-pane -e` re-emits SGR state at
    /// the start of every line, so cutting on a line boundary keeps colours
    /// intact. Line indices match [`Pane::preview_lines`].
    pub fn preview_window(&self, skip: usize, take: usize) -> Text<'static> {
        let lines = content_lines(&self.preview_raw);
        let window: Vec<&[u8]> = lines.into_iter().skip(skip).take(take).collect();
        if window.is_empty() {
            return Text::raw("No output captured");
        }

        let joined = window.join(&b'\n');
        let text = joined
            .as_slice()
            .into_text()
            .unwrap_or_else(|_| Text::from(String::from_utf8_lossy(&joined).to_string()));

        if text.lines.is_empty() {
            Text::raw("No output captured")
        } else {
            text
        }
    }
}

/// The lines of `raw` that carry content: split on newlines, with trailing
/// blank lines dropped. Blank means "nothing but whitespace once escape
/// sequences are removed", so a line of bare SGR codes does not count.
///
/// This is the single definition of "a preview line": `preview_lines` and every
/// rendered window are built from it, so indices agree across them.
fn content_lines(raw: &[u8]) -> Vec<&[u8]> {
    let mut lines: Vec<&[u8]> = raw.split(|b| *b == b'\n').collect();
    while lines.last().is_some_and(|l| line_is_blank(l)) {
        lines.pop();
    }
    lines
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

        // Mid-buffer windows keep colour too (Inspect mode scrolls into them).
        let mid = pane.preview_window(20, 4);
        assert_eq!(mid.lines.len(), 4);
        assert!(mid.lines[0].spans.iter().any(|s| s.style.fg.is_some()));
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
    fn test_preview_lines_are_plain_text() {
        let mut pane = test_pane();
        pane.set_preview(b"\x1b[32mok\x1b[0m done\n\x1b]0;title\x07plain\n".to_vec());
        assert_eq!(pane.preview_lines, vec!["ok done", "plain"]);
        // Search and clipboard both read preview_lines, so neither can see SGR.
        assert!(!pane.preview_lines.iter().any(|l| l.contains('\x1b')));
    }

    #[test]
    fn test_search_matches_text_split_by_escape_sequences() {
        let mut pane = test_pane();
        // tmux colours a substring, splitting the word with SGR codes.
        pane.set_preview(b"error: \x1b[1;31mconnection\x1b[0m refused\n".to_vec());
        assert!(
            pane.preview_lines[0].contains("connection refused"),
            "escape sequences broke the match: {:?}",
            pane.preview_lines[0]
        );
    }

    #[test]
    fn test_line_indices_agree_between_plain_and_rendered() {
        let mut pane = test_pane();
        let raw: String = (0..40)
            .map(|i| format!("\x1b[3{}mline {i}\x1b[0m\n", i % 8))
            .collect();
        pane.set_preview(raw.into_bytes());
        assert_eq!(pane.preview_lines.len(), 40);

        // A window starting at index 7 must render the line search reports at 7.
        let win = pane.preview_window(7, 3);
        assert_eq!(win.lines.len(), 3);
        let first: String = win.lines[0]
            .spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect();
        assert_eq!(first, pane.preview_lines[7]);
        assert_eq!(first, "line 7");
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
