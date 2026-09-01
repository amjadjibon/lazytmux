use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub struct AppLayout {
    pub header: Rect,
    pub sessions_col: Rect,
    pub windows_col: Rect,
    pub panes_col: Rect,
    pub breadcrumbs: Rect,
    pub footer: Rect,
}

impl AppLayout {
    pub fn split(area: Rect) -> Self {
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

        // Horizontal split for columns:
        // Sessions: 22%, Windows: 28%, Panes: 50%
        let col_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(22),
                Constraint::Percentage(28),
                Constraint::Percentage(50),
            ])
            .split(columns_area);

        Self {
            header,
            sessions_col: col_chunks[0],
            windows_col: col_chunks[1],
            panes_col: col_chunks[2],
            breadcrumbs,
            footer,
        }
    }
}
