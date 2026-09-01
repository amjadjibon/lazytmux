use super::id::{PaneId, SessionId, WindowId};
use super::pane::Pane;

#[derive(Debug, Clone)]
pub struct Window {
    pub id: WindowId,
    pub session_id: SessionId,
    pub index: u32,
    pub name: String,
    pub active: bool,
    pub layout_str: String,
    pub panes: Vec<Pane>,
}

impl Window {
    pub fn new(
        id: WindowId,
        session_id: SessionId,
        index: u32,
        name: String,
        active: bool,
        layout_str: String,
    ) -> Self {
        Self {
            id,
            session_id,
            index,
            name,
            active,
            layout_str,
            panes: Vec::new(),
        }
    }

    pub fn pane_count(&self) -> usize {
        self.panes.len()
    }

    pub fn active_pane(&self) -> Option<&Pane> {
        self.panes.iter().find(|p| p.active).or_else(|| self.panes.first())
    }

    pub fn get_pane(&self, pane_id: &PaneId) -> Option<&Pane> {
        self.panes.iter().find(|p| &p.id == pane_id)
    }

    pub fn get_pane_mut(&mut self, pane_id: &PaneId) -> Option<&mut Pane> {
        self.panes.iter_mut().find(|p| &p.id == pane_id)
    }
}
