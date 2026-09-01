use super::id::{SessionId, WindowId};
use super::window::Window;

#[derive(Debug, Clone)]
pub struct Session {
    pub id: SessionId,
    pub name: String,
    pub window_count: usize,
    pub attached: bool,
    pub is_favorite: bool,
    pub windows: Vec<Window>,
}

impl Session {
    pub fn new(id: SessionId, name: String, window_count: usize, attached: bool) -> Self {
        Self {
            id,
            name,
            window_count,
            attached,
            is_favorite: false,
            windows: Vec::new(),
        }
    }

    pub fn total_panes(&self) -> usize {
        self.windows.iter().map(|w| w.panes.len()).sum()
    }

    pub fn active_window(&self) -> Option<&Window> {
        self.windows
            .iter()
            .find(|w| w.active)
            .or_else(|| self.windows.first())
    }

    pub fn get_window(&self, window_id: &WindowId) -> Option<&Window> {
        self.windows.iter().find(|w| &w.id == window_id)
    }

    pub fn get_window_mut(&mut self, window_id: &WindowId) -> Option<&mut Window> {
        self.windows.iter_mut().find(|w| &w.id == window_id)
    }
}
