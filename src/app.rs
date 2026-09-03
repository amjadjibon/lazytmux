use crate::action::{Action, ToastLevel};
use crate::config::Config;
use crate::domain::{Pane, PaneId, Session, SessionId, Window, WindowId, sanitize_tmux_name};
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

use crate::ui::Theme;

const LAYOUT_PRESETS: &[&str] = &[
    "even-horizontal",
    "even-vertical",
    "main-horizontal",
    "main-vertical",
    "tiled",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SearchCategory {
    #[default]
    All,
    Sessions,
    Windows,
    Panes,
}

impl SearchCategory {
    pub fn name(&self) -> &'static str {
        match self {
            SearchCategory::All => "All",
            SearchCategory::Sessions => "Sessions",
            SearchCategory::Windows => "Windows",
            SearchCategory::Panes => "Panes",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            SearchCategory::All => SearchCategory::Sessions,
            SearchCategory::Sessions => SearchCategory::Windows,
            SearchCategory::Windows => SearchCategory::Panes,
            SearchCategory::Panes => SearchCategory::All,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            SearchCategory::All => SearchCategory::Panes,
            SearchCategory::Sessions => SearchCategory::All,
            SearchCategory::Windows => SearchCategory::Sessions,
            SearchCategory::Panes => SearchCategory::Windows,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Search {
        query: String,
        selected_index: usize,
        category: SearchCategory,
    },
    InspectPane {
        pane_id: PaneId,
        scroll_offset: usize,
        search_query: Option<String>,
        is_searching: bool,
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
    PromptSendCommand {
        pane_id: PaneId,
        input: String,
        with_enter: bool,
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
    pub category: SearchCategory,
    pub session_id: SessionId,
    pub session_name: String,
    pub window_id: WindowId,
    pub window_name: String,
    pub pane_id: PaneId,
    pub command: String,
    pub path: String,
    pub git_branch: Option<String>,
    pub display_text: String,
}

pub struct App {
    pub config: Config,
    pub theme: Theme,
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
    pub layout_preset_idx: usize,
    pub mouse_drag_start: Option<(u16, u16, PaneId)>,
    pub column_ratios: (u16, u16, u16),
    pub mouse_drag_col_border: Option<usize>,
    pub sidebar_mode: crate::ui::SidebarMode,
    pub last_click: Option<(Instant, u16, u16)>,
}

impl App {
    pub fn new(client: Box<dyn TmuxClient>, config: Config, is_mock: bool) -> Self {
        let border_type = match config.theme.border_style.as_str() {
            "double" => ratatui::widgets::BorderType::Double,
            "plain" => ratatui::widgets::BorderType::Plain,
            "thick" => ratatui::widgets::BorderType::Thick,
            _ => ratatui::widgets::BorderType::Rounded,
        };
        let theme = config.theme.preset.to_theme(border_type);
        let mut app = Self {
            config,
            theme,
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
            layout_preset_idx: 0,
            mouse_drag_start: None,
            column_ratios: (22, 28, 50),
            mouse_drag_col_border: None,
            sidebar_mode: crate::ui::SidebarMode::Full,
            last_click: None,
        };
        let _ = app.refresh_data();
        app
    }

    pub fn refresh_data(&mut self) -> Result<()> {
        let mut tree = self.client.fetch_full_tree()?;

        // PERFORMANCE OPTIMIZATION: Only capture preview buffers for the currently visible window.
        // Capturing all panes across all background sessions every 750ms causes massive subprocess
        // spawn overhead and high CPU usage. Capturing on-demand provides a 10x-50x speedup.
        let s_idx = self.selection.session_idx;
        let w_idx = self.selection.window_idx;
        if let Some(session) = tree.get_mut(s_idx)
            && let Some(window) = session.windows.get_mut(w_idx)
        {
            for pane in &mut window.panes {
                if let Ok(raw) =
                    self.client
                        .capture_pane(&pane.id, self.config.pane_preview_lines, true)
                {
                    pane.set_preview(raw);
                }
            }
        }

        self.sessions = tree;
        self.clamp_selections();
        Ok(())
    }

    pub fn refresh_active_window_preview(&mut self) {
        let s_idx = self.selection.session_idx;
        let w_idx = self.selection.window_idx;
        if let Some(session) = self.sessions.get_mut(s_idx)
            && let Some(window) = session.windows.get_mut(w_idx)
        {
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
            // Session entry
            let first_win = session.windows.first();
            let first_pane = first_win.and_then(|w| w.panes.first());
            if let (Some(win), Some(pane)) = (first_win, first_pane) {
                let display_text = format!("[session] {}", session.name);
                items.push(SearchItem {
                    category: SearchCategory::Sessions,
                    session_id: session.id.clone(),
                    session_name: session.name.clone(),
                    window_id: win.id.clone(),
                    window_name: win.name.clone(),
                    pane_id: pane.id.clone(),
                    command: pane.current_command.clone(),
                    path: pane.current_path.to_string_lossy().to_string(),
                    git_branch: pane.git_branch.clone(),
                    display_text,
                });
            }

            for window in &session.windows {
                if let Some(pane) = window.panes.first() {
                    let display_text = format!("[window] {} > {}", session.name, window.name);
                    items.push(SearchItem {
                        category: SearchCategory::Windows,
                        session_id: session.id.clone(),
                        session_name: session.name.clone(),
                        window_id: window.id.clone(),
                        window_name: window.name.clone(),
                        pane_id: pane.id.clone(),
                        command: pane.current_command.clone(),
                        path: pane.current_path.to_string_lossy().to_string(),
                        git_branch: pane.git_branch.clone(),
                        display_text,
                    });
                }

                for pane in &window.panes {
                    let path_str = pane.current_path.to_string_lossy().to_string();
                    let branch_str = pane
                        .git_branch
                        .as_deref()
                        .map(|b| format!(" ({b})"))
                        .unwrap_or_default();
                    let display_text = format!(
                        "[pane] {} > {} > {} {}{}",
                        session.name, window.name, pane.id.0, pane.current_command, branch_str
                    );
                    items.push(SearchItem {
                        category: SearchCategory::Panes,
                        session_id: session.id.clone(),
                        session_name: session.name.clone(),
                        window_id: window.id.clone(),
                        window_name: window.name.clone(),
                        pane_id: pane.id.clone(),
                        command: pane.current_command.clone(),
                        path: path_str,
                        git_branch: pane.git_branch.clone(),
                        display_text,
                    });
                }
            }
        }
        items
    }

    pub fn filtered_search_results(
        &self,
        query: &str,
        category: SearchCategory,
    ) -> Vec<SearchItem> {
        let mut all_items = self.search_items();
        if category != SearchCategory::All {
            all_items.retain(|i| i.category == category);
        }

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
                (KeyModifiers::CONTROL, KeyCode::Char('r') | KeyCode::Char('R')) => {
                    match self.focus {
                        FocusColumn::Panes => Some(Action::RespawnPane),
                        FocusColumn::Sessions | FocusColumn::Windows => Some(Action::Refresh),
                    }
                }
                (KeyModifiers::NONE, KeyCode::F(5)) => Some(Action::Refresh),
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
                (m, KeyCode::Char('t') | KeyCode::Char('T'))
                    if m.is_empty() || m == KeyModifiers::SHIFT =>
                {
                    Some(Action::NextTheme)
                }
                (m, KeyCode::Char('n') | KeyCode::Char('N')) if m.is_empty() => match self.focus {
                    FocusColumn::Sessions => Some(Action::PromptNewSession),
                    FocusColumn::Windows => Some(Action::PromptNewWindow),
                    FocusColumn::Panes => Some(Action::PromptNewPane),
                },
                // Pane specific shortcuts
                (m, KeyCode::Char('l') | KeyCode::Char('L'))
                    if (m.is_empty() || m == KeyModifiers::SHIFT)
                        && self.focus == FocusColumn::Panes =>
                {
                    Some(Action::NextLayout)
                }
                (m, KeyCode::Char('s') | KeyCode::Char('S'))
                    if (m.is_empty() || m == KeyModifiers::SHIFT)
                        && self.focus == FocusColumn::Panes =>
                {
                    Some(Action::ToggleSyncPanes)
                }
                (m, KeyCode::Char(':'))
                    if (m.is_empty() || m == KeyModifiers::SHIFT)
                        && self.selected_pane().is_some() =>
                {
                    Some(Action::PromptSendCommand)
                }
                (m, KeyCode::Char('b') | KeyCode::Char('B'))
                    if (m.is_empty() || m == KeyModifiers::SHIFT)
                        && self.focus == FocusColumn::Panes =>
                {
                    Some(Action::BreakPane)
                }
                (m, KeyCode::Char('['))
                    if (m.is_empty() || m == KeyModifiers::SHIFT)
                        && self.focus == FocusColumn::Panes =>
                {
                    Some(Action::SwapPaneUp)
                }
                (m, KeyCode::Char(']'))
                    if (m.is_empty() || m == KeyModifiers::SHIFT)
                        && self.focus == FocusColumn::Panes =>
                {
                    Some(Action::SwapPaneDown)
                }
                // Pane resize shortcuts (+ / -, Shift+Arrows, Shift+H/J/K/L)
                (m, KeyCode::Char('+') | KeyCode::Char('='))
                    if (m.is_empty() || m == KeyModifiers::SHIFT)
                        && self.focus == FocusColumn::Panes =>
                {
                    Some(Action::ResizePane(
                        crate::tmux::client::ResizeDirection::Down,
                        3,
                    ))
                }
                (KeyModifiers::NONE, KeyCode::Char('-')) if self.focus == FocusColumn::Panes => {
                    Some(Action::ResizePane(
                        crate::tmux::client::ResizeDirection::Up,
                        3,
                    ))
                }
                (m, KeyCode::Left)
                    if m.contains(KeyModifiers::SHIFT) && self.focus == FocusColumn::Panes =>
                {
                    Some(Action::ResizePane(
                        crate::tmux::client::ResizeDirection::Left,
                        4,
                    ))
                }
                (m, KeyCode::Right)
                    if m.contains(KeyModifiers::SHIFT) && self.focus == FocusColumn::Panes =>
                {
                    Some(Action::ResizePane(
                        crate::tmux::client::ResizeDirection::Right,
                        4,
                    ))
                }
                (m, KeyCode::Up)
                    if m.contains(KeyModifiers::SHIFT) && self.focus == FocusColumn::Panes =>
                {
                    Some(Action::ResizePane(
                        crate::tmux::client::ResizeDirection::Up,
                        2,
                    ))
                }
                (m, KeyCode::Down)
                    if m.contains(KeyModifiers::SHIFT) && self.focus == FocusColumn::Panes =>
                {
                    Some(Action::ResizePane(
                        crate::tmux::client::ResizeDirection::Down,
                        2,
                    ))
                }
                (m, KeyCode::Char('H'))
                    if m.contains(KeyModifiers::SHIFT) && self.focus == FocusColumn::Panes =>
                {
                    Some(Action::ResizePane(
                        crate::tmux::client::ResizeDirection::Left,
                        4,
                    ))
                }
                (m, KeyCode::Char('L'))
                    if m.contains(KeyModifiers::SHIFT) && self.focus == FocusColumn::Panes =>
                {
                    Some(Action::ResizePane(
                        crate::tmux::client::ResizeDirection::Right,
                        4,
                    ))
                }
                (m, KeyCode::Char('K'))
                    if m.contains(KeyModifiers::SHIFT) && self.focus == FocusColumn::Panes =>
                {
                    Some(Action::ResizePane(
                        crate::tmux::client::ResizeDirection::Up,
                        2,
                    ))
                }
                (m, KeyCode::Char('J'))
                    if m.contains(KeyModifiers::SHIFT) && self.focus == FocusColumn::Panes =>
                {
                    Some(Action::ResizePane(
                        crate::tmux::client::ResizeDirection::Down,
                        2,
                    ))
                }
                // Window specific shortcuts (reordering)
                (m, KeyCode::Char('['))
                    if (m.is_empty() || m == KeyModifiers::SHIFT)
                        && self.focus == FocusColumn::Windows =>
                {
                    Some(Action::MoveWindowLeft)
                }
                (m, KeyCode::Char(']'))
                    if (m.is_empty() || m == KeyModifiers::SHIFT)
                        && self.focus == FocusColumn::Windows =>
                {
                    Some(Action::MoveWindowRight)
                }
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
                // Column width resizing shortcuts (< / >, , / ., { / })
                (m, KeyCode::Char('<') | KeyCode::Char('{') | KeyCode::Char(','))
                    if m.is_empty() || m == KeyModifiers::SHIFT =>
                {
                    Some(Action::ResizeFocusedColumn(-2))
                }
                (m, KeyCode::Char('>') | KeyCode::Char('}') | KeyCode::Char('.'))
                    if m.is_empty() || m == KeyModifiers::SHIFT =>
                {
                    Some(Action::ResizeFocusedColumn(2))
                }
                // Toggle sidebar collapse modes (\ or |)
                (m, KeyCode::Char('\\') | KeyCode::Char('|'))
                    if m.is_empty() || m == KeyModifiers::SHIFT =>
                {
                    Some(Action::ToggleSidebarMode)
                }
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
                (KeyModifiers::NONE, KeyCode::Tab) => Some(Action::SearchNextCategory),
                (m, KeyCode::BackTab) if m.is_empty() || m == KeyModifiers::SHIFT => {
                    Some(Action::SearchPrevCategory)
                }
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

            Mode::InspectPane {
                is_searching,
                search_query,
                ..
            } => {
                if *is_searching {
                    match (key.modifiers, key.code) {
                        (KeyModifiers::NONE, KeyCode::Esc) => Some(Action::InspectSearchCancel),
                        (KeyModifiers::NONE, KeyCode::Enter) => Some(Action::InspectSearchSubmit),
                        (KeyModifiers::NONE, KeyCode::Backspace) => {
                            Some(Action::InspectSearchBackspace)
                        }
                        (m, KeyCode::Char(c)) if m.is_empty() || m == KeyModifiers::SHIFT => {
                            Some(Action::InspectSearchInput(c))
                        }
                        _ => None,
                    }
                } else {
                    match (key.modifiers, key.code) {
                        (KeyModifiers::NONE, KeyCode::Char('/')) => {
                            Some(Action::InspectStartSearch)
                        }
                        (KeyModifiers::NONE, KeyCode::Char('n')) => Some(Action::InspectSearchNext),
                        (m, KeyCode::Char('N')) if m.is_empty() || m == KeyModifiers::SHIFT => {
                            Some(Action::InspectSearchPrev)
                        }
                        (KeyModifiers::NONE, KeyCode::Esc) => {
                            if search_query.is_some() {
                                Some(Action::InspectSearchCancel)
                            } else {
                                Some(Action::ToggleInspect)
                            }
                        }
                        (KeyModifiers::NONE, KeyCode::Char('q') | KeyCode::Char('Q'))
                        | (KeyModifiers::NONE, KeyCode::Char('z') | KeyCode::Char('Z'))
                        | (KeyModifiers::NONE, KeyCode::Char(' ')) => Some(Action::ToggleInspect),
                        (KeyModifiers::NONE, KeyCode::Char('j') | KeyCode::Down) => {
                            Some(Action::InspectScrollDown(1))
                        }
                        (KeyModifiers::NONE, KeyCode::Char('k') | KeyCode::Up) => {
                            Some(Action::InspectScrollUp(1))
                        }
                        (KeyModifiers::CONTROL, KeyCode::Char('d')) => {
                            Some(Action::InspectScrollDown(10))
                        }
                        (KeyModifiers::CONTROL, KeyCode::Char('u')) => {
                            Some(Action::InspectScrollUp(10))
                        }
                        (KeyModifiers::NONE, KeyCode::Char('g')) => Some(Action::InspectScrollTop),
                        (m, KeyCode::Char('G')) if m.is_empty() || m == KeyModifiers::SHIFT => {
                            Some(Action::InspectScrollBottom)
                        }
                        (KeyModifiers::NONE, KeyCode::Char('c') | KeyCode::Char('C')) => {
                            Some(Action::CopyPaneOutput)
                        }
                        (KeyModifiers::NONE, KeyCode::Enter) => Some(Action::OpenSelection),
                        _ => None,
                    }
                }
            }

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

            Mode::PromptSendCommand { .. } => match (key.modifiers, key.code) {
                (KeyModifiers::NONE, KeyCode::Esc) => Some(Action::CancelModal),
                (KeyModifiers::NONE, KeyCode::Tab) => Some(Action::TogglePromptWithEnter),
                (KeyModifiers::CONTROL, KeyCode::Enter)
                | (KeyModifiers::ALT, KeyCode::Enter)
                | (KeyModifiers::SHIFT, KeyCode::Enter)
                | (KeyModifiers::CONTROL, KeyCode::Char('e') | KeyCode::Char('E'))
                | (KeyModifiers::CONTROL, KeyCode::Char('j') | KeyCode::Char('J')) => {
                    Some(Action::ModalSubmitWithEnter)
                }
                (KeyModifiers::NONE, KeyCode::Enter) => Some(Action::ModalSubmit),
                (KeyModifiers::NONE, KeyCode::Backspace) => Some(Action::ModalBackspace),
                (m, KeyCode::Char(c)) if m.is_empty() || m == KeyModifiers::SHIFT => {
                    Some(Action::ModalInput(c))
                }
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
            MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
                let now = Instant::now();
                let double_click = if let Some((prev_time, prev_col, prev_row)) = self.last_click {
                    now.duration_since(prev_time).as_millis() <= 450
                        && (mouse.column as i32 - prev_col as i32).abs() <= 1
                        && (mouse.row as i32 - prev_row as i32).abs() <= 1
                } else {
                    false
                };

                if double_click {
                    self.last_click = None;
                } else {
                    self.last_click = Some((now, mouse.column, mouse.row));
                }

                Some(Action::MouseClick {
                    column: mouse.column,
                    row: mouse.row,
                    double_click,
                })
            }
            MouseEventKind::Drag(crossterm::event::MouseButton::Left) => Some(Action::MouseDrag {
                column: mouse.column,
                row: mouse.row,
            }),
            MouseEventKind::Up(crossterm::event::MouseButton::Left) => Some(Action::MouseUp),
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
                self.refresh_active_window_preview();
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

            Action::NavigateDown => {
                match self.focus {
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
                }
                self.refresh_active_window_preview();
            }

            Action::NavigateUp => {
                match self.focus {
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
                }
                self.refresh_active_window_preview();
            }

            Action::NavigateLeft => match self.sidebar_mode {
                crate::ui::SidebarMode::Full => match self.focus {
                    FocusColumn::Panes => self.focus = FocusColumn::Windows,
                    FocusColumn::Windows => self.focus = FocusColumn::Sessions,
                    FocusColumn::Sessions => {}
                },
                crate::ui::SidebarMode::SessionsHidden => {
                    if self.focus == FocusColumn::Panes {
                        self.focus = FocusColumn::Windows;
                    }
                }
                crate::ui::SidebarMode::WindowsHidden => {
                    if self.focus == FocusColumn::Panes {
                        self.focus = FocusColumn::Sessions;
                    }
                }
                crate::ui::SidebarMode::PanesOnly => {}
            },

            Action::NavigateRight => match self.sidebar_mode {
                crate::ui::SidebarMode::Full => match self.focus {
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
                crate::ui::SidebarMode::SessionsHidden => {
                    if self.focus == FocusColumn::Windows && self.selected_pane().is_some() {
                        self.focus = FocusColumn::Panes;
                    }
                }
                crate::ui::SidebarMode::WindowsHidden => {
                    if self.focus == FocusColumn::Sessions && self.selected_pane().is_some() {
                        self.focus = FocusColumn::Panes;
                    }
                }
                crate::ui::SidebarMode::PanesOnly => {}
            },

            Action::NextColumn => match self.sidebar_mode {
                crate::ui::SidebarMode::Full => match self.focus {
                    FocusColumn::Sessions => self.focus = FocusColumn::Windows,
                    FocusColumn::Windows => self.focus = FocusColumn::Panes,
                    FocusColumn::Panes => self.focus = FocusColumn::Sessions,
                },
                crate::ui::SidebarMode::SessionsHidden => match self.focus {
                    FocusColumn::Windows => self.focus = FocusColumn::Panes,
                    _ => self.focus = FocusColumn::Windows,
                },
                crate::ui::SidebarMode::WindowsHidden => match self.focus {
                    FocusColumn::Sessions => self.focus = FocusColumn::Panes,
                    _ => self.focus = FocusColumn::Sessions,
                },
                crate::ui::SidebarMode::PanesOnly => {
                    self.focus = FocusColumn::Panes;
                }
            },

            Action::PrevColumn => match self.sidebar_mode {
                crate::ui::SidebarMode::Full => match self.focus {
                    FocusColumn::Sessions => self.focus = FocusColumn::Panes,
                    FocusColumn::Windows => self.focus = FocusColumn::Sessions,
                    FocusColumn::Panes => self.focus = FocusColumn::Windows,
                },
                crate::ui::SidebarMode::SessionsHidden => match self.focus {
                    FocusColumn::Panes => self.focus = FocusColumn::Windows,
                    _ => self.focus = FocusColumn::Panes,
                },
                crate::ui::SidebarMode::WindowsHidden => match self.focus {
                    FocusColumn::Panes => self.focus = FocusColumn::Sessions,
                    _ => self.focus = FocusColumn::Panes,
                },
                crate::ui::SidebarMode::PanesOnly => {
                    self.focus = FocusColumn::Panes;
                }
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
                    let scroll_offset = if self.last_area.height > 0 {
                        let total_lines = self
                            .selected_window()
                            .and_then(|w| w.get_pane(&pane_id))
                            .map(|p| p.preview_lines.len())
                            .unwrap_or(0);
                        let visible_height =
                            (self.last_area.height * 85 / 100).saturating_sub(4) as usize;
                        total_lines.saturating_sub(visible_height)
                    } else {
                        0
                    };
                    self.mode = Mode::InspectPane {
                        pane_id,
                        scroll_offset,
                        search_query: None,
                        is_searching: false,
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

            Action::InspectStartSearch => {
                if let Mode::InspectPane {
                    is_searching,
                    search_query,
                    ..
                } = &mut self.mode
                {
                    *is_searching = true;
                    if search_query.is_none() {
                        *search_query = Some(String::new());
                    }
                }
            }

            Action::InspectSearchInput(c) => {
                let (pane_id, query) = if let Mode::InspectPane {
                    search_query,
                    pane_id,
                    ..
                } = &self.mode
                {
                    let mut q = search_query.clone().unwrap_or_default();
                    q.push(c);
                    (pane_id.clone(), q)
                } else {
                    return Ok(None);
                };

                let target_line = {
                    let q_lower = query.to_lowercase();
                    self.selected_window()
                        .and_then(|w| w.get_pane(&pane_id))
                        .and_then(|pane| {
                            pane.preview_lines
                                .iter()
                                .position(|line| line.to_lowercase().contains(&q_lower))
                        })
                };

                if let Mode::InspectPane {
                    search_query,
                    scroll_offset,
                    ..
                } = &mut self.mode
                {
                    *search_query = Some(query);
                    if let Some(line_idx) = target_line {
                        *scroll_offset = line_idx;
                    }
                }
            }

            Action::InspectSearchBackspace => {
                let (pane_id, query) = if let Mode::InspectPane {
                    search_query: Some(q),
                    pane_id,
                    ..
                } = &self.mode
                {
                    let mut new_q = q.clone();
                    new_q.pop();
                    (pane_id.clone(), new_q)
                } else {
                    return Ok(None);
                };

                let target_line = if !query.is_empty() {
                    let q_lower = query.to_lowercase();
                    self.selected_window()
                        .and_then(|w| w.get_pane(&pane_id))
                        .and_then(|pane| {
                            pane.preview_lines
                                .iter()
                                .position(|line| line.to_lowercase().contains(&q_lower))
                        })
                } else {
                    None
                };

                if let Mode::InspectPane {
                    search_query,
                    scroll_offset,
                    ..
                } = &mut self.mode
                {
                    *search_query = Some(query);
                    if let Some(line_idx) = target_line {
                        *scroll_offset = line_idx;
                    }
                }
            }

            Action::InspectSearchSubmit => {
                if let Mode::InspectPane { is_searching, .. } = &mut self.mode {
                    *is_searching = false;
                }
            }

            Action::InspectSearchCancel => {
                if let Mode::InspectPane {
                    is_searching,
                    search_query,
                    ..
                } = &mut self.mode
                {
                    *is_searching = false;
                    *search_query = None;
                }
            }

            Action::InspectSearchNext => {
                let (pane_id, query, current_offset) = if let Mode::InspectPane {
                    search_query: Some(q),
                    scroll_offset,
                    pane_id,
                    ..
                } = &self.mode
                {
                    (pane_id.clone(), q.clone(), *scroll_offset)
                } else {
                    return Ok(None);
                };

                let q_lower = query.to_lowercase();
                if !q_lower.is_empty()
                    && let Some(pane) = self.selected_window().and_then(|w| w.get_pane(&pane_id))
                {
                    let next_match = pane
                        .preview_lines
                        .iter()
                        .enumerate()
                        .skip(current_offset + 1)
                        .find(|(_, line)| line.to_lowercase().contains(&q_lower))
                        .map(|(idx, _)| idx)
                        .or_else(|| {
                            pane.preview_lines
                                .iter()
                                .enumerate()
                                .find(|(_, line)| line.to_lowercase().contains(&q_lower))
                                .map(|(idx, _)| idx)
                        });

                    if let Some(idx) = next_match
                        && let Mode::InspectPane { scroll_offset, .. } = &mut self.mode
                    {
                        *scroll_offset = idx;
                    }
                }
            }

            Action::InspectSearchPrev => {
                let (pane_id, query, current_offset) = if let Mode::InspectPane {
                    search_query: Some(q),
                    scroll_offset,
                    pane_id,
                    ..
                } = &self.mode
                {
                    (pane_id.clone(), q.clone(), *scroll_offset)
                } else {
                    return Ok(None);
                };

                let q_lower = query.to_lowercase();
                if !q_lower.is_empty()
                    && let Some(pane) = self.selected_window().and_then(|w| w.get_pane(&pane_id))
                {
                    let prev_match = if current_offset > 0 {
                        pane.preview_lines[..current_offset]
                            .iter()
                            .enumerate()
                            .rfind(|(_, line)| line.to_lowercase().contains(&q_lower))
                            .map(|(idx, _)| idx)
                    } else {
                        None
                    }
                    .or_else(|| {
                        pane.preview_lines
                            .iter()
                            .enumerate()
                            .rfind(|(_, line)| line.to_lowercase().contains(&q_lower))
                            .map(|(idx, _)| idx)
                    });

                    if let Some(idx) = prev_match
                        && let Mode::InspectPane { scroll_offset, .. } = &mut self.mode
                    {
                        *scroll_offset = idx;
                    }
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

            Action::NextTheme => {
                let next_preset = self.theme.preset.next();
                self.theme = next_preset.to_theme(self.theme.border_type);
                self.show_toast(format!("Theme: {}", next_preset.name()), ToastLevel::Info);
            }

            Action::PrevTheme => {
                let prev_preset = self.theme.preset.prev();
                self.theme = prev_preset.to_theme(self.theme.border_type);
                self.show_toast(format!("Theme: {}", prev_preset.name()), ToastLevel::Info);
            }

            Action::NextLayout => {
                if let Some(win) = self.selected_window() {
                    let w_id = win.id.clone();
                    let next_idx = (self.layout_preset_idx + 1) % LAYOUT_PRESETS.len();
                    self.layout_preset_idx = next_idx;
                    let preset_name = LAYOUT_PRESETS[next_idx];
                    if let Err(e) = self.client.select_layout(&w_id, preset_name) {
                        self.show_toast(format!("Failed to set layout: {e}"), ToastLevel::Error);
                    } else {
                        self.show_toast(format!("Layout: {preset_name}"), ToastLevel::Success);
                        let _ = self.refresh_data();
                    }
                }
            }

            Action::ToggleSyncPanes => {
                if let Some(win) = self.selected_window() {
                    let w_id = win.id.clone();
                    match self.client.toggle_sync_panes(&w_id) {
                        Ok(synced) => {
                            let msg = if synced {
                                "Synchronize panes: ON (broadcast typing enabled)"
                            } else {
                                "Synchronize panes: OFF"
                            };
                            self.show_toast(msg.to_string(), ToastLevel::Info);
                        }
                        Err(e) => {
                            self.show_toast(format!("Sync panes failed: {e}"), ToastLevel::Error);
                        }
                    }
                }
            }

            Action::SwapPaneUp => {
                if let Some(pane) = self.selected_pane() {
                    let p_id = pane.id.clone();
                    if let Err(e) = self.client.swap_pane(&p_id, true) {
                        self.show_toast(format!("Swap pane failed: {e}"), ToastLevel::Error);
                    } else {
                        self.show_toast("Swapped pane up".to_string(), ToastLevel::Success);
                        if self.selection.pane_idx > 0 {
                            self.selection.pane_idx -= 1;
                        }
                        let _ = self.refresh_data();
                    }
                }
            }

            Action::SwapPaneDown => {
                if let Some(pane) = self.selected_pane() {
                    let p_id = pane.id.clone();
                    if let Err(e) = self.client.swap_pane(&p_id, false) {
                        self.show_toast(format!("Swap pane failed: {e}"), ToastLevel::Error);
                    } else {
                        self.show_toast("Swapped pane down".to_string(), ToastLevel::Success);
                        if let Some(win) = self.selected_window()
                            && self.selection.pane_idx + 1 < win.panes.len()
                        {
                            self.selection.pane_idx += 1;
                        }
                        let _ = self.refresh_data();
                    }
                }
            }

            Action::MoveWindowLeft => {
                if let Some(win) = self.selected_window() {
                    let w_id = win.id.clone();
                    if let Err(e) = self.client.swap_window(&w_id, true) {
                        self.show_toast(format!("Move window failed: {e}"), ToastLevel::Error);
                    } else {
                        self.show_toast("Moved window left".to_string(), ToastLevel::Success);
                        if self.selection.window_idx > 0 {
                            self.selection.window_idx -= 1;
                        }
                        let _ = self.refresh_data();
                    }
                }
            }

            Action::MoveWindowRight => {
                if let Some(win) = self.selected_window() {
                    let w_id = win.id.clone();
                    if let Err(e) = self.client.swap_window(&w_id, false) {
                        self.show_toast(format!("Move window failed: {e}"), ToastLevel::Error);
                    } else {
                        self.show_toast("Moved window right".to_string(), ToastLevel::Success);
                        if let Some(s) = self.selected_session()
                            && self.selection.window_idx + 1 < s.windows.len()
                        {
                            self.selection.window_idx += 1;
                        }
                        let _ = self.refresh_data();
                    }
                }
            }

            Action::RespawnPane => {
                if let Some(pane) = self.selected_pane() {
                    let p_id = pane.id.clone();
                    if let Err(e) = self.client.respawn_pane(&p_id) {
                        self.show_toast(format!("Respawn pane failed: {e}"), ToastLevel::Error);
                    } else {
                        self.show_toast(format!("Respawned pane {p_id}"), ToastLevel::Success);
                        let _ = self.refresh_data();
                    }
                }
            }

            Action::ResizePane(dir, amount) => {
                if let Some(pane) = self.selected_pane() {
                    let p_id = pane.id.clone();
                    if let Err(e) = self.client.resize_pane(&p_id, dir, amount) {
                        self.show_toast(format!("Resize failed: {e}"), ToastLevel::Error);
                    } else {
                        let dir_name = match dir {
                            crate::tmux::client::ResizeDirection::Up => "up",
                            crate::tmux::client::ResizeDirection::Down => "down",
                            crate::tmux::client::ResizeDirection::Left => "left",
                            crate::tmux::client::ResizeDirection::Right => "right",
                        };
                        self.show_toast(
                            format!("Resized pane {} {dir_name} ({amount})", p_id.0),
                            ToastLevel::Success,
                        );
                        let _ = self.refresh_data();
                    }
                }
            }

            Action::ResizeFocusedColumn(delta) => match self.focus {
                FocusColumn::Sessions => {
                    let cur_s = self.column_ratios.0 as i16;
                    let new_s = (cur_s + delta).clamp(12, 45) as u16;
                    let diff = new_s as i16 - cur_s;
                    let cur_p = self.column_ratios.2 as i16;
                    let new_p = (cur_p - diff).max(20) as u16;
                    let new_w = 100 - new_s - new_p;
                    self.column_ratios = (new_s, new_w, new_p);
                }
                FocusColumn::Windows => {
                    let cur_w = self.column_ratios.1 as i16;
                    let new_w = (cur_w + delta).clamp(15, 50) as u16;
                    let diff = new_w as i16 - cur_w;
                    let cur_p = self.column_ratios.2 as i16;
                    let new_p = (cur_p - diff).max(20) as u16;
                    let new_s = 100 - new_w - new_p;
                    self.column_ratios = (new_s, new_w, new_p);
                }
                FocusColumn::Panes => {
                    let cur_p = self.column_ratios.2 as i16;
                    let new_p = (cur_p + delta).clamp(25, 70) as u16;
                    let diff = new_p as i16 - cur_p;
                    let cur_w = self.column_ratios.1 as i16;
                    let new_w = (cur_w - diff).max(15) as u16;
                    let new_s = 100 - new_w - new_p;
                    self.column_ratios = (new_s, new_w, new_p);
                }
            },

            Action::ToggleSidebarMode => {
                let next_mode = match self.sidebar_mode {
                    crate::ui::SidebarMode::Full => crate::ui::SidebarMode::SessionsHidden,
                    crate::ui::SidebarMode::SessionsHidden => crate::ui::SidebarMode::PanesOnly,
                    crate::ui::SidebarMode::WindowsHidden => crate::ui::SidebarMode::PanesOnly,
                    crate::ui::SidebarMode::PanesOnly => crate::ui::SidebarMode::Full,
                };
                return self.update(Action::SetSidebarMode(next_mode));
            }

            Action::SetSidebarMode(mode) => {
                self.sidebar_mode = mode;
                match mode {
                    crate::ui::SidebarMode::Full => {
                        self.show_toast("View: Full (3 columns)".to_string(), ToastLevel::Info);
                    }
                    crate::ui::SidebarMode::SessionsHidden => {
                        if self.focus == FocusColumn::Sessions {
                            self.focus = FocusColumn::Windows;
                        }
                        self.show_toast(
                            "View: Sessions collapsed (Windows + Panes)".to_string(),
                            ToastLevel::Info,
                        );
                    }
                    crate::ui::SidebarMode::WindowsHidden => {
                        if self.focus == FocusColumn::Windows {
                            self.focus = FocusColumn::Sessions;
                        }
                        self.show_toast(
                            "View: Windows collapsed (Sessions + Panes)".to_string(),
                            ToastLevel::Info,
                        );
                    }
                    crate::ui::SidebarMode::PanesOnly => {
                        self.focus = FocusColumn::Panes;
                        self.show_toast(
                            "View: Wide Panes (Sessions & Windows collapsed)".to_string(),
                            ToastLevel::Info,
                        );
                    }
                }
            }

            Action::ToggleSearch => {
                if let Mode::Search { .. } = self.mode {
                    self.mode = Mode::Normal;
                } else {
                    self.mode = Mode::Search {
                        query: String::new(),
                        selected_index: 0,
                        category: SearchCategory::All,
                    };
                }
            }

            Action::SearchNextCategory => {
                if let Mode::Search {
                    category,
                    selected_index,
                    ..
                } = &mut self.mode
                {
                    *category = category.next();
                    *selected_index = 0;
                }
            }

            Action::SearchPrevCategory => {
                if let Mode::Search {
                    category,
                    selected_index,
                    ..
                } = &mut self.mode
                {
                    *category = category.prev();
                    *selected_index = 0;
                }
            }

            Action::SearchInput(c) => {
                if let Mode::Search {
                    query,
                    selected_index,
                    ..
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
                    ..
                } = &mut self.mode
                {
                    query.pop();
                    *selected_index = 0;
                }
            }

            Action::SearchNext => {
                let (query, category) = if let Mode::Search {
                    query, category, ..
                } = &self.mode
                {
                    (Some(query.clone()), *category)
                } else {
                    (None, SearchCategory::All)
                };

                if let Some(q) = query {
                    let results_len = self.filtered_search_results(&q, category).len();
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
                    category,
                } = &self.mode
                {
                    let results = self.filtered_search_results(query, *category);
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
                self.mode = Mode::Normal;
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
                if let Mode::Help = self.mode {
                    self.mode = Mode::Normal;
                    return Ok(None);
                }
                if let Mode::Search { .. } = self.mode {
                    if double_click {
                        return self.update(Action::SearchSelect);
                    }
                    return Ok(None);
                }
                if let Mode::InspectPane { .. } = self.mode {
                    if double_click {
                        return self.update(Action::OpenSelection);
                    }
                    return Ok(None);
                }

                if let Mode::PromptSendCommand { .. } = self.mode {
                    let overlay_area = crate::ui::modals::centered_rect(58, 25, self.last_area);
                    if column >= overlay_area.x
                        && column < overlay_area.x + overlay_area.width
                        && row >= overlay_area.y
                        && row < overlay_area.y + overlay_area.height
                    {
                        if row >= overlay_area.y + 4 && row <= overlay_area.y + 6 {
                            return self.update(Action::TogglePromptWithEnter);
                        }
                    } else {
                        self.mode = Mode::Normal;
                    }
                    return Ok(None);
                }

                let layout = crate::ui::layout::AppLayout::split_with_mode(
                    self.last_area,
                    self.column_ratios,
                    self.sidebar_mode,
                );

                // Check if clicked on a vertical column border to initiate column resizing
                if let Some(border_idx) = layout.find_column_border_at(column, row) {
                    self.mouse_drag_col_border = Some(border_idx);
                    self.mouse_drag_start = None;
                    return Ok(None);
                }

                // Check if clicked in sessions column
                if layout.sessions_col.width > 0
                    && column >= layout.sessions_col.x
                    && column < layout.sessions_col.x + layout.sessions_col.width
                    && row >= layout.sessions_col.y
                    && row < layout.sessions_col.y + layout.sessions_col.height
                {
                    // Check if clicked on [◀] or [▶ Windows] button on header row
                    if row == layout.sessions_col.y {
                        let rel_x = column.saturating_sub(layout.sessions_col.x);
                        if self.sidebar_mode == crate::ui::SidebarMode::WindowsHidden && rel_x >= 14
                        {
                            return self
                                .update(Action::SetSidebarMode(crate::ui::SidebarMode::Full));
                        }
                        if rel_x >= 8
                            || column
                                >= layout.sessions_col.x
                                    + layout.sessions_col.width.saturating_sub(6)
                        {
                            let next_mode =
                                if self.sidebar_mode == crate::ui::SidebarMode::WindowsHidden {
                                    crate::ui::SidebarMode::PanesOnly
                                } else {
                                    crate::ui::SidebarMode::SessionsHidden
                                };
                            return self.update(Action::SetSidebarMode(next_mode));
                        }
                    }

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
                else if layout.windows_col.width > 0
                    && column >= layout.windows_col.x
                    && column < layout.windows_col.x + layout.windows_col.width
                    && row >= layout.windows_col.y
                    && row < layout.windows_col.y + layout.windows_col.height
                {
                    // Check header row buttons ([▶ Sessions] on left, [◀] on right)
                    if row == layout.windows_col.y {
                        let rel_x = column.saturating_sub(layout.windows_col.x);
                        if self.sidebar_mode == crate::ui::SidebarMode::SessionsHidden && rel_x < 14
                        {
                            return self
                                .update(Action::SetSidebarMode(crate::ui::SidebarMode::Full));
                        }
                        let is_collapse = match self.sidebar_mode {
                            crate::ui::SidebarMode::SessionsHidden => {
                                rel_x >= 20
                                    || column
                                        >= layout.windows_col.x
                                            + layout.windows_col.width.saturating_sub(6)
                            }
                            _ => {
                                rel_x >= 8
                                    || column
                                        >= layout.windows_col.x
                                            + layout.windows_col.width.saturating_sub(6)
                            }
                        };
                        if is_collapse {
                            let next_mode =
                                if self.sidebar_mode == crate::ui::SidebarMode::SessionsHidden {
                                    crate::ui::SidebarMode::PanesOnly
                                } else {
                                    crate::ui::SidebarMode::WindowsHidden
                                };
                            return self.update(Action::SetSidebarMode(next_mode));
                        }
                    }

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
                else if layout.panes_col.width > 0
                    && column >= layout.panes_col.x
                    && column < layout.panes_col.x + layout.panes_col.width
                    && row >= layout.panes_col.y
                    && row < layout.panes_col.y + layout.panes_col.height
                {
                    // Check header row buttons ([▶ EXPAND SIDEBARS] or [▶ SESSIONS])
                    if row == layout.panes_col.y
                        && self.sidebar_mode != crate::ui::SidebarMode::Full
                    {
                        let btn_len = if self.sidebar_mode == crate::ui::SidebarMode::PanesOnly {
                            24
                        } else {
                            15
                        };
                        if column < layout.panes_col.x + btn_len {
                            return self
                                .update(Action::SetSidebarMode(crate::ui::SidebarMode::Full));
                        }
                    }

                    self.focus = FocusColumn::Panes;
                    if let Some(window) = self.selected_window() {
                        let inner_panes_area = ratatui::widgets::Block::default()
                            .borders(ratatui::widgets::Borders::ALL)
                            .inner(layout.panes_col);

                        let mut found_pane_id = None;
                        let mut found_rect = None;
                        if let Some(root) = crate::domain::LayoutNode::parse(&window.layout_str)
                            && let Some((p_id, r)) =
                                root.find_pane_rect_at(inner_panes_area, column, row)
                        {
                            found_pane_id = Some(p_id);
                            found_rect = Some(r);
                        }

                        if let Some(p_id) = found_pane_id {
                            if let Some(pos) = window.panes.iter().position(|p| p.id == p_id) {
                                self.selection.pane_idx = pos;
                            }
                            // Check if click was on bottom border controls ([◀] [▼] [▲] [▶] [↕ swap])
                            if let Some(rect) = found_rect
                                && row == rect.y + rect.height.saturating_sub(1)
                            {
                                let col_offset = column.saturating_sub(rect.x);
                                if (1..=4).contains(&col_offset) {
                                    return self.update(Action::ResizePane(
                                        crate::tmux::client::ResizeDirection::Left,
                                        4,
                                    ));
                                } else if (5..=8).contains(&col_offset) {
                                    return self.update(Action::ResizePane(
                                        crate::tmux::client::ResizeDirection::Down,
                                        2,
                                    ));
                                } else if (9..=12).contains(&col_offset) {
                                    return self.update(Action::ResizePane(
                                        crate::tmux::client::ResizeDirection::Up,
                                        2,
                                    ));
                                } else if (13..=16).contains(&col_offset) {
                                    return self.update(Action::ResizePane(
                                        crate::tmux::client::ResizeDirection::Right,
                                        4,
                                    ));
                                } else if (17..=26).contains(&col_offset) {
                                    return self.update(Action::SwapPaneDown);
                                }
                            }
                            self.mouse_drag_start = Some((column, row, p_id));
                        } else if !window.panes.is_empty() && row > inner_panes_area.y {
                            let pane_height =
                                inner_panes_area.height / window.panes.len().max(1) as u16;
                            if let Some(clicked_idx) = row
                                .saturating_sub(inner_panes_area.y)
                                .checked_div(pane_height)
                            {
                                let idx = clicked_idx as usize;
                                if idx < window.panes.len() {
                                    let pane_id = window.panes[idx].id.clone();
                                    self.selection.pane_idx = idx;
                                    self.mouse_drag_start = Some((column, row, pane_id));
                                }
                            }
                        }

                        if double_click {
                            return self.update(Action::OpenSelection);
                        }
                    }
                }
            }

            Action::MouseDrag { column, row } => {
                if let Some(border_idx) = self.mouse_drag_col_border {
                    let layout = crate::ui::layout::AppLayout::split_with_mode(
                        self.last_area,
                        self.column_ratios,
                        self.sidebar_mode,
                    );
                    let total_w = layout.columns_area.width as f32;
                    if total_w > 10.0 {
                        if border_idx == 0 {
                            let rel_x = column.saturating_sub(layout.columns_area.x) as f32;
                            let new_s = ((rel_x / total_w) * 100.0).round() as u16;
                            let clamped_s = new_s.clamp(12, 45);
                            let remain = 100 - clamped_s;
                            let cur_w = self.column_ratios.1 as f32;
                            let cur_p = self.column_ratios.2 as f32;
                            let w_ratio = cur_w / (cur_w + cur_p).max(1.0);
                            let new_w = ((remain as f32) * w_ratio).round() as u16;
                            let clamped_w = new_w.clamp(15, remain.saturating_sub(20));
                            let clamped_p = remain - clamped_w;
                            self.column_ratios = (clamped_s, clamped_w, clamped_p);
                        } else if border_idx == 1 {
                            let rel_x = column.saturating_sub(layout.columns_area.x) as f32;
                            let s_pct = self.column_ratios.0;
                            let target_w_and_s = ((rel_x / total_w) * 100.0).round() as u16;
                            let new_w = target_w_and_s.saturating_sub(s_pct);
                            let max_w = (100 - s_pct).saturating_sub(20);
                            let clamped_w = new_w.clamp(15, max_w);
                            let clamped_p = 100 - s_pct - clamped_w;
                            self.column_ratios = (s_pct, clamped_w, clamped_p);
                        }
                    }
                    return Ok(None);
                }

                if let Some((start_col, start_row, pane_id)) = self.mouse_drag_start.clone() {
                    let dx = column as i32 - start_col as i32;
                    let dy = row as i32 - start_row as i32;
                    if dx >= 3 {
                        let _ = self.client.resize_pane(
                            &pane_id,
                            crate::tmux::client::ResizeDirection::Right,
                            dx.unsigned_abs() as usize,
                        );
                        self.mouse_drag_start = Some((column, row, pane_id));
                        let _ = self.refresh_data();
                    } else if dx <= -3 {
                        let _ = self.client.resize_pane(
                            &pane_id,
                            crate::tmux::client::ResizeDirection::Left,
                            dx.unsigned_abs() as usize,
                        );
                        self.mouse_drag_start = Some((column, row, pane_id));
                        let _ = self.refresh_data();
                    } else if dy >= 2 {
                        let _ = self.client.resize_pane(
                            &pane_id,
                            crate::tmux::client::ResizeDirection::Down,
                            dy.unsigned_abs() as usize,
                        );
                        self.mouse_drag_start = Some((column, row, pane_id));
                        let _ = self.refresh_data();
                    } else if dy <= -2 {
                        let _ = self.client.resize_pane(
                            &pane_id,
                            crate::tmux::client::ResizeDirection::Up,
                            dy.unsigned_abs() as usize,
                        );
                        self.mouse_drag_start = Some((column, row, pane_id));
                        let _ = self.refresh_data();
                    }
                }
            }

            Action::MouseUp => {
                self.mouse_drag_start = None;
                self.mouse_drag_col_border = None;
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

            Action::PromptSendCommand => {
                if let Some(pane) = self.selected_pane() {
                    self.mode = Mode::PromptSendCommand {
                        pane_id: pane.id.clone(),
                        input: String::new(),
                        with_enter: false,
                    };
                }
            }

            Action::TogglePromptWithEnter => {
                if let Mode::PromptSendCommand { with_enter, .. } = &mut self.mode {
                    *with_enter = !*with_enter;
                }
            }

            Action::BreakPane => {
                if let Some(pane) = self.selected_pane() {
                    let pane_id = pane.id.clone();
                    if let Err(e) = self.client.break_pane(&pane_id) {
                        self.show_toast(format!("Break pane failed: {e}"), ToastLevel::Error);
                    } else {
                        self.show_toast(
                            format!("Broke pane {} into new window", pane_id.0),
                            ToastLevel::Success,
                        );
                        let _ = self.refresh_data();
                    }
                }
            }

            Action::CancelModal => {
                self.mode = Mode::Normal;
            }

            Action::ModalInput(c) => match &mut self.mode {
                Mode::PromptNewSession { input }
                | Mode::PromptNewWindow { input, .. }
                | Mode::PromptRenameSession { input, .. }
                | Mode::PromptRenameWindow { input, .. }
                | Mode::PromptSendCommand { input, .. } => {
                    input.push(c);
                }
                _ => {}
            },

            Action::ModalBackspace => match &mut self.mode {
                Mode::PromptNewSession { input }
                | Mode::PromptNewWindow { input, .. }
                | Mode::PromptRenameSession { input, .. }
                | Mode::PromptRenameWindow { input, .. }
                | Mode::PromptSendCommand { input, .. } => {
                    input.pop();
                }
                _ => {}
            },

            Action::ModalSubmit => {
                let mode = self.mode.clone();
                match mode {
                    Mode::PromptNewSession { input } => {
                        let name = sanitize_tmux_name(&input);
                        if !name.is_empty() {
                            if let Err(e) = self.client.create_session(&name) {
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
                        } else {
                            self.show_toast(
                                "Invalid session name (cannot be empty or only symbols)"
                                    .to_string(),
                                ToastLevel::Warning,
                            );
                        }
                    }
                    Mode::PromptNewWindow { session_id, input } => {
                        let name = sanitize_tmux_name(&input);
                        if !name.is_empty() {
                            if let Err(e) = self.client.create_window(&session_id, &name) {
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
                        } else {
                            self.show_toast(
                                "Invalid window name (cannot be empty or only symbols)".to_string(),
                                ToastLevel::Warning,
                            );
                        }
                    }
                    Mode::PromptRenameSession { session_id, input } => {
                        let name = sanitize_tmux_name(&input);
                        if !name.is_empty() {
                            if let Err(e) = self.client.rename_session(&session_id, &name) {
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
                        } else {
                            self.show_toast(
                                "Invalid session name".to_string(),
                                ToastLevel::Warning,
                            );
                        }
                    }
                    Mode::PromptRenameWindow { window_id, input } => {
                        let name = sanitize_tmux_name(&input);
                        if !name.is_empty() {
                            if let Err(e) = self.client.rename_window(&window_id, &name) {
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
                        } else {
                            self.show_toast("Invalid window name".to_string(), ToastLevel::Warning);
                        }
                    }
                    Mode::PromptSendCommand {
                        pane_id,
                        input,
                        with_enter,
                    } => {
                        let trimmed = input.trim();
                        if with_enter {
                            if trimmed.is_empty() {
                                if let Err(e) = self.client.send_keys_with_enter(&pane_id, "") {
                                    self.show_toast(
                                        format!("Send Enter failed: {e}"),
                                        ToastLevel::Error,
                                    );
                                } else {
                                    self.show_toast(
                                        format!("Sent <Enter> to {}", pane_id.0),
                                        ToastLevel::Success,
                                    );
                                    self.refresh_active_window_preview();
                                }
                            } else if let Err(e) =
                                self.client.send_keys_with_enter(&pane_id, trimmed)
                            {
                                self.show_toast(
                                    format!("Send with Enter failed: {e}"),
                                    ToastLevel::Error,
                                );
                            } else {
                                self.show_toast(
                                    format!("Sent to {}: {trimmed} + ↵", pane_id.0),
                                    ToastLevel::Success,
                                );
                                self.refresh_active_window_preview();
                            }
                        } else if !trimmed.is_empty() {
                            // Enter = normal send
                            if let Err(e) = self.client.send_keys(&pane_id, trimmed) {
                                self.show_toast(
                                    format!("Send keys failed: {e}"),
                                    ToastLevel::Error,
                                );
                            } else {
                                self.show_toast(
                                    format!("Sent to {}: {trimmed}", pane_id.0),
                                    ToastLevel::Success,
                                );
                                self.refresh_active_window_preview();
                            }
                        }
                    }
                    _ => {}
                }
                self.mode = Mode::Normal;
            }

            Action::ModalSubmitWithEnter => {
                if let Mode::PromptSendCommand { pane_id, input, .. } = &self.mode {
                    let trimmed = input.trim();
                    if trimmed.is_empty() {
                        if let Err(e) = self.client.send_keys_with_enter(pane_id, "") {
                            self.show_toast(format!("Send Enter failed: {e}"), ToastLevel::Error);
                        } else {
                            self.show_toast(
                                format!("Sent <Enter> to {}", pane_id.0),
                                ToastLevel::Success,
                            );
                            self.refresh_active_window_preview();
                        }
                    } else if let Err(e) = self.client.send_keys_with_enter(pane_id, trimmed) {
                        self.show_toast(format!("Send with Enter failed: {e}"), ToastLevel::Error);
                    } else {
                        self.show_toast(
                            format!("Sent to {}: {trimmed} + ↵", pane_id.0),
                            ToastLevel::Success,
                        );
                        self.refresh_active_window_preview();
                    }
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
