use crate::app::{App, ConfirmTarget, Mode};
use crate::ui::theme::Theme;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap};

pub fn render(app: &App, frame: &mut Frame, area: Rect, theme: &Theme) {
    match &app.mode {
        Mode::Confirm(target) => render_confirm(target, frame, area, theme),
        Mode::PromptNewSession { input } => render_input_prompt(
            "New Session",
            "Enter session name:",
            input,
            frame,
            area,
            theme,
            false,
        ),
        Mode::PromptNewWindow { input, .. } => render_input_prompt(
            "New Window",
            "Enter window name:",
            input,
            frame,
            area,
            theme,
            false,
        ),
        Mode::PromptNewPane { pane_id } => render_new_pane(pane_id, frame, area, theme),
        Mode::PromptRenameSession { input, .. } => render_input_prompt(
            "Rename Session",
            "Enter new name:",
            input,
            frame,
            area,
            theme,
            false,
        ),
        Mode::PromptRenameWindow { input, .. } => render_input_prompt(
            "Rename Window",
            "Enter new name:",
            input,
            frame,
            area,
            theme,
            false,
        ),
        Mode::PromptSendCommand {
            pane_id,
            input,
            broadcast,
        } => {
            let (title, prompt) = if *broadcast {
                (
                    format!("Send to ALL panes (sync on) — {}", pane_id.0),
                    "synchronize-panes is ON: this runs in EVERY pane of the window.",
                )
            } else {
                (
                    format!("Send to Pane {}", pane_id.0),
                    "Command / prompt (Enter executes in pane):",
                )
            };
            render_input_prompt(&title, prompt, input, frame, area, theme, *broadcast)
        }
        Mode::Help => render_help(frame, area, theme),
        _ => {}
    }
}

fn render_new_pane(pane_id: &crate::domain::PaneId, frame: &mut Frame, area: Rect, theme: &Theme) {
    let overlay_area = centered_fixed(52, 9, area);
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

/// A clickable choice in the confirmation dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmButton {
    Yes,
    No,
}

struct ButtonSpec {
    key: &'static str,
    text: &'static str,
    button: ConfirmButton,
}

/// The dialog's buttons, widest form first. Drawing and hit-testing both walk
/// the same list, so a button is always clickable exactly where it is painted.
/// The key badge and its word form one hit region, to give the mouse a bigger
/// target.
const BUTTONS_FULL: &[ButtonSpec] = &[
    ButtonSpec {
        key: " [y/Enter] ",
        text: " Yes ",
        button: ConfirmButton::Yes,
    },
    ButtonSpec {
        key: " [n/Esc] ",
        text: " No ",
        button: ConfirmButton::No,
    },
];

const BUTTONS_COMPACT: &[ButtonSpec] = &[
    ButtonSpec {
        key: " [y] ",
        text: " Yes ",
        button: ConfirmButton::Yes,
    },
    ButtonSpec {
        key: " [n] ",
        text: " No ",
        button: ConfirmButton::No,
    },
];

/// Last resort: the choices themselves, with the keys left to the help text.
/// A dialog must never be too narrow to answer.
const BUTTONS_MINIMAL: &[ButtonSpec] = &[
    ButtonSpec {
        key: "",
        text: " Yes ",
        button: ConfirmButton::Yes,
    },
    ButtonSpec {
        key: "",
        text: " No ",
        button: ConfirmButton::No,
    },
];

const BUTTON_GAP: &str = "   ";

fn buttons_width(specs: &[ButtonSpec]) -> u16 {
    use unicode_width::UnicodeWidthStr;
    let content: usize = specs.iter().map(|s| s.key.width() + s.text.width()).sum();
    let gaps = specs.len().saturating_sub(1) * BUTTON_GAP.width();
    (content + gaps) as u16
}

/// The widest button set that fits `width` columns.
fn buttons_for(width: u16) -> &'static [ButtonSpec] {
    for specs in [BUTTONS_FULL, BUTTONS_COMPACT] {
        if buttons_width(specs) <= width {
            return specs;
        }
    }
    BUTTONS_MINIMAL
}

/// Where the confirmation dialog and its parts are drawn inside `area`.
pub struct ConfirmLayout {
    pub overlay: Rect,
    pub message: Rect,
    pub buttons: Rect,
}

pub fn confirm_layout(area: Rect) -> ConfirmLayout {
    // Fixed frame: the message wraps to fit rather than stretching the box,
    // which also keeps the button geometry independent of the message text.
    let overlay = centered_fixed(64, 9, area);
    let inner = Block::default().borders(Borders::ALL).inner(overlay);
    // Min(1), not Min(2): on a short terminal a two-row minimum for the
    // message left no row for the buttons, so they were silently clipped and
    // the dialog could not be answered with the mouse at all.
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);
    ConfirmLayout {
        overlay,
        message: chunks[0],
        buttons: chunks[1],
    }
}

/// The confirmation button at a screen position, if any.
pub fn confirm_button_at(area: Rect, column: u16, row: u16) -> Option<ConfirmButton> {
    use unicode_width::UnicodeWidthStr;

    let layout = confirm_layout(area);
    if layout.buttons.height == 0 || row != layout.buttons.y {
        return None;
    }

    let mut x = layout.buttons.x;
    for (idx, spec) in buttons_for(layout.buttons.width).iter().enumerate() {
        if idx > 0 {
            x += BUTTON_GAP.width() as u16;
        }
        let width = (spec.key.width() + spec.text.width()) as u16;
        if column >= x && column < x + width {
            return Some(spec.button);
        }
        x += width;
    }
    None
}

