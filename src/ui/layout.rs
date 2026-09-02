use ratatui::layout::{Constraint, Direction, Layout, Rect};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SidebarMode {
    #[default]
    Full, // Sessions, Windows, Panes all visible
    SessionsHidden, // Sessions hidden, Windows & Panes visible
    WindowsHidden,  // Windows hidden, Sessions & Panes visible
    PanesOnly,      // Both Sessions & Windows hidden, Panes full width
}

pub struct AppLayout {
    pub header: Rect,
    pub columns_area: Rect,
    pub sessions_col: Rect,
    pub windows_col: Rect,
    pub panes_col: Rect,
    pub breadcrumbs: Rect,
    pub footer: Rect,
    pub sidebar_mode: SidebarMode,
}

impl AppLayout {
    pub fn split(area: Rect) -> Self {
        Self::split_with_mode(area, (22, 28, 50), SidebarMode::Full)
    }

    pub fn split_with_ratios(area: Rect, ratios: (u16, u16, u16)) -> Self {
        Self::split_with_mode(area, ratios, SidebarMode::Full)
    }

    pub fn split_with_mode(area: Rect, ratios: (u16, u16, u16), mode: SidebarMode) -> Self {
        // Vertical layout:
        // 1. Header (height 1)
        // 2. Main columns area (Constraint::Min(5))
        // 3. Breadcrumbs (height 1)
        // 4. Footer (height 1)
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Header
                Constraint::Min(5),    // Columns
                Constraint::Length(1), // Breadcrumbs
                Constraint::Length(1), // Footer
            ])
            .split(area);

        let header = main_chunks[0];
        let columns_area = main_chunks[1];
        let breadcrumbs = main_chunks[2];
        let footer = main_chunks[3];

        let (sessions_col, windows_col, panes_col) = match mode {
            SidebarMode::Full => {
                let (s_pct, w_pct, p_pct) = ratios;
                let col_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage(s_pct),
                        Constraint::Percentage(w_pct),
                        Constraint::Percentage(p_pct),
                    ])
                    .split(columns_area);
                (col_chunks[0], col_chunks[1], col_chunks[2])
            }
            SidebarMode::SessionsHidden => {
                let (_, w_pct, p_pct) = ratios;
                let remain = (w_pct + p_pct).max(1) as u32;
                let win_ratio = ((w_pct as u32 * 100) / remain) as u16;
                let panes_ratio = 100 - win_ratio;
                let col_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage(win_ratio),
                        Constraint::Percentage(panes_ratio),
                    ])
                    .split(columns_area);
                let empty_sessions = Rect {
                    x: columns_area.x,
                    y: columns_area.y,
                    width: 0,
                    height: columns_area.height,
                };
                (empty_sessions, col_chunks[0], col_chunks[1])
            }
            SidebarMode::WindowsHidden => {
                let (s_pct, _, p_pct) = ratios;
                let remain = (s_pct + p_pct).max(1) as u32;
                let s_ratio = ((s_pct as u32 * 100) / remain) as u16;
                let panes_ratio = 100 - s_ratio;
                let col_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage(s_ratio),
                        Constraint::Percentage(panes_ratio),
                    ])
                    .split(columns_area);
                let empty_windows = Rect {
                    x: columns_area.x,
                    y: columns_area.y,
                    width: 0,
                    height: columns_area.height,
                };
                (col_chunks[0], empty_windows, col_chunks[1])
            }
            SidebarMode::PanesOnly => {
                let empty_sessions = Rect {
                    x: columns_area.x,
                    y: columns_area.y,
                    width: 0,
                    height: columns_area.height,
                };
                let empty_windows = Rect {
                    x: columns_area.x,
                    y: columns_area.y,
                    width: 0,
                    height: columns_area.height,
                };
                (empty_sessions, empty_windows, columns_area)
            }
        };

        Self {
            header,
            columns_area,
            sessions_col,
            windows_col,
            panes_col,
            breadcrumbs,
            footer,
            sidebar_mode: mode,
        }
    }

    /// Check if coordinates (x, y) fall on or adjacent to visible vertical split borders.
    /// Returns Some(0) if on border between Sessions & Windows (or Sessions & Panes if WindowsHidden).
    /// Returns Some(1) if on border between Windows & Panes.
    pub fn find_column_border_at(&self, x: u16, y: u16) -> Option<usize> {
        if y < self.columns_area.y || y >= self.columns_area.y + self.columns_area.height {
            return None;
        }

        match self.sidebar_mode {
            SidebarMode::Full => {
                // Border between sessions and windows
                let b0 = self.sessions_col.x + self.sessions_col.width;
                if (x as i32 - b0 as i32).abs() <= 1 {
                    return Some(0);
                }

                // Border between windows and panes
                let b1 = self.windows_col.x + self.windows_col.width;
                if (x as i32 - b1 as i32).abs() <= 1 {
                    return Some(1);
                }
                None
            }
            SidebarMode::SessionsHidden => {
                // Only border between windows and panes is visible
                let b1 = self.windows_col.x + self.windows_col.width;
                if (x as i32 - b1 as i32).abs() <= 1 {
                    return Some(1);
                }
                None
            }
            SidebarMode::WindowsHidden => {
                // Only border between sessions and panes is visible
                let b0 = self.sessions_col.x + self.sessions_col.width;
                if (x as i32 - b0 as i32).abs() <= 1 {
                    return Some(0);
                }
                None
            }
            SidebarMode::PanesOnly => None,
        }
    }
}
