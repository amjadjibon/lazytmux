use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub struct AppLayout {
    pub header: Rect,
    pub columns_area: Rect,
    pub sessions_col: Rect,
    pub windows_col: Rect,
    pub panes_col: Rect,
    pub breadcrumbs: Rect,
    pub footer: Rect,
}

impl AppLayout {
    pub fn split(area: Rect) -> Self {
        Self::split_with_ratios(area, (22, 28, 50))
    }

    pub fn split_with_ratios(area: Rect, ratios: (u16, u16, u16)) -> Self {
        // Vertical layout:
        // 1. Header (height 1)
        // 2. Main 3 columns (Constraint::Min(5))
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

        let (s_pct, w_pct, p_pct) = ratios;
        let col_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(s_pct),
                Constraint::Percentage(w_pct),
                Constraint::Percentage(p_pct),
            ])
            .split(columns_area);

        Self {
            header,
            columns_area,
            sessions_col: col_chunks[0],
            windows_col: col_chunks[1],
            panes_col: col_chunks[2],
            breadcrumbs,
            footer,
        }
    }

    /// Check if coordinates (x, y) fall on or adjacent to the column vertical split borders.
    /// Returns Some(0) if on border between Sessions & Windows.
    /// Returns Some(1) if on border between Windows & Panes.
    pub fn find_column_border_at(&self, x: u16, y: u16) -> Option<usize> {
        if y < self.columns_area.y || y >= self.columns_area.y + self.columns_area.height {
            return None;
        }

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
}
