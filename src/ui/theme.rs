use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, BorderType, Borders};

pub struct Theme {
    pub border_style: Style,
    pub border_focused: Style,
    pub border_type: BorderType,
    pub title: Style,
    pub title_focused: Style,
    pub selection: Style,
    pub attached_session: Style,
    pub detached_session: Style,
    pub favorite: Style,
    pub active_item: Style,
    pub dim: Style,
    pub error: Style,
    pub success: Style,
    pub warning: Style,
    pub info: Style,
    pub breadcrumb_label: Style,
    pub breadcrumb_val: Style,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            border_style: Style::default().fg(Color::DarkGray),
            border_focused: Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            border_type: BorderType::Rounded,
            title: Style::default()
                .fg(Color::Gray)
                .add_modifier(Modifier::BOLD),
            title_focused: Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            selection: Style::default()
                .bg(Color::Rgb(30, 45, 65))
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
            attached_session: Style::default().fg(Color::Green),
            detached_session: Style::default().fg(Color::DarkGray),
            favorite: Style::default().fg(Color::Yellow),
            active_item: Style::default()
                .fg(Color::LightCyan)
                .add_modifier(Modifier::BOLD),
            dim: Style::default().fg(Color::DarkGray),
            error: Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            success: Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
            warning: Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
            info: Style::default().fg(Color::Cyan),
            breadcrumb_label: Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            breadcrumb_val: Style::default().fg(Color::White),
        }
    }
}

impl Theme {
    pub fn block<'a>(&self, title: &'a str, focused: bool) -> Block<'a> {
        let border_style = if focused {
            self.border_focused
        } else {
            self.border_style
        };
        let title_style = if focused {
            self.title_focused
        } else {
            self.title
        };

        Block::default()
            .borders(Borders::ALL)
            .border_type(self.border_type)
            .border_style(border_style)
            .title(format!(" {title} "))
            .title_style(title_style)
    }
}
