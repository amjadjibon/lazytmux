use crate::action::Action;
use crate::app::{App, FocusColumn};
use crate::domain::{LayoutNode, LayoutSplit, Pane, Window};
use crate::ui::theme::Theme;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::widgets::{Block, Borders, Paragraph};

pub fn render(app: &App, frame: &mut Frame, area: Rect, theme: &Theme) {
    let focused = app.focus == FocusColumn::Panes;
    let title = match app.sidebar_mode {
        crate::ui::SidebarMode::PanesOnly => "[▶ EXPAND SIDEBARS] PANES",
        crate::ui::SidebarMode::SessionsHidden => "[▶ SESSIONS] PANES",
        crate::ui::SidebarMode::WindowsHidden => "[▶ WINDOWS] PANES",
        crate::ui::SidebarMode::Full => "PANES",
    };
    let main_block = theme.block(title, focused);

    let window = match app.selected_window() {
        Some(w) => w,
        None => {
            let empty_msg = Paragraph::new(" No window selected")
                .style(theme.dim)
                .block(main_block);
            frame.render_widget(empty_msg, area);
            return;
        }
    };

    if window.panes.is_empty() {
        let empty_msg = Paragraph::new(" No panes in selected window")
            .style(theme.dim)
            .block(main_block);
        frame.render_widget(empty_msg, area);
        return;
    }

    let inner_area = main_block.inner(area);
    frame.render_widget(main_block, area);

    // Try parsing the tmux window_layout AST to render true 2D geometry
    let rendered_with_layout = if let Some(root_node) = LayoutNode::parse(&window.layout_str) {
        if root_node.leaf_count() == window.panes.len() && root_node.all_panes_found(window) {
            render_layout_node(&root_node, window, app, frame, inner_area, focused);
            true
        } else {
            false
        }
    } else {
        false
    };

    if !rendered_with_layout {
        render_fallback(window, app, frame, inner_area, focused);
    }
}

fn render_layout_node(
    node: &LayoutNode,
    window: &Window,
    app: &App,
    frame: &mut Frame,
    area: Rect,
    focused: bool,
) {
    match node {
        LayoutNode::Leaf { pane_id, .. } => {
            if let Some(id) = pane_id {
                if let Some(pane) = window.get_pane(id) {
                    render_pane_card(pane, window, app, frame, area, focused);
                }
            } else if let Some(pane) = window.panes.first() {
                render_pane_card(pane, window, app, frame, area, focused);
            }
        }
        LayoutNode::Container {
            split, children, ..
        } => {
            if children.is_empty() {
                return;
            }

            let dir = match split {
                LayoutSplit::Horizontal => Direction::Horizontal,
                LayoutSplit::Vertical => Direction::Vertical,
            };

            let total_dim: u32 = children.iter().map(|c| c.dimension(split) as u32).sum();
            let constraints: Vec<Constraint> = children
                .iter()
                .map(|c| {
                    let dim = c.dimension(split) as u32;
                    if total_dim > 0 {
                        Constraint::Ratio(dim.max(1), total_dim.max(1))
                    } else {
                        Constraint::Ratio(1, children.len() as u32)
                    }
                })
                .collect();

            let chunks = Layout::default()
                .direction(dir)
                .constraints(constraints)
                .split(area);

            for (idx, child) in children.iter().enumerate() {
                if idx < chunks.len() {
                    render_layout_node(child, window, app, frame, chunks[idx], focused);
                }
            }
        }
    }
}

/// A clickable control drawn on the bottom border of the selected pane card.
///
/// The strip is built once here and used for both drawing and hit-testing, so
/// the two can never drift apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneControl {
    ResizeLeft,
    ResizeDown,
    ResizeUp,
    ResizeRight,
    Swap,
    /// Split into stacked panes, one above the other.
    SplitStacked,
    /// Split into side-by-side panes.
    SplitSideBySide,
    /// Wipe the pane's screen and scrollback, leaving the process running.
    Clear,
    Kill,
}

impl PaneControl {
    pub fn label(self) -> &'static str {
        match self {
            PaneControl::ResizeLeft => "[◀]",
            PaneControl::ResizeDown => "[▼]",
            PaneControl::ResizeUp => "[▲]",
            PaneControl::ResizeRight => "[▶]",
            PaneControl::Swap => "[↕]",
            // Each glyph is the resulting layout, shaded: bottom half for a
            // stacked split, left half for a side-by-side one.
            PaneControl::SplitStacked => "[⬓]",
            PaneControl::SplitSideBySide => "[◧]",
            PaneControl::Clear => "[c]",
            PaneControl::Kill => "[x]",
        }
    }

    pub fn action(self) -> Action {
        use crate::tmux::client::ResizeDirection;
        match self {
            PaneControl::ResizeLeft => Action::ResizePane(ResizeDirection::Left, 4),
            PaneControl::ResizeDown => Action::ResizePane(ResizeDirection::Down, 2),
            PaneControl::ResizeUp => Action::ResizePane(ResizeDirection::Up, 2),
            PaneControl::ResizeRight => Action::ResizePane(ResizeDirection::Right, 4),
            PaneControl::Swap => Action::SwapPaneDown,
            PaneControl::SplitStacked => Action::SplitPane { vertical: false },
            PaneControl::SplitSideBySide => Action::SplitPane { vertical: true },
            // Unlike the `c` key, the button asks first: a misclick must not
            // silently discard scrollback.
            PaneControl::Clear => Action::PromptClearPane,
            // Goes through the normal kill path, so it obeys `confirm_on_kill`
            // and asks before destroying anything.
            PaneControl::Kill => Action::PromptKill,
        }
    }
}

