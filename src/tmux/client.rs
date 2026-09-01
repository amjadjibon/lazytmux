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

    // Focus & Navigation
    fn focus_pane(&self, session: &SessionId, window: &WindowId, pane: &PaneId) -> Result<()>;
}
