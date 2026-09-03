use crate::app::{App, KillTarget, Mode};
use crate::ui::theme::Theme;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};

pub fn render(app: &App, frame: &mut Frame, area: Rect, theme: &Theme) {
    match &app.mode {
        Mode::ConfirmKill(target) => render_confirm_kill(target, frame, area, theme),
        Mode::PromptNewSession { input } => render_input_prompt(
            "New Session",
            "Enter session name:",
            input,
            frame,
            area,
            theme,
        ),
        Mode::PromptNewWindow { input, .. } => render_input_prompt(
            "New Window",
            "Enter window name:",
            input,
            frame,
            area,
            theme,
        ),
        Mode::PromptNewPane { pane_id } => render_new_pane(pane_id, frame, area, theme),
        Mode::PromptRenameSession { input, .. } => render_input_prompt(
            "Rename Session",
            "Enter new name:",
            input,
            frame,
            area,
            theme,
        ),
        Mode::PromptRenameWindow { input, .. } => render_input_prompt(
            "Rename Window",
            "Enter new name:",
            input,
            frame,
            area,
            theme,
        ),
        Mode::PromptSendCommand {
            pane_id,
            input,
            with_enter,
        } => render_send_command_prompt(pane_id, input, *with_enter, frame, area, theme),
        Mode::Help => render_help(frame, area, theme),
        _ => {}
    }
}

