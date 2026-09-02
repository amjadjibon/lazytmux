use crate::app::{App, FocusColumn};
use crate::domain::{LayoutNode, LayoutSplit, Pane, Window};
use crate::ui::theme::Theme;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::widgets::{Block, Borders, Paragraph};

pub fn render(app: &App, frame: &mut Frame, area: Rect, theme: &Theme) {
    let focused = app.focus == FocusColumn::Panes;
    let main_block = theme.block("PANES", focused);

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
    if let Some(root_node) = LayoutNode::parse(&window.layout_str) {
        render_layout_node(&root_node, window, app, frame, inner_area, focused);
    } else {
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

    if is_selected && area.height >= 4 {
        if area.width >= 35 {
            pane_block = pane_block.title_bottom(" [◀] [▼] [▲] [▶] [↕ swap] ");
        } else if area.width >= 20 {
            pane_block = pane_block.title_bottom(" [◀][▼][▲][▶] ");
        }
    }

    let text = pane.preview_text();
    let preview_widget = Paragraph::new(text).block(pane_block);
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
