use crate::ui::SidebarMode;
use unicode_width::UnicodeWidthStr;

/// A clickable control in a column header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderControl {
    /// Restore the full three-column view.
    Expand,
    /// Collapse this column.
    Collapse,
    /// Create a session (Sessions column) or a window (Windows column).
    New,
    /// Kill the selected session or window, with the usual confirmation.
    Kill,
}

impl HeaderControl {
    fn label(self, expand_text: &str) -> String {
        match self {
            HeaderControl::Expand => expand_text.to_string(),
            HeaderControl::Collapse => "[◀]".to_string(),
            HeaderControl::New => "[+]".to_string(),
            HeaderControl::Kill => "[x]".to_string(),
        }
    }
}

/// A column header: the title to draw, and where each control sits in it.
///
/// Built once and used for both drawing and hit-testing, so a control is always
/// clickable exactly where it is painted.
#[derive(Debug, Clone)]
pub struct HeaderStrip {
    title: String,
    hits: Vec<(u16, u16, HeaderControl)>,
}

impl HeaderStrip {
    pub fn title(&self) -> &str {
        &self.title
    }

    /// The control at `col_offset` columns from the column's left edge.
    pub fn control_at(&self, col_offset: u16) -> Option<HeaderControl> {
        self.hits
            .iter()
            .find(|(start, end, _)| col_offset >= *start && col_offset < *end)
            .map(|(_, _, control)| *control)
    }
}

/// One piece of a header: either plain text or a clickable control.
struct Piece {
    text: String,
    control: Option<HeaderControl>,
}

/// Lay pieces out from the column's left edge, or `None` if they do not fit.
///
/// Column offsets account for two things ahead of the first piece: the border
/// corner at offset 0, and the space `Theme::block` pads every title with. The
/// strip must not add padding of its own or everything shifts.
const TITLE_START: u16 = 2;

fn lay_out(pieces: &[Piece], separator: &str, width: u16) -> Option<HeaderStrip> {
    let mut title = String::new();
    let mut column = TITLE_START;
    let mut hits = Vec::new();

    for (idx, piece) in pieces.iter().enumerate() {
        if idx > 0 {
            title.push_str(separator);
            column += separator.width() as u16;
        }
        let piece_width = piece.text.width() as u16;
        title.push_str(&piece.text);
        if let Some(control) = piece.control {
            hits.push((column, column + piece_width, control));
        }
        column += piece_width;
    }
    // Leave room for the title's trailing pad and the closing corner.
    (column + 2 <= width).then_some(HeaderStrip { title, hits })
}

fn strip(name: &str, expand: Option<&str>, width: u16) -> HeaderStrip {
    let controls = |with_name: bool| -> Vec<Piece> {
        let mut pieces = Vec::new();
        if let Some(expand) = expand {
            pieces.push(Piece {
                text: expand.to_string(),
                control: Some(HeaderControl::Expand),
            });
        }
        if with_name {
            pieces.push(Piece {
                text: name.to_string(),
                control: None,
            });
        }
        for control in [
            HeaderControl::New,
            HeaderControl::Kill,
            HeaderControl::Collapse,
        ] {
            pieces.push(Piece {
                text: control.label(""),
                control: Some(control),
            });
        }
        pieces
    };

    // Widest form that fits: spaced, then tight, then controls only. The title
    // text is what goes first — the buttons are the part you cannot replace
    // with a keystroke you can see.
    lay_out(&controls(true), " ", width)
        .or_else(|| lay_out(&controls(true), "", width))
        .or_else(|| lay_out(&controls(false), "", width))
        .unwrap_or_else(|| HeaderStrip {
            title: String::new(),
            hits: Vec::new(),
        })
}

pub fn sessions_header(width: u16, mode: SidebarMode) -> HeaderStrip {
    let expand = (mode == SidebarMode::WindowsHidden).then_some("[▶ Windows]");
    strip("SESSIONS", expand, width)
}

pub fn windows_header(width: u16, mode: SidebarMode) -> HeaderStrip {
    let expand = (mode == SidebarMode::SessionsHidden).then_some("[▶ Sessions]");
    strip("WINDOWS", expand, width)
}