const ALL_CONTROLS: &[PaneControl] = &[
    PaneControl::ResizeLeft,
    PaneControl::ResizeDown,
    PaneControl::ResizeUp,
    PaneControl::ResizeRight,
    PaneControl::Swap,
    PaneControl::SplitStacked,
    PaneControl::SplitSideBySide,
    PaneControl::Clear,
    PaneControl::Kill,
];

/// Splitting and closing stay available on the narrowest cards that show
/// anything at all; resizing has keyboard and drag equivalents.
const ESSENTIAL_CONTROLS: &[PaneControl] = &[
    PaneControl::SplitStacked,
    PaneControl::SplitSideBySide,
    PaneControl::Kill,
];

/// The control strip for a card of a given width: what to draw, and which
/// column each control occupies.
#[derive(Debug, Clone)]
pub struct ControlStrip {
    label: String,
    hits: Vec<(u16, u16, PaneControl)>,
    /// The column just past the last label. The card needs one more column
    /// after it for the right border corner.
    content_end: u16,
}

impl ControlStrip {
    pub fn label(&self) -> &str {
        &self.label
    }

    /// The narrowest card this strip fits on, corners included.
    pub fn min_width(&self) -> u16 {
        self.content_end + 1
    }

    /// The control at `col_offset` columns from the card's left edge, if any.
    pub fn control_at(&self, col_offset: u16) -> Option<PaneControl> {
        self.hits
            .iter()
            .find(|(start, end, _)| col_offset >= *start && col_offset < *end)
            .map(|(_, _, control)| *control)
    }
}

/// Build the widest control strip that fits a card `width` columns across, or
/// `None` when even the smallest set would not fit.
///
/// Each candidate is measured rather than compared against a hardcoded
/// threshold, so adding or renaming a control cannot silently push the strip
/// past a card's right border.
pub fn control_strip(width: u16) -> Option<ControlStrip> {
    [
        (ALL_CONTROLS, " "),
        (ALL_CONTROLS, ""),
        (ESSENTIAL_CONTROLS, ""),
    ]
    .into_iter()
    .map(|(controls, separator)| build_strip(controls, separator))
    .find(|strip| strip.min_width() <= width)
}

fn build_strip(controls: &[PaneControl], separator: &str) -> ControlStrip {
    use unicode_width::UnicodeWidthStr;

    // A left-aligned bottom title starts one column in, past the corner.
    let mut column: u16 = 1;
    let mut label = String::from(" ");
    column += 1;

    let mut hits = Vec::with_capacity(controls.len());
    for (idx, control) in controls.iter().enumerate() {
        if idx > 0 {
            label.push_str(separator);
            column += separator.width() as u16;
        }
        // Always keep the destructive control clear of its neighbour.
        if *control == PaneControl::Kill && separator.is_empty() {
            label.push(' ');
            column += 1;
        }

        let text = control.label();
        let text_width = text.width() as u16;
        label.push_str(text);
        hits.push((column, column + text_width, *control));
        column += text_width;
    }
    let content_end = column;
    // Decorative only: it is the first thing clipped on a card that is exactly
    // wide enough, which is why it is not part of `content_end`.
    label.push(' ');

    ControlStrip {
        label,
        hits,
        content_end,
    }
}

fn render_pane_card(
    pane: &Pane,
    window: &Window,
    app: &App,
    frame: &mut Frame,
    area: Rect,
    focused: bool,
) {
    let is_selected = window
        .panes
        .get(app.selection.pane_idx)
        .map(|p| p.id == pane.id)
        .unwrap_or(false);

    let border_style = if is_selected && focused {
        app.theme.border_focused
    } else if is_selected {
        app.theme.info
    } else {
        app.theme.border_style
    };

    let active_tag = if pane.active { " (active)" } else { "" };
    let branch_str = pane
        .git_branch
        .as_deref()
        .map(|b| format!(" [{b}]"))
        .unwrap_or_default();
    let title = format!(
        " {} {}{}{} ",
        pane.id.0, pane.current_command, branch_str, active_tag
    );

    let mut pane_block = Block::default()
        .borders(Borders::ALL)
        .border_type(app.theme.border_type)
        .border_style(border_style)
        .title(title)
        .title_style(if is_selected {
            app.theme.title_focused
        } else {
            app.theme.title
        });

    // Controls are drawn only on the selected card, and `App` only hit-tests a
    // card that was already selected, so a click can never hit a button that
    // was not on screen.
    if is_selected
        && area.height >= 4
        && let Some(strip) = control_strip(area.width)
    {
        pane_block = pane_block.title_bottom(strip.label().to_string());
    }

    // Only the bottom `inner_height` lines are ever on screen; the rest were
    // parsed and then scrolled out of view every frame.
    let inner_height = area.height.saturating_sub(2);
    let text = pane.preview_text_tail(inner_height as usize);
    let total_lines = text.lines.len() as u16;
    let scroll_y = total_lines.saturating_sub(inner_height);

    let preview_widget = Paragraph::new(text).block(pane_block).scroll((scroll_y, 0));
    frame.render_widget(preview_widget, area);
}

fn render_fallback(window: &Window, app: &App, frame: &mut Frame, area: Rect, focused: bool) {
    let pane_count = window.panes.len();
    let constraints: Vec<Constraint> = (0..pane_count)
        .map(|_| Constraint::Ratio(1, pane_count as u32))
        .collect();

    let pane_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    for (idx, pane) in window.panes.iter().enumerate() {
        if idx >= pane_chunks.len() {
            break;
        }
        render_pane_card(pane, window, app, frame, pane_chunks[idx], focused);
    }
}
