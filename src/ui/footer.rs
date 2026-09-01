use crate::action::ToastLevel;
use crate::app::{App, FocusColumn};
use crate::ui::theme::Theme;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub fn render_header(app: &App, frame: &mut Frame, area: Rect, _theme: &Theme) {
    let total_sessions = app.sessions.len();
    let total_windows: usize = app.sessions.iter().map(|s| s.windows.len()).sum();
    let total_panes: usize = app.sessions.iter().map(|s| s.total_panes()).sum();

    let stats = format!(
        " {} sessions · {} windows · {} panes",
        total_sessions, total_windows, total_panes
    );

    let mode_str = if app.is_mock { " [MOCK MODE]" } else { "" };

    let header_line = Line::from(vec![
        Span::styled(
            " 󰒋 LazyTmux",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
        Span::styled(mode_str, Style::default().fg(Color::Yellow)),
        Span::styled(" │", Style::default().fg(Color::DarkGray)),
        Span::styled(stats, Style::default().fg(Color::Gray)),
    ]);

    let header_widget = Paragraph::new(header_line);
    frame.render_widget(header_widget, area);
}

pub fn render_breadcrumbs(app: &App, frame: &mut Frame, area: Rect, theme: &Theme) {
    let s_name = app.selected_session().map(|s| s.name.as_str()).unwrap_or("-");
    let w_name = app.selected_window().map(|w| w.name.as_str()).unwrap_or("-");
    let (p_id, p_cmd, p_path) = match app.selected_pane() {
        Some(p) => (
            p.id.0.as_str(),
            p.current_command.as_str(),
            p.current_path.to_string_lossy().to_string(),
        ),
        None => ("-", "-", "-".to_string()),
    };

    let is_attached = app.selected_session().map(|s| s.attached).unwrap_or(false);
    let attached_span = if is_attached {
        Span::styled(" attached ●", theme.attached_session)
    } else {
        Span::styled(" detached ○", theme.detached_session)
    };

    let breadcrumb_line = Line::from(vec![
        Span::raw(" "),
        Span::styled(s_name, theme.breadcrumb_label),
        Span::styled(" › ", theme.dim),
        Span::styled(w_name, theme.breadcrumb_label),
        Span::styled(" › ", theme.dim),
        Span::styled(p_id, theme.breadcrumb_label),
        Span::raw("   "),
        Span::styled(p_cmd, theme.breadcrumb_val),
        Span::raw("   "),
        Span::styled(p_path, theme.dim),
        Span::raw("   "),
        attached_span,
    ]);

    let widget = Paragraph::new(breadcrumb_line);
    frame.render_widget(widget, area);
}

pub fn render_footer(app: &App, frame: &mut Frame, area: Rect, theme: &Theme) {
    // If there is an active toast, show it on the right side
    let toast_span = if let Some(toast) = app.toasts.last() {
        let (color, icon) = match toast.level {
            ToastLevel::Info => (Color::Cyan, "ℹ "),
            ToastLevel::Success => (Color::Green, "✔ "),
            ToastLevel::Warning => (Color::Yellow, "⚠ "),
            ToastLevel::Error => (Color::Red, "✖ "),
        };
        Span::styled(
            format!(" [ {}{} ] ", icon, toast.message),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        )
    } else {
        Span::raw("")
    };

    let hints = match app.focus {
        FocusColumn::Sessions => vec![
            Span::styled(" h/l ", Style::default().bg(Color::DarkGray).fg(Color::White)),
            Span::raw(" col "),
            Span::styled(" j/k ", Style::default().bg(Color::DarkGray).fg(Color::White)),
            Span::raw(" move "),
            Span::styled(" Enter ", Style::default().bg(Color::Cyan).fg(Color::Black).add_modifier(Modifier::BOLD)),
            Span::raw(" attach "),
            Span::styled(" n ", Style::default().bg(Color::DarkGray).fg(Color::White)),
            Span::raw(" new "),
            Span::styled(" R ", Style::default().bg(Color::DarkGray).fg(Color::White)),
            Span::raw(" rename "),
            Span::styled(" x ", Style::default().bg(Color::DarkGray).fg(Color::White)),
            Span::raw(" kill "),
            Span::styled(" f ", Style::default().bg(Color::DarkGray).fg(Color::White)),
            Span::raw(" fav "),
            Span::styled(" / ", Style::default().bg(Color::DarkGray).fg(Color::White)),
            Span::raw(" search "),
            Span::styled(" ? ", Style::default().bg(Color::DarkGray).fg(Color::White)),
            Span::raw(" help"),
        ],
        FocusColumn::Windows => vec![
            Span::styled(" h/l ", Style::default().bg(Color::DarkGray).fg(Color::White)),
            Span::raw(" col "),
            Span::styled(" j/k ", Style::default().bg(Color::DarkGray).fg(Color::White)),
            Span::raw(" move "),
            Span::styled(" Enter ", Style::default().bg(Color::Cyan).fg(Color::Black).add_modifier(Modifier::BOLD)),
            Span::raw(" select "),
            Span::styled(" n ", Style::default().bg(Color::DarkGray).fg(Color::White)),
            Span::raw(" new "),
            Span::styled(" R ", Style::default().bg(Color::DarkGray).fg(Color::White)),
            Span::raw(" rename "),
            Span::styled(" x ", Style::default().bg(Color::DarkGray).fg(Color::White)),
            Span::raw(" kill "),
            Span::styled(" / ", Style::default().bg(Color::DarkGray).fg(Color::White)),
            Span::raw(" search "),
            Span::styled(" ? ", Style::default().bg(Color::DarkGray).fg(Color::White)),
            Span::raw(" help"),
        ],
        FocusColumn::Panes => vec![
            Span::styled(" h/l ", Style::default().bg(Color::DarkGray).fg(Color::White)),
            Span::raw(" col "),
            Span::styled(" j/k ", Style::default().bg(Color::DarkGray).fg(Color::White)),
            Span::raw(" move "),
            Span::styled(" Enter ", Style::default().bg(Color::Cyan).fg(Color::Black).add_modifier(Modifier::BOLD)),
            Span::raw(" focus "),
            Span::styled(" n ", Style::default().bg(Color::DarkGray).fg(Color::White)),
            Span::raw(" new "),
            Span::styled(" Space ", Style::default().bg(Color::DarkGray).fg(Color::White)),
            Span::raw(" inspect "),
            Span::styled(" z ", Style::default().bg(Color::DarkGray).fg(Color::White)),
            Span::raw(" zoom "),
            Span::styled(" c ", Style::default().bg(Color::DarkGray).fg(Color::White)),
            Span::raw(" copy "),
            Span::styled(" x ", Style::default().bg(Color::DarkGray).fg(Color::White)),
            Span::raw(" kill "),
            Span::styled(" ? ", Style::default().bg(Color::DarkGray).fg(Color::White)),
            Span::raw(" help"),
        ],
    };

    let mut footer_spans = vec![Span::raw(" ")];
    footer_spans.extend(hints);
    if !toast_span.content.is_empty() {
        footer_spans.push(Span::raw("   "));
        footer_spans.push(toast_span);
    }

    let line = Line::from(footer_spans);
    let widget = Paragraph::new(line).style(theme.dim);
    frame.render_widget(widget, area);
}
