use crate::app::{App, FocusColumn};
use crate::ui::theme::Theme;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::Frame;

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

    let pane_count = window.panes.len();
    let constraints: Vec<Constraint> = (0..pane_count)
        .map(|_| Constraint::Ratio(1, pane_count as u32))
        .collect();

    let pane_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner_area);

    for (idx, pane) in window.panes.iter().enumerate() {
        if idx >= pane_chunks.len() {
            break;
        }
        let pane_area = pane_chunks[idx];
        let is_selected = idx == app.selection.pane_idx;

        let border_style = if is_selected && focused {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else if is_selected {
            Style::default().fg(Color::LightCyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let active_tag = if pane.active { " (active)" } else { "" };
        let title = format!(
            " {} {}{} ",
            pane.id.0, pane.current_command, active_tag
        );

        let pane_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(border_style)
            .title(title)
            .title_style(if is_selected {
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            });

        let text = pane.preview_text();
        let preview_widget = Paragraph::new(text).block(pane_block);
        frame.render_widget(preview_widget, pane_area);
    }
}
