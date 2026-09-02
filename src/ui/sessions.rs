use crate::app::{App, FocusColumn};
use crate::ui::theme::Theme;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, Paragraph};

pub fn render(app: &App, frame: &mut Frame, area: Rect, theme: &Theme) {
    let focused = app.focus == FocusColumn::Sessions;
    let block = theme.block("SESSIONS [◀]", focused);

    if app.sessions.is_empty() {
        let empty_msg = Paragraph::new(" No sessions\n Press 'n' to create")
            .style(theme.dim)
            .block(block);
        frame.render_widget(empty_msg, area);
        return;
    }

    let items: Vec<ListItem> = app
        .sessions
        .iter()
        .enumerate()
        .map(|(idx, session)| {
            let is_selected = idx == app.selection.session_idx;

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

            let fav_span = if session.is_favorite {
                Span::styled("★ ", theme.favorite)
            } else {
                Span::raw("")
            };

            let attached_span = if session.attached {
                Span::styled("● ", theme.attached_session)
            } else {
                Span::styled("○ ", theme.detached_session)
            };

            let name_style = if is_selected {
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else if session.attached {
                Style::default().fg(Color::White)
            } else {
                Style::default().fg(Color::Gray)
            };

            let name_span = Span::styled(&session.name, name_style);

            let win_count_str = format!(" {}", session.windows.len());
            let count_span = Span::styled(win_count_str, theme.dim);

            let line = Line::from(vec![
                cursor_span,
                fav_span,
                attached_span,
                name_span,
                Span::raw(" "),
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
