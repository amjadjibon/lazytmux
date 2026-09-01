use crate::app::{App, FocusColumn};
use crate::ui::theme::Theme;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, Paragraph};

pub fn render(app: &App, frame: &mut Frame, area: Rect, theme: &Theme) {
    let focused = app.focus == FocusColumn::Windows;
    let block = theme.block("WINDOWS", focused);

    let session = match app.selected_session() {
        Some(s) => s,
        None => {
            let empty_msg = Paragraph::new(" No session selected")
                .style(theme.dim)
                .block(block);
            frame.render_widget(empty_msg, area);
            return;
        }
    };

    if session.windows.is_empty() {
        let empty_msg = Paragraph::new(" No windows in session\n Press 'n' to create")
            .style(theme.dim)
            .block(block);
        frame.render_widget(empty_msg, area);
        return;
    }

    let items: Vec<ListItem> = session
        .windows
        .iter()
        .enumerate()
        .map(|(idx, window)| {
            let is_selected = idx == app.selection.window_idx;

            let cursor_span = if is_selected {
                Span::styled(
                    "▶ ",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::raw("  ")
            };

            let active_span = if window.active {
                Span::styled(
                    "* ",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::raw("  ")
            };

            let index_span = Span::styled(
                format!("{}: ", window.index),
                Style::default().fg(Color::DarkGray),
            );

            let name_style = if is_selected {
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else if window.active {
                Style::default().fg(Color::White)
            } else {
                Style::default().fg(Color::Gray)
            };

            let name_span = Span::styled(&window.name, name_style);

            let pane_count_str = format!(" ({})", window.panes.len());
            let count_span = Span::styled(pane_count_str, theme.dim);

            let line = Line::from(vec![
                cursor_span,
                active_span,
                index_span,
                name_span,
                count_span,
            ]);

            let item = ListItem::new(line);
            if is_selected && focused {
                item.style(theme.selection)
            } else if is_selected {
                item.style(Style::default().bg(Color::Rgb(20, 30, 45)))
            } else {
                item
            }
        })
        .collect();

    let list = List::new(items).block(block);
    frame.render_widget(list, area);
}