fn render_confirm(target: &ConfirmTarget, frame: &mut Frame, area: Rect, theme: &Theme) {
    let layout = confirm_layout(area);
    let overlay_area = layout.overlay;
    frame.render_widget(Clear, overlay_area);

    let (title, message) = match target {
        ConfirmTarget::KillSession(id, name) => (
            " Kill Session ",
            format!(
                "Are you sure you want to kill session \"{name}\" ({id})?\nAll windows and running panes will be terminated."
            ),
        ),
        ConfirmTarget::KillWindow(id, name) => (
            " Kill Window ",
            format!(
                "Are you sure you want to kill window \"{name}\" ({id})?\nAll panes in this window will be closed."
            ),
        ),
        ConfirmTarget::KillPane(id, cmd) => (
            " Kill Pane ",
            format!("Are you sure you want to kill pane {id} running \"{cmd}\"?"),
        ),
        ConfirmTarget::RespawnPane(id, cmd) => (
            " Respawn Pane ",
            format!("Respawn pane {id}?\nThis kills \"{cmd}\" and restarts the pane's command."),
        ),
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
        .title(title)
        .title_style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD));

    frame.render_widget(block, overlay_area);

    let msg_widget = Paragraph::new(message)
        .wrap(Wrap { trim: false })
        .style(Style::default().fg(Color::White));
    frame.render_widget(msg_widget, layout.message);

    let specs = buttons_for(layout.buttons.width);
    let mut spans = Vec::with_capacity(specs.len() * 3);
    for (idx, spec) in specs.iter().enumerate() {
        if idx > 0 {
            spans.push(Span::raw(BUTTON_GAP));
        }
        let (bg, fg) = match spec.button {
            ConfirmButton::Yes => (Color::Red, Color::White),
            ConfirmButton::No => (Color::DarkGray, Color::White),
        };
        spans.push(Span::styled(
            spec.key,
            Style::default().bg(bg).fg(fg).add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(
            spec.text,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ));
    }

    let prompt_widget = Paragraph::new(Line::from(spans)).style(theme.dim);
    frame.render_widget(prompt_widget, layout.buttons);
}

#[allow(clippy::too_many_arguments)]
fn render_input_prompt(
    title: &str,
    prompt: &str,
    input: &str,
    frame: &mut Frame,
    area: Rect,
    _theme: &Theme,
    warn: bool,
) {
    let overlay_area = prompt_overlay(area);
    frame.render_widget(Clear, overlay_area);

    // A destructive-by-default prompt is coloured like the kill dialog so it
    // cannot be mistaken for an ordinary text entry.
    let accent = if warn { Color::Yellow } else { Color::Cyan };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(accent).add_modifier(Modifier::BOLD))
        .title(format!(" {title} "))
        .title_style(Style::default().fg(accent).add_modifier(Modifier::BOLD));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(2), // Prompt label, two rows so it can wrap
            Constraint::Length(3), // Input box
            Constraint::Length(1), // Key hint
        ])
        .split(block.inner(overlay_area));

    frame.render_widget(block, overlay_area);

    let prompt_style = if warn {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };
    let prompt_widget = Paragraph::new(prompt)
        .wrap(Wrap { trim: false })
        .style(prompt_style);
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
                .bg(accent)
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
            Span::raw("Switch column focus (in Panes, 'l' cycles layout instead)"),
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
            Span::styled("  F5 / Ctrl+r           ", Style::default().fg(Color::Cyan)),
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
            Span::raw("Send command / keystrokes direct to background pane, then Enter"),
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
            Span::raw("Cycle layout preset (Panes column only: even-h, even-v, tiled, ...)"),
        ]),
        Line::from(vec![
            Span::styled("  s                     ", Style::default().fg(Color::Cyan)),
            Span::raw("Toggle synchronize-panes — while ON, ':' broadcasts to ALL panes"),
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
            Span::raw("On the selected card: [◀][▼][▲][▶] resize, [↕] swap,"),
        ]),
        Line::from(vec![
            Span::styled(
                "  [⬓] [◧] [x]            ",
                Style::default().fg(Color::Yellow),
            ),
            Span::raw("Click to split stacked / side-by-side, or close (confirms)"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl + x              ", Style::default().fg(Color::Red)),
            Span::raw("Respawn pane process — kills what is running (with confirm)"),
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

/// A centred rectangle of a fixed size, shrunk to fit `area` when it must.
///
/// Dialog content is a fixed number of rows — a prompt, an input box, a hint —
/// so sizing by percentage made these grow with the terminal until a one-line
/// text field sat in a 75-column box. Anything that needs to adapt does so by
/// wrapping its text, not by inflating the frame.
pub fn centered_fixed(width: u16, height: u16, area: Rect) -> Rect {
    let width = width.min(area.width.saturating_sub(2)).max(1);
    let height = height.min(area.height.saturating_sub(2)).max(1);
    Rect {
        x: area.x + (area.width.saturating_sub(width)) / 2,
        y: area.y + (area.height.saturating_sub(height)) / 2,
        width,
        height,
    }
}

/// Frame of the text-entry dialogs (new/rename/send). One prompt line, a
/// three-row input box, one hint line, plus margin and borders.
pub fn prompt_overlay(area: Rect) -> Rect {
    centered_fixed(58, 10, area)
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
