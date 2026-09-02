use crate::app::App;
use crate::ui::theme::Theme;
use ansi_to_tui::IntoText;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};

pub fn render(app: &App, frame: &mut Frame, area: Rect, theme: &Theme) {
    let (pane_id, scroll_offset, search_query, is_searching) = match &app.mode {
        crate::app::Mode::InspectPane {
            pane_id,
            scroll_offset,
            search_query,
            is_searching,
        } => (
            pane_id,
            *scroll_offset,
            search_query.as_deref(),
            *is_searching,
        ),
        _ => return,
    };

    let pane = match app.selected_window().and_then(|w| w.get_pane(pane_id)) {
        Some(p) => p,
        None => return,
    };

    let s_name = app
        .selected_session()
        .map(|s| s.name.as_str())
        .unwrap_or("?");
    let w_name = app
        .selected_window()
        .map(|w| w.name.as_str())
        .unwrap_or("?");

    let title = format!(
        " Inspect: {} › {} › {} ({}) ",
        s_name, w_name, pane.id.0, pane.current_command
    );

    // Compute overlay size (88% width, 85% height)
    let overlay_area = centered_rect(88, 85, area);
    frame.render_widget(Clear, overlay_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(5),    // Body
            Constraint::Length(1), // Keybindings footer / search bar
        ])
        .split(overlay_area);

    let total_lines = pane.preview_lines.len();
    let mut line_info = format!(" [Line {}/{}] ", scroll_offset + 1, total_lines.max(1));
    if let Some(q) = search_query {
        let q_lower = q.to_lowercase();
        let match_count = pane
            .preview_lines
            .iter()
            .filter(|l| l.to_lowercase().contains(&q_lower))
            .count();
        line_info = format!(
            " [Line {}/{} · \"{}\": {} matches (n/N)] ",
            scroll_offset + 1,
            total_lines.max(1),
            q,
            match_count
        );
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .title(title)
        .title_style(
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .title_bottom(line_info);

    // Slice preview lines based on scroll offset
    let visible_lines: Vec<String> = pane
        .preview_lines
        .iter()
        .skip(scroll_offset)
        .cloned()
        .collect();

    let joined = visible_lines.join("\n");
    let content_text = joined
        .as_bytes()
        .into_text()
        .unwrap_or_else(|_| Text::raw(joined));

    let paragraph = Paragraph::new(content_text).block(block);
    frame.render_widget(paragraph, chunks[0]);

    // Footer or Search Bar
    if is_searching {
        let q_display = search_query.unwrap_or("");
        let search_line = Line::from(vec![
            Span::styled(
                " / Search: ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                q_display,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("█", Style::default().fg(Color::Yellow)),
            Span::styled(
                "  (Enter: finish · Esc: cancel)",
                Style::default().fg(Color::DarkGray),
            ),
        ]);
        let search_widget = Paragraph::new(search_line);
        frame.render_widget(search_widget, chunks[1]);
    } else {
        let mut footer_spans = vec![
            Span::styled(
                " Esc ",
                Style::default().bg(Color::DarkGray).fg(Color::White),
            ),
            Span::raw(" Back  "),
            Span::styled(
                " j/k ",
                Style::default().bg(Color::DarkGray).fg(Color::White),
            ),
            Span::raw(" Scroll  "),
            Span::styled(
                " Ctrl+d/u ",
                Style::default().bg(Color::DarkGray).fg(Color::White),
            ),
            Span::raw(" Page  "),
            Span::styled(" / ", Style::default().bg(Color::DarkGray).fg(Color::White)),
            Span::raw(" Search  "),
        ];

        if search_query.is_some() {
            footer_spans.push(Span::styled(
                " n/N ",
                Style::default().bg(Color::DarkGray).fg(Color::White),
            ));
            footer_spans.push(Span::raw(" Next/Prev  "));
        }

        footer_spans.extend(vec![
            Span::styled(" c ", Style::default().bg(Color::DarkGray).fg(Color::White)),
            Span::raw(" Copy  "),
            Span::styled(
                " Enter ",
                Style::default()
                    .bg(Color::Cyan)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Focus in Tmux"),
        ]);

        let footer_widget = Paragraph::new(Line::from(footer_spans)).style(theme.dim);
        frame.render_widget(footer_widget, chunks[1]);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
