use crate::app::{App, SearchCategory};
use crate::ui::theme::Theme;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph};

pub fn render(app: &App, frame: &mut Frame, area: Rect, theme: &Theme) {
    let (query, selected_index, current_category) = match &app.mode {
        crate::app::Mode::Search {
            query,
            selected_index,
            category,
        } => (query, *selected_index, *category),
        _ => return,
    };

    let overlay_area = centered_rect(85, 75, area);
    frame.render_widget(Clear, overlay_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Input box & Category Tabs
            Constraint::Min(4),    // Results list
            Constraint::Length(1), // Footer hint
        ])
        .split(overlay_area);

    // Categories tabs in title
    let mut title_spans = vec![Span::styled(
        " Fuzzy Search  ",
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )];
    for cat in &[
        SearchCategory::All,
        SearchCategory::Sessions,
        SearchCategory::Windows,
        SearchCategory::Panes,
    ] {
        if *cat == current_category {
            title_spans.push(Span::styled(format!(" [{}] ", cat.name()), theme.selection));
        } else {
            title_spans.push(Span::styled(format!(" {} ", cat.name()), theme.dim));
        }
        title_spans.push(Span::raw(" "));
    }
    title_spans.push(Span::styled("(Tab)", theme.dim));

    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_type(theme.border_type)
        .border_style(theme.border_focused)
        .title(Line::from(title_spans));

    let input_line = Line::from(vec![
        Span::styled(" > ", theme.title_focused),
        Span::styled(
            query,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("█", theme.title_focused),
    ]);

    let input_widget = Paragraph::new(input_line).block(input_block);
    frame.render_widget(input_widget, chunks[0]);

    // Results List
    let results = app.filtered_search_results(query, current_category);

    let results_block = Block::default()
        .borders(Borders::ALL)
        .border_type(theme.border_type)
        .border_style(theme.border_style)
        .title(format!(" Results ({}) ", results.len()));

    if results.is_empty() {
        let empty_msg = Paragraph::new(" No matching sessions, windows, or panes found")
            .style(theme.dim)
            .block(results_block);
        frame.render_widget(empty_msg, chunks[1]);
    } else {
        let items: Vec<ListItem> = results
            .iter()
            .enumerate()
            .map(|(idx, item)| {
                let is_selected = idx == selected_index;

                let cursor_span = if is_selected {
                    Span::styled("▶ ", theme.title_focused)
                } else {
                    Span::raw("  ")
                };

                let category_tag = match item.category {
                    SearchCategory::Sessions => Span::styled("[Session] ", theme.favorite),
                    SearchCategory::Windows => Span::styled("[Window]  ", theme.success),
                    SearchCategory::Panes | SearchCategory::All => {
                        Span::styled("[Pane]    ", theme.info)
                    }
                };

                let session_span = Span::styled(
                    format!("{:<12} ", item.session_name),
                    Style::default().fg(Color::Yellow),
                );

                let window_span = Span::styled(
                    format!("{:<12} ", item.window_name),
                    Style::default().fg(Color::Green),
                );

                let pane_span =
                    Span::styled(format!("{:<6} ", item.pane_id.0), theme.title_focused);

                let cmd_span = Span::styled(
                    format!("{:<14} ", item.command),
                    Style::default().fg(Color::White),
                );

                let branch_span = item
                    .git_branch
                    .as_deref()
                    .map(|b| Span::styled(format!(" [{b}]"), theme.info))
                    .unwrap_or_else(|| Span::raw(""));

                let path_span = Span::styled(format!(" {}", item.path), theme.dim);

                let line = Line::from(vec![
                    cursor_span,
                    category_tag,
                    session_span,
                    window_span,
                    pane_span,
                    cmd_span,
                    branch_span,
                    path_span,
                ]);

                let list_item = ListItem::new(line);
                if is_selected {
                    list_item.style(theme.selection)
                } else {
                    list_item
                }
            })
            .collect();

        let list_widget = List::new(items).block(results_block);
        frame.render_widget(list_widget, chunks[1]);
    }

    // Footer Hint
    let footer_line = Line::from(vec![
        Span::styled(
            " Enter ",
            Style::default()
                .bg(Color::Cyan)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Jump to Pane  "),
        Span::styled(
            " Tab / Shift+Tab ",
            Style::default().bg(Color::DarkGray).fg(Color::White),
        ),
        Span::raw(" Filter Category  "),
        Span::styled(
            " ↑/↓ / Ctrl+p/n ",
            Style::default().bg(Color::DarkGray).fg(Color::White),
        ),
        Span::raw(" Navigate  "),
        Span::styled(
            " Esc ",
            Style::default().bg(Color::DarkGray).fg(Color::White),
        ),
        Span::raw(" Close"),
    ]);

    let footer_widget = Paragraph::new(footer_line).style(theme.dim);
    frame.render_widget(footer_widget, chunks[2]);
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
