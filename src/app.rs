use crate::action::{Action, ToastLevel};
use crate::config::Config;
use crate::domain::{Pane, PaneId, Session, SessionId, Window, WindowId};
use crate::tmux::TmuxClient;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config as MatcherConfig, Matcher, Utf32Str};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusColumn {
    Sessions,
    Windows,
    Panes,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KillTarget {
    Session(SessionId, String),
    Window(WindowId, String),
    Pane(PaneId, String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Search {
        query: String,
        selected_index: usize,
    },
    InspectPane {
        pane_id: PaneId,
        scroll_offset: usize,
    },
    PromptNewSession {
        input: String,
    },
    PromptNewWindow {
        session_id: SessionId,
        input: String,
    },
    PromptNewPane {
        pane_id: PaneId,
    },
    PromptRenameSession {
        session_id: SessionId,
        input: String,
    },
    PromptRenameWindow {
        window_id: WindowId,
        input: String,
    },
    ConfirmKill(KillTarget),
    Help,
}

#[derive(Debug, Clone, Default)]
pub struct SelectionState {
    pub session_idx: usize,
    pub window_idx: usize,
    pub pane_idx: usize,
}

#[derive(Debug, Clone)]
pub struct Toast {
    pub message: String,
    pub level: ToastLevel,
    pub created_at: Instant,
    pub ttl: Duration,
}

#[derive(Debug, Clone)]
pub struct SearchItem {
    pub session_id: SessionId,
    pub session_name: String,
    pub window_id: WindowId,
    pub window_name: String,
    pub pane_id: PaneId,
    pub command: String,
    pub path: String,
    pub display_text: String,
}

pub struct App {
    pub config: Config,
    pub focus: FocusColumn,
    pub mode: Mode,
    pub selection: SelectionState,
    pub sessions: Vec<Session>,
    pub client: Box<dyn TmuxClient>,
    pub toasts: Vec<Toast>,
    pub should_quit: bool,
    pub pending_handoff: Option<(SessionId, String, WindowId, PaneId)>,
    pub is_mock: bool,
    pub last_area: ratatui::layout::Rect,
}

impl App {
    pub fn new(client: Box<dyn TmuxClient>, config: Config, is_mock: bool) -> Self {
        let mut app = Self {
            config,
            focus: FocusColumn::Sessions,
            mode: Mode::Normal,
            selection: SelectionState::default(),
            sessions: Vec::new(),
            client,
            toasts: Vec::new(),
            should_quit: false,
            pending_handoff: None,
            is_mock,
            last_area: ratatui::layout::Rect::default(),
        };
        let _ = app.refresh_data();
        app
    }

    pub fn refresh_data(&mut self) -> Result<()> {
        let mut tree = self.client.fetch_full_tree()?;

        // Preserve preview buffers for existing panes if available
        for session in &mut tree {
            for window in &mut session.windows {
                for pane in &mut window.panes {
                    if let Ok(raw) =
                        self.client
                            .capture_pane(&pane.id, self.config.pane_preview_lines, true)
                    {
                        pane.set_preview(raw);
                    }
                }
            }
        }

        self.sessions = tree;
        self.clamp_selections();
        Ok(())
    }

    pub fn clamp_selections(&mut self) {
        if self.sessions.is_empty() {
            self.selection.session_idx = 0;
            self.selection.window_idx = 0;
            self.selection.pane_idx = 0;
            return;
        }

        if self.selection.session_idx >= self.sessions.len() {
            self.selection.session_idx = self.sessions.len().saturating_sub(1);
        }

        let session = &self.sessions[self.selection.session_idx];
        if session.windows.is_empty() {
            self.selection.window_idx = 0;
            self.selection.pane_idx = 0;
            return;
        }

        if self.selection.window_idx >= session.windows.len() {
            self.selection.window_idx = session.windows.len().saturating_sub(1);
        }

        let window = &session.windows[self.selection.window_idx];
        if window.panes.is_empty() {
            self.selection.pane_idx = 0;
            return;
        }

        if self.selection.pane_idx >= window.panes.len() {
            self.selection.pane_idx = window.panes.len().saturating_sub(1);
        }
    }

    pub fn selected_session(&self) -> Option<&Session> {
        self.sessions.get(self.selection.session_idx)
    }

    pub fn selected_window(&self) -> Option<&Window> {
        self.selected_session()
            .and_then(|s| s.windows.get(self.selection.window_idx))
    }

    pub fn selected_pane(&self) -> Option<&Pane> {
        self.selected_window()
            .and_then(|w| w.panes.get(self.selection.pane_idx))
    }

    pub fn search_items(&self) -> Vec<SearchItem> {
        let mut items = Vec::new();
        for session in &self.sessions {
            for window in &session.windows {
                for pane in &window.panes {
                    let path_str = pane.current_path.to_string_lossy().to_string();
                    let display_text = format!(
                        "{} {} {} {} {}",
                        session.name, window.name, pane.id.0, pane.current_command, path_str
                    );
                    items.push(SearchItem {
                        session_id: session.id.clone(),
                        session_name: session.name.clone(),
                        window_id: window.id.clone(),
                        window_name: window.name.clone(),
                        pane_id: pane.id.clone(),
                        command: pane.current_command.clone(),
                        path: path_str,
                        display_text,
                    });
                }
            }
        }
        items
    }

    pub fn filtered_search_results(&self, query: &str) -> Vec<SearchItem> {
        let all_items = self.search_items();
        if query.trim().is_empty() {
            return all_items;
        }

        let mut matcher = Matcher::new(MatcherConfig::DEFAULT);
        let pattern = Pattern::parse(query, CaseMatching::Ignore, Normalization::Smart);

        let mut matched: Vec<(SearchItem, u32)> = all_items
            .into_iter()
            .filter_map(|item| {
                let mut buf = Vec::new();
                let haystack = Utf32Str::new(&item.display_text, &mut buf);
                pattern
                    .score(haystack, &mut matcher)
                    .map(|score| (item, score))
            })
            .collect();

        matched.sort_by_key(|b| std::cmp::Reverse(b.1));
        matched.into_iter().map(|(item, _)| item).collect()
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> Option<Action> {
        match &self.mode {
            Mode::Normal => match (key.modifiers, key.code) {
                (m, KeyCode::Char('q') | KeyCode::Char('Q')) if m.is_empty() => Some(Action::Quit),
                (KeyModifiers::NONE, KeyCode::Esc) => Some(Action::Quit),
                (m, KeyCode::Char('j') | KeyCode::Char('J') | KeyCode::Down) if m.is_empty() => {
                    Some(Action::NavigateDown)
                }
                (m, KeyCode::Char('k') | KeyCode::Char('K') | KeyCode::Up) if m.is_empty() => {
                    Some(Action::NavigateUp)
                }
                (m, KeyCode::Char('h') | KeyCode::Char('H') | KeyCode::Left) if m.is_empty() => {
                    Some(Action::NavigateLeft)
                }
                (m, KeyCode::Char('l') | KeyCode::Char('L') | KeyCode::Right) if m.is_empty() => {
                    Some(Action::NavigateRight)
                }
                (KeyModifiers::NONE, KeyCode::Tab) => Some(Action::NextColumn),
                (m, KeyCode::BackTab) if m.is_empty() || m == KeyModifiers::SHIFT => {
                    Some(Action::PrevColumn)
                }
                (KeyModifiers::SHIFT, KeyCode::Tab) => Some(Action::PrevColumn),
                (KeyModifiers::NONE, KeyCode::Enter) => Some(Action::OpenSelection),
                (m, KeyCode::Char('/')) if m.is_empty() => Some(Action::ToggleSearch),
                (m, KeyCode::Char('?')) if m.is_empty() || m == KeyModifiers::SHIFT => {
                    Some(Action::Help)
                }
                (KeyModifiers::CONTROL, KeyCode::Char('r') | KeyCode::Char('R'))
                | (KeyModifiers::NONE, KeyCode::F(5)) => Some(Action::Refresh),
                (KeyModifiers::NONE, KeyCode::Char(' ')) => Some(Action::ToggleInspect),
                (m, KeyCode::Char('z') | KeyCode::Char('Z')) if m.is_empty() => {
                    Some(Action::ToggleZoom)
                }
                (m, KeyCode::Char('f') | KeyCode::Char('F')) if m.is_empty() => {
                    Some(Action::ToggleFavorite)
                }
                (m, KeyCode::Char('c') | KeyCode::Char('C')) if m.is_empty() => {
                    Some(Action::CopyPaneOutput)
                }
                (m, KeyCode::Char('x') | KeyCode::Char('X')) if m.is_empty() => {
                    Some(Action::PromptKill)
                }
                (m, KeyCode::Char('n') | KeyCode::Char('N')) if m.is_empty() => match self.focus {
                    FocusColumn::Sessions => Some(Action::PromptNewSession),
                    FocusColumn::Windows => Some(Action::PromptNewWindow),
                    FocusColumn::Panes => Some(Action::PromptNewPane),
                },
                // Allow both 'r', 'R' (with or without Shift modifier), and F2 for Rename
                (m, KeyCode::Char('r') | KeyCode::Char('R'))
                    if m.is_empty() || m == KeyModifiers::SHIFT =>
                {
                    match self.focus {
                        FocusColumn::Sessions => Some(Action::PromptRenameSession),
                        FocusColumn::Windows | FocusColumn::Panes => {
                            Some(Action::PromptRenameWindow)
                        }
                    }
                }
                (KeyModifiers::NONE, KeyCode::F(2)) => match self.focus {
                    FocusColumn::Sessions => Some(Action::PromptRenameSession),
                    FocusColumn::Windows | FocusColumn::Panes => Some(Action::PromptRenameWindow),
                },
                _ => None,
            },

            Mode::PromptNewPane { .. } => match (key.modifiers, key.code) {
                (KeyModifiers::NONE, KeyCode::Esc)
                | (KeyModifiers::NONE, KeyCode::Char('q') | KeyCode::Char('Q')) => {
                    Some(Action::CancelModal)
                }
                (m, KeyCode::Char('v') | KeyCode::Char('V'))
                    if m.is_empty() || m == KeyModifiers::SHIFT =>
                {
                    Some(Action::SplitPane { vertical: true })
                }
                (
                    m,
                    KeyCode::Char('h')
                    | KeyCode::Char('H')
                    | KeyCode::Char('s')
                    | KeyCode::Char('S'),
                ) if m.is_empty() || m == KeyModifiers::SHIFT => {
                    Some(Action::SplitPane { vertical: false })
                }
                _ => None,
            },

            Mode::Search { .. } => match (key.modifiers, key.code) {
                (KeyModifiers::NONE, KeyCode::Esc) => Some(Action::ToggleSearch),
                (KeyModifiers::NONE, KeyCode::Enter) => Some(Action::SearchSelect),
                (KeyModifiers::NONE, KeyCode::Down)
                | (KeyModifiers::CONTROL, KeyCode::Char('n') | KeyCode::Char('j')) => {
                    Some(Action::SearchNext)
                }
                (KeyModifiers::NONE, KeyCode::Up)
                | (KeyModifiers::CONTROL, KeyCode::Char('p') | KeyCode::Char('k')) => {
                    Some(Action::SearchPrev)
                }
                (KeyModifiers::NONE, KeyCode::Backspace) => Some(Action::SearchBackspace),
                (m, KeyCode::Char(c)) if m.is_empty() || m == KeyModifiers::SHIFT => {
                    Some(Action::SearchInput(c))
                }
                _ => None,
            },

            Mode::InspectPane { .. } => match (key.modifiers, key.code) {
                (KeyModifiers::NONE, KeyCode::Esc)
                | (KeyModifiers::NONE, KeyCode::Char('q') | KeyCode::Char('Q'))
                | (KeyModifiers::NONE, KeyCode::Char('z') | KeyCode::Char('Z'))
                | (KeyModifiers::NONE, KeyCode::Char(' ')) => Some(Action::ToggleInspect),
                (KeyModifiers::NONE, KeyCode::Char('j') | KeyCode::Down) => {
                    Some(Action::InspectScrollDown(1))
                }
                (KeyModifiers::NONE, KeyCode::Char('k') | KeyCode::Up) => {
                    Some(Action::InspectScrollUp(1))
                }
                (KeyModifiers::CONTROL, KeyCode::Char('d')) => Some(Action::InspectScrollDown(10)),
                (KeyModifiers::CONTROL, KeyCode::Char('u')) => Some(Action::InspectScrollUp(10)),
                (KeyModifiers::NONE, KeyCode::Char('g')) => Some(Action::InspectScrollTop),
                (m, KeyCode::Char('G')) if m.is_empty() || m == KeyModifiers::SHIFT => {
                    Some(Action::InspectScrollBottom)
                }
                (KeyModifiers::NONE, KeyCode::Char('c') | KeyCode::Char('C')) => {
                    Some(Action::CopyPaneOutput)
                }
                (KeyModifiers::NONE, KeyCode::Enter) => Some(Action::OpenSelection),
                _ => None,
            },

            Mode::ConfirmKill(_) => match (key.modifiers, key.code) {
                (m, KeyCode::Char('y') | KeyCode::Char('Y'))
                    if m.is_empty() || m == KeyModifiers::SHIFT =>
                {
                    Some(Action::ConfirmKill)
                }
                (KeyModifiers::NONE, KeyCode::Enter) => Some(Action::ConfirmKill),
                (KeyModifiers::NONE, KeyCode::Esc)
                | (
                    KeyModifiers::NONE,
                    KeyCode::Char('n')
                    | KeyCode::Char('N')
                    | KeyCode::Char('q')
                    | KeyCode::Char('Q'),
                ) => Some(Action::CancelModal),
                _ => None,
            },

            Mode::PromptNewSession { .. }
            | Mode::PromptNewWindow { .. }
            | Mode::PromptRenameSession { .. }
            | Mode::PromptRenameWindow { .. } => match (key.modifiers, key.code) {
                (KeyModifiers::NONE, KeyCode::Esc) => Some(Action::CancelModal),
                (KeyModifiers::NONE, KeyCode::Enter) => Some(Action::ModalSubmit),
                (KeyModifiers::NONE, KeyCode::Backspace) => Some(Action::ModalBackspace),
                (m, KeyCode::Char(c)) if m.is_empty() || m == KeyModifiers::SHIFT => {
                    Some(Action::ModalInput(c))
                }
                _ => None,
            },

            Mode::Help => match (key.modifiers, key.code) {
                (KeyModifiers::NONE, KeyCode::Esc)
                | (KeyModifiers::NONE, KeyCode::Char('q') | KeyCode::Char('Q')) => {
                    Some(Action::Help)
                }
                (m, KeyCode::Char('?')) if m.is_empty() || m == KeyModifiers::SHIFT => {
                    Some(Action::Help)
                }
                _ => None,
            },
        }
    }

    pub fn handle_mouse_event(
        &mut self,
        mouse: crossterm::event::MouseEvent,
        area: ratatui::layout::Rect,
    ) -> Option<Action> {
        self.last_area = area;
        use crossterm::event::MouseEventKind;
        match mouse.kind {
            MouseEventKind::Down(crossterm::event::MouseButton::Left) => Some(Action::MouseClick {
                column: mouse.column,
                row: mouse.row,
                double_click: false,
            }),
            MouseEventKind::ScrollUp => Some(Action::MouseScrollUp {
                column: mouse.column,
                row: mouse.row,
            }),
            MouseEventKind::ScrollDown => Some(Action::MouseScrollDown {
                column: mouse.column,
                row: mouse.row,
            }),
            _ => None,
        }
    }

    pub fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Quit => {
                self.should_quit = true;
            }

            Action::Tick => {
                self.toasts.retain(|t| t.created_at.elapsed() < t.ttl);
            }

            Action::Refresh => {
                if let Err(e) = self.refresh_data() {
                    self.show_toast(format!("Refresh failed: {e}"), ToastLevel::Error);
                } else {
                    self.show_toast("Refreshed".to_string(), ToastLevel::Info);
                }
            }

            Action::DataRefreshed => {
                self.clamp_selections();
            }

            Action::NavigateDown => match self.focus {
                FocusColumn::Sessions => {
                    if !self.sessions.is_empty()
                        && self.selection.session_idx + 1 < self.sessions.len()
                    {
                        self.selection.session_idx += 1;
                        self.selection.window_idx = 0;
                        self.selection.pane_idx = 0;
                        self.clamp_selections();
                    }
                }
                FocusColumn::Windows => {
                    if let Some(session) = self.selected_session()
                        && !session.windows.is_empty()
                        && self.selection.window_idx + 1 < session.windows.len()
                    {
                        self.selection.window_idx += 1;
                        self.selection.pane_idx = 0;
                        self.clamp_selections();
                    }
                }
                FocusColumn::Panes => {
                    if let Some(window) = self.selected_window()
                        && !window.panes.is_empty()
                        && self.selection.pane_idx + 1 < window.panes.len()
                    {
                        self.selection.pane_idx += 1;
                    }
                }
            },

            Action::NavigateUp => match self.focus {
                FocusColumn::Sessions => {
                    if self.selection.session_idx > 0 {
                        self.selection.session_idx -= 1;
                        self.selection.window_idx = 0;
                        self.selection.pane_idx = 0;
                        self.clamp_selections();
                    }
                }
                FocusColumn::Windows => {
                    if self.selection.window_idx > 0 {
                        self.selection.window_idx -= 1;
                        self.selection.pane_idx = 0;
                        self.clamp_selections();
                    }
                }
                FocusColumn::Panes => {
                    if self.selection.pane_idx > 0 {
                        self.selection.pane_idx -= 1;
                    }
                }
            },

            Action::NavigateLeft => match self.focus {
                FocusColumn::Panes => self.focus = FocusColumn::Windows,
                FocusColumn::Windows => self.focus = FocusColumn::Sessions,
                FocusColumn::Sessions => {}
            },

            Action::NavigateRight => match self.focus {
                FocusColumn::Sessions => {
                    if self.selected_window().is_some() {
                        self.focus = FocusColumn::Windows;
                    }
                }
                FocusColumn::Windows => {
                    if self.selected_pane().is_some() {
                        self.focus = FocusColumn::Panes;
                    }
                }
                FocusColumn::Panes => {}
            },

            Action::NextColumn => match self.focus {
                FocusColumn::Sessions => self.focus = FocusColumn::Windows,
                FocusColumn::Windows => self.focus = FocusColumn::Panes,
                FocusColumn::Panes => self.focus = FocusColumn::Sessions,
            },

            Action::PrevColumn => match self.focus {
                FocusColumn::Sessions => self.focus = FocusColumn::Panes,
                FocusColumn::Windows => self.focus = FocusColumn::Sessions,
                FocusColumn::Panes => self.focus = FocusColumn::Windows,
            },

            Action::OpenSelection => {
                if let (Some(session), Some(window), Some(pane)) = (
                    self.selected_session(),
                    self.selected_window(),
                    self.selected_pane(),
                ) {
                    return Ok(Some(Action::Handoff {
                        session_id: session.id.clone(),
                        session_name: session.name.clone(),
                        window_id: window.id.clone(),
                        pane_id: pane.id.clone(),
                    }));
                }
            }

            Action::FocusSelection => {
                if let (Some(session), Some(window), Some(pane)) = (
                    self.selected_session(),
                    self.selected_window(),
                    self.selected_pane(),
                ) {
                    let _ = self.client.focus_pane(&session.id, &window.id, &pane.id);
                }
            }

            Action::Handoff {
                session_id,
                session_name,
                window_id,
                pane_id,
            } => {
                self.pending_handoff = Some((session_id, session_name, window_id, pane_id));
                self.should_quit = true;
            }

            Action::ToggleInspect => {
                if let Mode::InspectPane { .. } = self.mode {
                    self.mode = Mode::Normal;
                } else if let Some(pane) = self.selected_pane() {
                    // Fetch full deep scrollback buffer for inspect mode
                    let raw = self
                        .client
                        .capture_pane(&pane.id, 2000, true)
                        .unwrap_or_else(|_| pane.preview_raw.clone());
                    let pane_id = pane.id.clone();
                    if let Some(w) = self
                        .sessions
                        .get_mut(self.selection.session_idx)
                        .and_then(|s| s.windows.get_mut(self.selection.window_idx))
                        && let Some(p) = w.panes.get_mut(self.selection.pane_idx)
                    {
                        p.set_preview(raw);
                    }
                    self.mode = Mode::InspectPane {
                        pane_id,
                        scroll_offset: 0,
                    };
                }
            }

            Action::InspectScrollUp(lines) => {
                if let Mode::InspectPane { scroll_offset, .. } = &mut self.mode {
                    *scroll_offset = scroll_offset.saturating_sub(lines);
                }
            }

            Action::InspectScrollDown(lines) => {
                let (pane_id, max_lines) = if let Mode::InspectPane { pane_id, .. } = &self.mode {
                    let total = self
                        .selected_window()
                        .and_then(|w| w.get_pane(pane_id))
                        .map(|p| p.preview_lines.len())
                        .unwrap_or(0);
                    (Some(pane_id.clone()), total)
                } else {
                    (None, 0)
                };

                if pane_id.is_some()
                    && let Mode::InspectPane { scroll_offset, .. } = &mut self.mode
                    && *scroll_offset + lines < max_lines
                {
                    *scroll_offset += lines;
                }
            }

            Action::InspectScrollTop => {
                if let Mode::InspectPane { scroll_offset, .. } = &mut self.mode {
                    *scroll_offset = 0;
                }
            }

            Action::InspectScrollBottom => {
                let (pane_id, max_lines) = if let Mode::InspectPane { pane_id, .. } = &self.mode {
                    let total = self
                        .selected_window()
                        .and_then(|w| w.get_pane(pane_id))
                        .map(|p| p.preview_lines.len())
                        .unwrap_or(0);
                    (Some(pane_id.clone()), total)
                } else {
                    (None, 0)
                };

                if pane_id.is_some()
                    && let Mode::InspectPane { scroll_offset, .. } = &mut self.mode
                {
                    *scroll_offset = max_lines.saturating_sub(10);
                }
            }

            Action::CopyPaneOutput => {
                if let Some(pane) = self.selected_pane() {
                    let text = pane.preview_lines.join("\n");
                    match arboard::Clipboard::new() {
                        Ok(mut clipboard) => {
                            if clipboard.set_text(text).is_ok() {
                                self.show_toast(
                                    "Copied pane output to clipboard".to_string(),
                                    ToastLevel::Success,
                                );
                            } else {
                                self.show_toast(
                                    "Failed to copy to clipboard".to_string(),
                                    ToastLevel::Error,
                                );
                            }
                        }
                        Err(e) => {
                            self.show_toast(
                                format!("Clipboard unavailable: {e}"),
                                ToastLevel::Warning,
                            );
                        }
                    }
                }
            }

            Action::ToggleFavorite => {
                if let Some(s) = self.sessions.get_mut(self.selection.session_idx) {
                    s.is_favorite = !s.is_favorite;
                    let msg = if s.is_favorite {
                        "Added to favorites"
                    } else {
                        "Removed from favorites"
                    };
                    self.show_toast(msg.to_string(), ToastLevel::Info);
                }
            }

            Action::ToggleZoom => {
                return self.update(Action::ToggleInspect);
            }

            Action::ToggleSearch => {
                if let Mode::Search { .. } = self.mode {
                    self.mode = Mode::Normal;
                } else {
                    self.mode = Mode::Search {
                        query: String::new(),
                        selected_index: 0,
                    };
                }
            }

            Action::SearchInput(c) => {
                if let Mode::Search {
                    query,
                    selected_index,
                } = &mut self.mode
                {
                    query.push(c);
                    *selected_index = 0;
                }
            }

            Action::SearchBackspace => {
                if let Mode::Search {
                    query,
                    selected_index,
                } = &mut self.mode
                {
                    query.pop();
                    *selected_index = 0;
                }
            }

            Action::SearchNext => {
                let query = if let Mode::Search { query, .. } = &self.mode {
                    Some(query.clone())
                } else {
                    None
                };

                if let Some(q) = query {
                    let results_len = self.filtered_search_results(&q).len();
                    if let Mode::Search { selected_index, .. } = &mut self.mode
                        && results_len > 0
                        && *selected_index + 1 < results_len
                    {
                        *selected_index += 1;
                    }
                }
            }

            Action::SearchPrev => {
                if let Mode::Search { selected_index, .. } = &mut self.mode
                    && *selected_index > 0
                {
                    *selected_index -= 1;
                }
            }

            Action::SearchSelect => {
                if let Mode::Search {
                    query,
                    selected_index,
                } = &self.mode
                {
                    let results = self.filtered_search_results(query);
                    if let Some(item) = results.get(*selected_index) {
                        let (s_id, s_name, w_id, p_id) = (
                            item.session_id.clone(),
                            item.session_name.clone(),
                            item.window_id.clone(),
                            item.pane_id.clone(),
                        );
                        self.mode = Mode::Normal;
                        return Ok(Some(Action::Handoff {
                            session_id: s_id,
                            session_name: s_name,
                            window_id: w_id,
                            pane_id: p_id,
                        }));
                    }
                }
            }

            Action::PromptKill => match self.focus {
                FocusColumn::Sessions => {
                    if let Some(s) = self.selected_session() {
                        self.mode =
                            Mode::ConfirmKill(KillTarget::Session(s.id.clone(), s.name.clone()));
                    }
                }
                FocusColumn::Windows => {
                    if let Some(w) = self.selected_window() {
                        self.mode =
                            Mode::ConfirmKill(KillTarget::Window(w.id.clone(), w.name.clone()));
                    }
                }
                FocusColumn::Panes => {
                    if let Some(p) = self.selected_pane() {
                        self.mode = Mode::ConfirmKill(KillTarget::Pane(
                            p.id.clone(),
                            p.current_command.clone(),
                        ));
                    }
                }
            },

            Action::ConfirmKill => {
                if let Mode::ConfirmKill(target) = self.mode.clone() {
                    match target {
                        KillTarget::Session(s_id, name) => {
                            if let Err(e) = self.client.kill_session(&s_id) {
                                self.show_toast(
                                    format!("Kill session failed: {e}"),
                                    ToastLevel::Error,
                                );
                            } else {
                                self.show_toast(
                                    format!("Killed session '{name}'"),
                                    ToastLevel::Success,
                                );
                                let _ = self.refresh_data();
                            }
                        }
                        KillTarget::Window(w_id, name) => {
                            if let Err(e) = self.client.kill_window(&w_id) {
                                self.show_toast(
                                    format!("Kill window failed: {e}"),
                                    ToastLevel::Error,
                                );
                            } else {
                                self.show_toast(
                                    format!("Killed window '{name}'"),
                                    ToastLevel::Success,
                                );
                                let _ = self.refresh_data();
                            }
                        }
                        KillTarget::Pane(p_id, name) => {
                            if let Err(e) = self.client.kill_pane(&p_id) {
                                self.show_toast(
                                    format!("Kill pane failed: {e}"),
                                    ToastLevel::Error,
                                );
                            } else {
                                self.show_toast(
                                    format!("Killed pane {p_id} ({name})"),
                                    ToastLevel::Success,
                                );
                                let _ = self.refresh_data();
                            }
                        }
                    }
                    self.mode = Mode::Normal;
                }
            }

            Action::PromptNewSession => {
                self.mode = Mode::PromptNewSession {
                    input: String::new(),
                };
            }

            Action::PromptNewWindow => {
                if let Some(s) = self.selected_session() {
                    self.mode = Mode::PromptNewWindow {
                        session_id: s.id.clone(),
                        input: String::new(),
                    };
                }
            }

            Action::PromptNewPane => {
                if let Some(p) = self.selected_pane() {
                    self.mode = Mode::PromptNewPane {
                        pane_id: p.id.clone(),
                    };
                }
            }

            Action::SplitPane { vertical } => {
                if let Mode::PromptNewPane { pane_id } = self.mode.clone() {
                    match self.client.split_pane(&pane_id, vertical) {
                        Ok(new_id) => {
                            let split_type = if vertical { "vertical" } else { "horizontal" };
                            self.show_toast(
                                format!("Created {split_type} split pane {new_id}"),
                                ToastLevel::Success,
                            );
                            let _ = self.refresh_data();
                            if let Some(w) = self.selected_window()
                                && let Some(pos) = w.panes.iter().position(|p| p.id == new_id)
                            {
                                self.selection.pane_idx = pos;
                            }
                        }
                        Err(e) => {
                            self.show_toast(format!("Split pane failed: {e}"), ToastLevel::Error);
                        }
                    }
                    self.mode = Mode::Normal;
                }
            }

            Action::MouseScrollUp { .. } => {
                if let Mode::InspectPane { .. } = self.mode {
                    return self.update(Action::InspectScrollUp(3));
                }
                return self.update(Action::NavigateUp);
            }

            Action::MouseScrollDown { .. } => {
                if let Mode::InspectPane { .. } = self.mode {
                    return self.update(Action::InspectScrollDown(3));
                }
                return self.update(Action::NavigateDown);
            }

            Action::MouseClick {
                column,
                row,
                double_click,
            } => {
                let layout = crate::ui::layout::AppLayout::split(self.last_area);

                // Check if clicked in sessions column
                if column >= layout.sessions_col.x
                    && column < layout.sessions_col.x + layout.sessions_col.width
                    && row >= layout.sessions_col.y
                    && row < layout.sessions_col.y + layout.sessions_col.height
                {
                    self.focus = FocusColumn::Sessions;
                    if row > layout.sessions_col.y
                        && row < layout.sessions_col.y + layout.sessions_col.height - 1
                    {
                        let clicked_idx = (row - (layout.sessions_col.y + 1)) as usize;
                        if clicked_idx < self.sessions.len() {
                            self.selection.session_idx = clicked_idx;
                            self.selection.window_idx = 0;
                            self.selection.pane_idx = 0;
                            self.clamp_selections();
                            if double_click {
                                return self.update(Action::OpenSelection);
                            }
                        }
                    }
                }
                // Check if clicked in windows column
                else if column >= layout.windows_col.x
                    && column < layout.windows_col.x + layout.windows_col.width
                    && row >= layout.windows_col.y
                    && row < layout.windows_col.y + layout.windows_col.height
                {
                    self.focus = FocusColumn::Windows;
                    if row > layout.windows_col.y
                        && row < layout.windows_col.y + layout.windows_col.height - 1
                    {
                        let clicked_idx = (row - (layout.windows_col.y + 1)) as usize;
                        if let Some(session) = self.selected_session()
                            && clicked_idx < session.windows.len()
                        {
                            self.selection.window_idx = clicked_idx;
                            self.selection.pane_idx = 0;
                            self.clamp_selections();
                            if double_click {
                                return self.update(Action::OpenSelection);
                            }
                        }
                    }
                }
                // Check if clicked in panes column
                else if column >= layout.panes_col.x
                    && column < layout.panes_col.x + layout.panes_col.width
                    && row >= layout.panes_col.y
                    && row < layout.panes_col.y + layout.panes_col.height
                {
                    self.focus = FocusColumn::Panes;
                    if let Some(window) = self.selected_window() {
                        let inner_panes_area = ratatui::widgets::Block::default()
                            .borders(ratatui::widgets::Borders::ALL)
                            .inner(layout.panes_col);

                        let mut found_pane_id = None;
                        if let Some(root) = crate::domain::LayoutNode::parse(&window.layout_str) {
                            found_pane_id = root.find_pane_at(inner_panes_area, column, row);
                        }

                        if let Some(p_id) = found_pane_id {
                            if let Some(pos) = window.panes.iter().position(|p| p.id == p_id) {
                                self.selection.pane_idx = pos;
                            }
                        } else if !window.panes.is_empty() && row > inner_panes_area.y {
                            let pane_height =
                                inner_panes_area.height / window.panes.len().max(1) as u16;
                            if let Some(clicked_idx) = row
                                .saturating_sub(inner_panes_area.y)
                                .checked_div(pane_height)
                            {
                                let idx = clicked_idx as usize;
                                if idx < window.panes.len() {
                                    self.selection.pane_idx = idx;
                                }
                            }
                        }

                        if double_click {
                            return self.update(Action::OpenSelection);
                        }
                    }
                }
            }

            Action::PromptRenameSession => {
                if let Some(s) = self.selected_session() {
                    self.mode = Mode::PromptRenameSession {
                        session_id: s.id.clone(),
                        input: s.name.clone(),
                    };
                }
            }

            Action::PromptRenameWindow => {
                if let Some(w) = self.selected_window() {
                    self.mode = Mode::PromptRenameWindow {
                        window_id: w.id.clone(),
                        input: w.name.clone(),
                    };
                }
            }

            Action::CancelModal => {
                self.mode = Mode::Normal;
            }

            Action::ModalInput(c) => match &mut self.mode {
                Mode::PromptNewSession { input }
                | Mode::PromptNewWindow { input, .. }
                | Mode::PromptRenameSession { input, .. }
                | Mode::PromptRenameWindow { input, .. } => {
                    input.push(c);
                }
                _ => {}
            },

            Action::ModalBackspace => match &mut self.mode {
                Mode::PromptNewSession { input }
                | Mode::PromptNewWindow { input, .. }
                | Mode::PromptRenameSession { input, .. }
                | Mode::PromptRenameWindow { input, .. } => {
                    input.pop();
                }
                _ => {}
            },

            Action::ModalSubmit => {
                let mode = self.mode.clone();
                match mode {
                    Mode::PromptNewSession { input } => {
                        let name = input.trim();
                        if !name.is_empty() {
                            if let Err(e) = self.client.create_session(name) {
                                self.show_toast(
                                    format!("Create session failed: {e}"),
                                    ToastLevel::Error,
                                );
                            } else {
                                self.show_toast(
                                    format!("Created session '{name}'"),
                                    ToastLevel::Success,
                                );
                                let _ = self.refresh_data();
                            }
                        }
                    }
                    Mode::PromptNewWindow { session_id, input } => {
                        let name = input.trim();
                        if !name.is_empty() {
                            if let Err(e) = self.client.create_window(&session_id, name) {
                                self.show_toast(
                                    format!("Create window failed: {e}"),
                                    ToastLevel::Error,
                                );
                            } else {
                                self.show_toast(
                                    format!("Created window '{name}'"),
                                    ToastLevel::Success,
                                );
                                let _ = self.refresh_data();
                            }
                        }
                    }
                    Mode::PromptRenameSession { session_id, input } => {
                        let name = input.trim();
                        if !name.is_empty() {
                            if let Err(e) = self.client.rename_session(&session_id, name) {
                                self.show_toast(
                                    format!("Rename session failed: {e}"),
                                    ToastLevel::Error,
                                );
                            } else {
                                self.show_toast(
                                    format!("Renamed session to '{name}'"),
                                    ToastLevel::Success,
                                );
                                let _ = self.refresh_data();
                            }
                        }
                    }
                    Mode::PromptRenameWindow { window_id, input } => {
                        let name = input.trim();
                        if !name.is_empty() {
                            if let Err(e) = self.client.rename_window(&window_id, name) {
                                self.show_toast(
                                    format!("Rename window failed: {e}"),
                                    ToastLevel::Error,
                                );
                            } else {
                                self.show_toast(
                                    format!("Renamed window to '{name}'"),
                                    ToastLevel::Success,
                                );
                                let _ = self.refresh_data();
                            }
                        }
                    }
                    _ => {}
                }
                self.mode = Mode::Normal;
            }

            Action::Help => {
                if let Mode::Help = self.mode {
                    self.mode = Mode::Normal;
                } else {
                    self.mode = Mode::Help;
                }
            }

            Action::ShowToast { message, level } => {
                self.show_toast(message, level);
            }
        }

        Ok(None)
    }

    pub fn show_toast(&mut self, message: String, level: ToastLevel) {
        self.toasts.push(Toast {
            message,
            level,
            created_at: Instant::now(),
            ttl: Duration::from_millis(3000),
        });
    }
}
