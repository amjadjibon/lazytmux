pub mod footer;
pub mod inspect;
pub mod layout;
pub mod modals;
pub mod panes;
pub mod search;
pub mod sessions;
pub mod theme;
pub mod windows;

pub use theme::{Theme, ThemePreset};

use crate::app::App;
use layout::AppLayout;
use ratatui::Frame;

pub fn render(app: &App, frame: &mut Frame) {
    let theme = &app.theme;
    let area = frame.area();
    let app_layout = AppLayout::split_with_ratios(area, app.column_ratios);

    // 1. Header
    footer::render_header(app, frame, app_layout.header, theme);

    // 2. Main 3 columns
    sessions::render(app, frame, app_layout.sessions_col, theme);
    windows::render(app, frame, app_layout.windows_col, theme);
    panes::render(app, frame, app_layout.panes_col, theme);

    // 3. Breadcrumbs
    footer::render_breadcrumbs(app, frame, app_layout.breadcrumbs, theme);

    // 4. Footer
    footer::render_footer(app, frame, app_layout.footer, theme);

    // 5. Active Modals & Overlays
    match &app.mode {
        crate::app::Mode::InspectPane { .. } => {
            inspect::render(app, frame, area, theme);
        }
        crate::app::Mode::Search { .. } => {
            search::render(app, frame, area, theme);
        }
        crate::app::Mode::ConfirmKill(_)
        | crate::app::Mode::PromptNewSession { .. }
        | crate::app::Mode::PromptNewWindow { .. }
        | crate::app::Mode::PromptNewPane { .. }
        | crate::app::Mode::PromptRenameSession { .. }
        | crate::app::Mode::PromptRenameWindow { .. }
        | crate::app::Mode::PromptSendCommand { .. }
        | crate::app::Mode::Help => {
            modals::render(app, frame, area, theme);
        }
        crate::app::Mode::Normal => {}
    }
}