fn render_new_pane(pane_id: &crate::domain::PaneId, frame: &mut Frame, area: Rect, theme: &Theme) {
    let overlay_area = centered_rect(58, 30, area);
    frame.render_widget(Clear, overlay_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .title(" New Pane Split ")
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Min(4), Constraint::Length(1)])
        .split(block.inner(overlay_area));

    frame.render_widget(block, overlay_area);

    let lines = vec![
        Line::from(vec![
            Span::raw("Split target pane "),
            Span::styled(
                &pane_id.0,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(":"),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled(
                " [v] ",
                Style::default()
                    .bg(Color::Cyan)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Vertical Split   (side-by-side columns)"),
        ]),
        Line::from(vec![
            Span::styled(
                " [h] ",
                Style::default()
                    .bg(Color::Cyan)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Horizontal Split (stacked top/bottom)"),
        ]),
    ];

    let msg_widget = Paragraph::new(lines);
    frame.render_widget(msg_widget, chunks[0]);

    let prompt_line = Line::from(vec![
        Span::styled(
            " [v/h] ",
            Style::default()
                .bg(Color::Cyan)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Choose Split   "),
        Span::styled(
            " [Esc] ",
            Style::default().bg(Color::DarkGray).fg(Color::White),
        ),
        Span::raw(" Cancel"),
    ]);

    let prompt_widget = Paragraph::new(prompt_line).style(theme.dim);
    frame.render_widget(prompt_widget, chunks[1]);
}

fn render_confirm_kill(target: &KillTarget, frame: &mut Frame, area: Rect, theme: &Theme) {
    let overlay_area = centered_rect(55, 30, area);
    frame.render_widget(Clear, overlay_area);

    let (title, message) = match target {
        KillTarget::Session(id, name) => (
            " Kill Session ",
            format!(
                "Are you sure you want to kill session \"{name}\" ({id})?\nAll windows and running panes will be terminated."
            ),
        ),
        KillTarget::Window(id, name) => (
            " Kill Window ",
            format!(
                "Are you sure you want to kill window \"{name}\" ({id})?\nAll panes in this window will be closed."
            ),
        ),
        KillTarget::Pane(id, cmd) => (
            " Kill Pane ",
            format!("Are you sure you want to kill pane {id} running \"{cmd}\"?"),
        ),
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
        .title(title)
        .title_style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Min(2), Constraint::Length(1)])
        .split(block.inner(overlay_area));

    frame.render_widget(block, overlay_area);

    let msg_widget = Paragraph::new(message).style(Style::default().fg(Color::White));
    frame.render_widget(msg_widget, chunks[0]);

    let prompt_line = Line::from(vec![
        Span::styled(
            " [y/Enter] ",
            Style::default()
                .bg(Color::Red)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Confirm Kill   "),
        Span::styled(
            " [n/Esc] ",
            Style::default().bg(Color::DarkGray).fg(Color::White),
        ),
        Span::raw(" Cancel"),
    ]);

    let prompt_widget = Paragraph::new(prompt_line).style(theme.dim);
    frame.render_widget(prompt_widget, chunks[1]);
}

fn render_input_prompt(
    title: &str,
    prompt: &str,
    input: &str,
    frame: &mut Frame,
    area: Rect,
    _theme: &Theme,
) {
    let overlay_area = centered_rect(50, 25, area);
    frame.render_widget(Clear, overlay_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .title(format!(" {title} "))
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1), // Prompt label
            Constraint::Length(3), // Input box
            Constraint::Length(1), // Key hint
        ])
        .split(block.inner(overlay_area));

    frame.render_widget(block, overlay_area);

    let prompt_widget = Paragraph::new(prompt).style(Style::default().fg(Color::Gray));
    frame.render_widget(prompt_widget, chunks[0]);

    let input_box = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::White));

    let input_line = Line::from(vec![
        Span::styled(
            input,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("█", Style::default().fg(Color::Cyan)),
    ]);

    let input_widget = Paragraph::new(input_line).block(input_box);
    frame.render_widget(input_widget, chunks[1]);

    let hint_line = Line::from(vec![
        Span::styled(
            " Enter ",
            Style::default()
                .bg(Color::Cyan)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Submit   "),
        Span::styled(
            " Esc ",
            Style::default().bg(Color::DarkGray).fg(Color::White),
        ),
        Span::raw(" Cancel"),
    ]);

    let hint_widget = Paragraph::new(hint_line).style(Style::default().fg(Color::DarkGray));
    frame.render_widget(hint_widget, chunks[2]);
}

pub fn render_send_command_prompt(
    pane_id: &crate::domain::PaneId,
    input: &str,
    with_enter: bool,
    frame: &mut Frame,
    area: Rect,
    _theme: &Theme,
) {
    let overlay_area = centered_rect(58, 26, area);
    frame.render_widget(Clear, overlay_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .title(format!(" Send Prompt / Command to Pane {} ", pane_id.0))
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1), // Prompt label
            Constraint::Length(3), // Input box
            Constraint::Length(1), // Mode toggle indicator
            Constraint::Length(1), // Action hints
        ])
        .split(block.inner(overlay_area));

    frame.render_widget(block, overlay_area);

    let prompt_label =
        Paragraph::new("Enter command or prompt for interactive text fields (Claude/Codex/AGY):")
            .style(Style::default().fg(Color::Gray));
    frame.render_widget(prompt_label, chunks[0]);

    let input_box = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::White));

    let input_line = Line::from(vec![
        Span::styled(
            input,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("█", Style::default().fg(Color::Cyan)),
    ]);
    frame.render_widget(Paragraph::new(input_line).block(input_box), chunks[1]);

    // Mode Toggle line
    let mode_spans = if with_enter {
        vec![
            Span::raw("Mode: "),
            Span::styled(
                " [● With Enter ↵] ",
                Style::default()
                    .bg(Color::Cyan)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(" [○ Normal] ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                "   (Press Tab to switch to Normal)",
                Style::default().fg(Color::DarkGray),
            ),
        ]
    } else {
        vec![
            Span::raw("Mode: "),
            Span::styled(" [○ With Enter ↵] ", Style::default().fg(Color::DarkGray)),
            Span::raw("  "),
            Span::styled(
                " [● Normal] ",
                Style::default()
                    .bg(Color::Cyan)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "   (Press Tab to switch to With Enter ↵)",
                Style::default().fg(Color::DarkGray),
            ),
        ]
    };
    frame.render_widget(Paragraph::new(Line::from(mode_spans)), chunks[2]);

    // Keyboard hints
    let hint_line = Line::from(vec![
        Span::styled(
            " Enter ",
            Style::default()
                .bg(Color::Cyan)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Send   "),
        Span::styled(
            " Tab ",
            Style::default().bg(Color::DarkGray).fg(Color::White),
        ),
        Span::raw(" Toggle Mode   "),
        Span::styled(
            " Ctrl+E ",
            Style::default()
                .bg(Color::Yellow)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Send with ↵   "),
        Span::styled(
            " Esc ",
            Style::default().bg(Color::DarkGray).fg(Color::White),
        ),
        Span::raw(" Cancel"),
    ]);
    frame.render_widget(Paragraph::new(hint_line), chunks[3]);
}

fn render_help(frame: &mut Frame, area: Rect, theme: &Theme) {
    let overlay_area = centered_rect(75, 75, area);
    frame.render_widget(Clear, overlay_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .title(" LazyTmux Keybindings & Help ")
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    let inner = block.inner(overlay_area);
    frame.render_widget(block, overlay_area);

    let help_text = vec![
        Line::from(vec![Span::styled(
            "GLOBAL NAVIGATION",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("  h / l, ← / →, Tab     ", Style::default().fg(Color::Cyan)),
            Span::raw("Switch column focus (Sessions, Windows, Panes)"),
        ]),
        Line::from(vec![
            Span::styled("  j / k, ↓ / ↑          ", Style::default().fg(Color::Cyan)),
            Span::raw("Move selection up / down"),
        ]),
        Line::from(vec![
            Span::styled("  Enter                 ", Style::default().fg(Color::Cyan)),
            Span::raw("Focus selection / Attach to workspace"),
        ]),
        Line::from(vec![
            Span::styled("  /                     ", Style::default().fg(Color::Cyan)),
            Span::raw("Global fuzzy search (sessions, windows, panes)"),
        ]),
        Line::from(vec![
            Span::styled("  r                     ", Style::default().fg(Color::Cyan)),
            Span::raw("Force refresh tmux state"),
        ]),
        Line::from(vec![
            Span::styled("  < / >, , / .          ", Style::default().fg(Color::Cyan)),
            Span::raw("Resize focused column width (Sessions, Windows, Panes)"),
        ]),
        Line::from(vec![
            Span::styled("  Mouse Drag Borders    ", Style::default().fg(Color::Cyan)),
            Span::raw("Drag column separator borders to resize 3 parts"),
        ]),
        Line::from(vec![
            Span::styled(
                r"  \ / |                 ",
                Style::default().fg(Color::Cyan),
            ),
            Span::raw("Toggle sidebar collapse: Full → Sessions collapsed → Wide Panes"),
        ]),
        Line::from(vec![
            Span::styled("  Header [◀] / [▶]      ", Style::default().fg(Color::Cyan)),
            Span::raw("Click header buttons to collapse or expand sidebars"),
        ]),
        Line::from(vec![
            Span::styled("  Double Click          ", Style::default().fg(Color::Cyan)),
            Span::raw("Attach to session / Focus window / Handoff to pane (Enter)"),
        ]),
        Line::from(vec![
            Span::styled("  ?                     ", Style::default().fg(Color::Cyan)),
            Span::raw("Toggle this help modal"),
        ]),
        Line::from(vec![
            Span::styled("  q / Esc               ", Style::default().fg(Color::Cyan)),
            Span::raw("Quit LazyTmux"),
        ]),
        Line::raw(""),
        Line::from(vec![Span::styled(
            "ACTIONS BY COLUMN",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled(
                "  n                     ",
                Style::default().fg(Color::Green),
            ),
            Span::raw("New session (in Sessions) / New window (in Windows)"),
        ]),
        Line::from(vec![
            Span::styled(
                "  r / R / F2            ",
                Style::default().fg(Color::Green),
            ),
            Span::raw("Rename selected session / window"),
        ]),
        Line::from(vec![
            Span::styled("  x                     ", Style::default().fg(Color::Red)),
            Span::raw("Kill selected session / window / pane (with confirm)"),
        ]),
        Line::from(vec![
            Span::styled("  t                     ", Style::default().fg(Color::Cyan)),
            Span::raw("Toggle theme preset live (Tokyo Night, Catppuccin, Nord, Gruvbox, etc.)"),
        ]),
        Line::from(vec![
            Span::styled(
                "  :                     ",
                Style::default().fg(Color::Yellow),
            ),
            Span::raw("Send command / keystrokes direct to background pane"),
        ]),
        Line::from(vec![
            Span::styled(
                "  b                     ",
                Style::default().fg(Color::Green),
            ),
            Span::raw("Break pane into a new window (break-pane)"),
        ]),
        Line::from(vec![
            Span::styled("  l                     ", Style::default().fg(Color::Cyan)),
            Span::raw("Cycle layout preset (even-h, even-v, main-h, main-v, tiled)"),
        ]),
        Line::from(vec![
            Span::styled("  s                     ", Style::default().fg(Color::Cyan)),
            Span::raw("Toggle synchronize-panes (broadcast typing to all panes)"),
        ]),
        Line::from(vec![
            Span::styled("  [ / ]                 ", Style::default().fg(Color::Cyan)),
            Span::raw("Swap pane up/down (Panes) or Move window left/right (Windows)"),
        ]),
        Line::from(vec![
            Span::styled(
                "  + / -                 ",
                Style::default().fg(Color::Green),
            ),
            Span::raw("Resize selected pane vertically (grow/shrink)"),
        ]),
        Line::from(vec![
            Span::styled(
                "  Shift + ←/→/↑/↓, H/J/K/L",
                Style::default().fg(Color::Green),
            ),
            Span::raw("Resize selected pane in 4 directions"),
        ]),
        Line::from(vec![
            Span::styled(
                "  Mouse Drag / Buttons  ",
                Style::default().fg(Color::Yellow),
            ),
            Span::raw("Drag pane to resize, or click [◀][▼][▲][▶][↕ swap] controls"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl + r              ", Style::default().fg(Color::Cyan)),
            Span::raw("Respawn pane process (restart crashed shell)"),
        ]),
        Line::from(vec![
            Span::styled(
                "  f                     ",
                Style::default().fg(Color::Yellow),
            ),
            Span::raw("Toggle favorite on selected session"),
        ]),
        Line::from(vec![
            Span::styled("  z / Space             ", Style::default().fg(Color::Cyan)),
            Span::raw("Zoom / inspect full-screen scrollback for selected pane"),
        ]),
        Line::from(vec![
            Span::styled("  c                     ", Style::default().fg(Color::Cyan)),
            Span::raw("Copy pane output to system clipboard"),
        ]),
        Line::raw(""),
        Line::from(vec![Span::styled(
            "INSPECT MODE (Space / z)",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled(
                "  j / k, Ctrl+d / Ctrl+u ",
                Style::default().fg(Color::Cyan),
            ),
            Span::raw("Scroll up / down buffer history"),
        ]),
        Line::from(vec![
            Span::styled("  g / G                 ", Style::default().fg(Color::Cyan)),
            Span::raw("Jump to top / bottom"),
        ]),
        Line::from(vec![
            Span::styled(
                "  /                     ",
                Style::default().fg(Color::Green),
            ),
            Span::raw("Search inside buffer history"),
        ]),
        Line::from(vec![
            Span::styled(
                "  n / N                 ",
                Style::default().fg(Color::Green),
            ),
            Span::raw("Jump to next / previous search match"),
        ]),
        Line::from(vec![
            Span::styled("  c                     ", Style::default().fg(Color::Cyan)),
            Span::raw("Copy entire buffer to clipboard"),
        ]),
        Line::from(vec![
            Span::styled("  Esc / Space           ", Style::default().fg(Color::Cyan)),
            Span::raw("Exit inspect mode"),
        ]),
    ];

    let paragraph = Paragraph::new(help_text).style(theme.dim);
    frame.render_widget(paragraph, inner);
}

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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
