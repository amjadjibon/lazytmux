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
    /// True when tmux `synchronize-panes` is on for this window, meaning input
    /// sent to one pane is broadcast to every pane in it.
    pub synchronized: bool,
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
            synchronized: false,
        }
    }

    pub fn pane_count(&self) -> usize {
        self.panes.len()
    }

    pub fn active_pane(&self) -> Option<&Pane> {
        self.panes
            .iter()
            .find(|p| p.active)
            .or_else(|| self.panes.first())
    }

    pub fn get_pane(&self, pane_id: &PaneId) -> Option<&Pane> {
        if let Some(p) = self.panes.iter().find(|p| &p.id == pane_id) {
            return Some(p);
        }
        let num_str = pane_id.0.trim_start_matches('%');
        if let Ok(idx) = num_str.parse::<u32>() {
            if let Some(p) = self.panes.iter().find(|p| p.index == idx) {
                return Some(p);
            }
            if idx > 0 && (idx as usize) <= self.panes.len() {
                return Some(&self.panes[(idx - 1) as usize]);
            }
        }
        None
    }

    pub fn get_pane_mut(&mut self, pane_id: &PaneId) -> Option<&mut Pane> {
        if let Some(pos) = self.panes.iter().position(|p| &p.id == pane_id) {
            return Some(&mut self.panes[pos]);
        }
        let num_str = pane_id.0.trim_start_matches('%');
        if let Ok(idx) = num_str.parse::<u32>() {
            if let Some(pos) = self.panes.iter().position(|p| p.index == idx) {
                return Some(&mut self.panes[pos]);
            }
            if idx > 0 && (idx as usize) <= self.panes.len() {
                return Some(&mut self.panes[(idx - 1) as usize]);
            }
        }
        None
    }
}
