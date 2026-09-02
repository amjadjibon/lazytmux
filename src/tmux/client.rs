use crate::domain::{Pane, PaneId, Session, SessionId, Window, WindowId};
use anyhow::Result;

pub trait TmuxClient: Send + Sync {
    // Queries
    fn list_sessions(&self) -> Result<Vec<Session>>;
    fn list_windows(&self, session: &SessionId) -> Result<Vec<Window>>;
    fn list_panes(&self, window: &WindowId) -> Result<Vec<Pane>>;
    fn fetch_full_tree(&self) -> Result<Vec<Session>>;
    fn capture_pane(&self, pane: &PaneId, lines: usize, preserve_ansi: bool) -> Result<Vec<u8>>;

    // Mutations
    fn create_session(&mut self, name: &str) -> Result<SessionId>;
    fn rename_session(&mut self, session: &SessionId, new_name: &str) -> Result<()>;
    fn kill_session(&mut self, session: &SessionId) -> Result<()>;

    fn create_window(&mut self, session: &SessionId, name: &str) -> Result<WindowId>;
    fn rename_window(&mut self, window: &WindowId, new_name: &str) -> Result<()>;
    fn kill_window(&mut self, window: &WindowId) -> Result<()>;

    fn kill_pane(&mut self, pane: &PaneId) -> Result<()>;
    fn zoom_pane(&mut self, pane: &PaneId) -> Result<()>;
    fn split_pane(&mut self, pane: &PaneId, vertical: bool) -> Result<PaneId>;
    fn select_layout(&mut self, window: &WindowId, layout: &str) -> Result<()>;
    fn toggle_sync_panes(&mut self, window: &WindowId) -> Result<bool>;
    fn swap_pane(&mut self, pane: &PaneId, up: bool) -> Result<()>;
    fn swap_window(&mut self, window: &WindowId, left: bool) -> Result<()>;
    fn respawn_pane(&mut self, pane: &PaneId) -> Result<()>;
    fn send_keys(&mut self, pane: &PaneId, keys: &str) -> Result<()>;
    fn break_pane(&mut self, pane: &PaneId) -> Result<()>;

    // Focus & Navigation
    fn focus_pane(&self, session: &SessionId, window: &WindowId, pane: &PaneId) -> Result<()>;
}
