use lazytmux::action::{Action, ToastLevel};
use lazytmux::app::{App, FocusColumn, Mode};
use lazytmux::config::Config;
use lazytmux::tmux::MockTmuxClient;

#[test]
fn test_live_cli_client() {
    use lazytmux::tmux::{CliTmuxClient, TmuxClient};
    use std::process::Command;

    // Check if tmux CLI is installed in environment
    if Command::new("tmux").arg("-V").output().is_err() {
        println!("tmux CLI not available, skipping live test");
        return;
    }

    // Spawn a temporary headless test session to ensure at least one session exists
    let test_session = "lazytmux_ci_test_session";
    let _ = Command::new("tmux")
        .args(["new-session", "-d", "-s", test_session, "-n", "ci_window"])
        .output();

    let client = CliTmuxClient::new();
    let tree_res = client.fetch_full_tree();

    // Always cleanup the temporary test session
    let _ = Command::new("tmux")
        .args(["kill-session", "-t", test_session])
        .output();

    let tree = tree_res.expect("fetch_full_tree should succeed");
    assert!(
        !tree.is_empty(),
        "Live tmux session should be discovered when tmux is running"
    );
}

#[test]
fn test_app_initialization_with_mock() {
    let mock = Box::new(MockTmuxClient::new());
    let app = App::new(mock, Config::default(), true);

    assert_eq!(app.focus, FocusColumn::Sessions);
    assert_eq!(app.mode, Mode::Normal);
    assert_eq!(app.sessions.len(), 3);
    assert_eq!(app.selection.session_idx, 0);
    assert_eq!(app.selection.window_idx, 0);
    assert_eq!(app.selection.pane_idx, 0);

    let session = app.selected_session().expect("Session should exist");
    assert_eq!(session.name, "work");
    // Favorites come from the persisted store, not from tmux.
    assert!(!session.is_favorite);

    let window = app.selected_window().expect("Window should exist");
    assert_eq!(window.name, "editor");

    let pane = app.selected_pane().expect("Pane should exist");
    assert_eq!(pane.current_command, "nvim");
}

#[test]
fn test_app_navigation() {
    let mock = Box::new(MockTmuxClient::new());
    let mut app = App::new(mock, Config::default(), true);

    // Navigate down sessions
    app.update(Action::NavigateDown).unwrap();
    assert_eq!(app.selection.session_idx, 1);
    assert_eq!(app.selected_session().unwrap().name, "personal");

    // Navigate right to windows column
    app.update(Action::NavigateRight).unwrap();
    assert_eq!(app.focus, FocusColumn::Windows);
    assert_eq!(app.selected_window().unwrap().name, "blog");

    // Navigate right to panes column
    app.update(Action::NavigateRight).unwrap();
    assert_eq!(app.focus, FocusColumn::Panes);
    assert_eq!(app.selected_pane().unwrap().current_command, "hugo server");

    // Navigate left back to sessions
    app.update(Action::NavigateLeft).unwrap();
    assert_eq!(app.focus, FocusColumn::Windows);
    app.update(Action::NavigateLeft).unwrap();
    assert_eq!(app.focus, FocusColumn::Sessions);
}

#[test]
fn test_column_cycling() {
    let mock = Box::new(MockTmuxClient::new());
    let mut app = App::new(mock, Config::default(), true);

    assert_eq!(app.focus, FocusColumn::Sessions);
    app.update(Action::NextColumn).unwrap();
    assert_eq!(app.focus, FocusColumn::Windows);
    app.update(Action::NextColumn).unwrap();
    assert_eq!(app.focus, FocusColumn::Panes);
    app.update(Action::NextColumn).unwrap();
    assert_eq!(app.focus, FocusColumn::Sessions);

    app.update(Action::PrevColumn).unwrap();
    assert_eq!(app.focus, FocusColumn::Panes);
}

#[test]
fn test_fuzzy_search() {
    let mock = Box::new(MockTmuxClient::new());
    let app = App::new(mock, Config::default(), true);

    let all_items = app.search_items();
    assert!(!all_items.is_empty());

    let results = app.filtered_search_results("nvim", lazytmux::app::SearchCategory::All);
    assert!(!results.is_empty());
    assert!(results[0].command.contains("nvim") || results[0].display_text.contains("nvim"));

    let results_blog = app.filtered_search_results("blog", lazytmux::app::SearchCategory::All);
    assert!(!results_blog.is_empty());
    assert!(
        results_blog[0].window_name.contains("blog")
            || results_blog[0].display_text.contains("blog")
    );
}

#[test]
fn test_inspect_mode_and_scrolling() {
    let mock = Box::new(MockTmuxClient::new());
    let mut app = App::new(mock, Config::default(), true);

    // Open inspect mode
    app.update(Action::ToggleInspect).unwrap();
    match &app.mode {
        Mode::InspectPane { scroll_offset, .. } => {
            assert_eq!(*scroll_offset, 0);
        }
        _ => panic!("Expected InspectPane mode"),
    }

    // Scroll down and up
    app.update(Action::InspectScrollDown(5)).unwrap();
    if let Mode::InspectPane { scroll_offset, .. } = app.mode {
        assert_eq!(scroll_offset, 5);
    }

    app.update(Action::InspectScrollUp(2)).unwrap();
    if let Mode::InspectPane { scroll_offset, .. } = app.mode {
        assert_eq!(scroll_offset, 3);
    }

    app.update(Action::InspectScrollTop).unwrap();
    if let Mode::InspectPane { scroll_offset, .. } = app.mode {
        assert_eq!(scroll_offset, 0);
    }

    // Exit inspect mode
    app.update(Action::ToggleInspect).unwrap();
    assert_eq!(app.mode, Mode::Normal);
}

#[test]
fn test_toast_notification() {
    let mock = Box::new(MockTmuxClient::new());
    let mut app = App::new(mock, Config::default(), true);

    app.update(Action::ShowToast {
        message: "Test notification".to_string(),
        level: ToastLevel::Success,
    })
    .unwrap();

    assert_eq!(app.toasts.len(), 1);
    assert_eq!(app.toasts[0].message, "Test notification");
}

#[test]
fn test_mouse_selection() {
    let mock = Box::new(MockTmuxClient::new());
    let mut app = App::new(mock, Config::default(), true);
    app.last_area = ratatui::layout::Rect::new(0, 0, 100, 30);

    // Click on 1st session row (sessions col top border is y=1, row 0 is y=2)
    app.update(Action::MouseClick {
        column: 5,
        row: 2,
        double_click: false,
    })
    .unwrap();

    assert_eq!(app.focus, lazytmux::app::FocusColumn::Sessions);
    assert_eq!(app.selection.session_idx, 0);

    // Click on 2nd window row in windows col (top border is y=1, row 0 is y=2, row 1 is y=3)
    app.update(Action::MouseClick {
        column: 30,
        row: 3,
        double_click: false,
    })
    .unwrap();

    assert_eq!(app.focus, lazytmux::app::FocusColumn::Windows);
    assert_eq!(app.selection.window_idx, 1);

    // Click on panes col (x: 55..100)
    app.update(Action::MouseClick {
        column: 70,
        row: 5,
        double_click: false,
    })
    .unwrap();

    assert_eq!(app.focus, lazytmux::app::FocusColumn::Panes);
}

#[test]
fn test_search_mode_full_flow() {
    let mock = Box::new(MockTmuxClient::new());
    let mut app = App::new(mock, Config::default(), true);

    // Open search mode
    app.update(Action::ToggleSearch).unwrap();
    assert_eq!(
        app.mode,
        Mode::Search {
            query: String::new(),
            selected_index: 0,
            category: lazytmux::app::SearchCategory::All,
        }
    );

    // Type query: "blog"
    app.update(Action::SearchInput('b')).unwrap();
    app.update(Action::SearchInput('l')).unwrap();
    app.update(Action::SearchInput('o')).unwrap();
    app.update(Action::SearchInput('g')).unwrap();

    if let Mode::Search {
        query,
        selected_index,
        ..
    } = &app.mode
    {
        assert_eq!(query, "blog");
        assert_eq!(*selected_index, 0);
    } else {
        panic!("Expected Search mode");
    }

    // Backspace
    app.update(Action::SearchBackspace).unwrap();
    if let Mode::Search { query, .. } = &app.mode {
        assert_eq!(query, "blo");
    }

    // Select result -> triggers Handoff
    let action = app.update(Action::SearchSelect).unwrap();
    assert_eq!(app.mode, Mode::Normal);
    assert!(matches!(action, Some(Action::Handoff { .. })));
}

#[test]
fn test_toggle_favorite() {
    let mock = Box::new(MockTmuxClient::new());
    let mut app = App::new(mock, Config::default(), true);

    // Favorites start empty: mock mode uses an ephemeral store.
    assert!(!app.selected_session().unwrap().is_favorite);

    // Toggle favorite
    app.update(Action::ToggleFavorite).unwrap();
    assert!(app.selected_session().unwrap().is_favorite);

    // Toggle back
    app.update(Action::ToggleFavorite).unwrap();
    assert!(!app.selected_session().unwrap().is_favorite);
}

#[test]
fn test_toggle_zoom() {
    let mock = Box::new(MockTmuxClient::new());
    let mut app = App::new(mock, Config::default(), true);

    // Initial state is Normal mode
    assert_eq!(app.mode, Mode::Normal);

    // Toggle zoom enters Inspect mode
    app.update(Action::ToggleZoom).unwrap();
    assert!(matches!(app.mode, Mode::InspectPane { .. }));

    // Toggle zoom again exits Inspect mode back to Normal
    app.update(Action::ToggleZoom).unwrap();
    assert_eq!(app.mode, Mode::Normal);
}

#[test]
fn test_help_modal_toggle() {
    let mock = Box::new(MockTmuxClient::new());
    let mut app = App::new(mock, Config::default(), true);

    assert_eq!(app.mode, Mode::Normal);
    app.update(Action::Help).unwrap();
    assert_eq!(app.mode, Mode::Help);

    app.update(Action::Help).unwrap();
    assert_eq!(app.mode, Mode::Normal);
}

#[test]
fn test_copy_pane_output() {
    let mock = Box::new(MockTmuxClient::new());
    let mut app = App::new(mock, Config::default(), true);

    app.update(Action::CopyPaneOutput).unwrap();
    assert!(!app.toasts.is_empty());
    let msg = &app.toasts.last().unwrap().message;
    assert!(
        msg.contains("Copied")
            || msg.contains("Clipboard unavailable")
            || msg.contains("Failed to copy"),
        "Unexpected toast message: {msg}"
    );
}

#[test]
fn test_mouse_scroll_inspect() {
    let mock = Box::new(MockTmuxClient::new());
    let mut app = App::new(mock, Config::default(), true);

    app.mode = Mode::InspectPane {
        pane_id: lazytmux::domain::PaneId::from("%1"),
        scroll_offset: 0,
        search_query: None,
        is_searching: false,
    };

    // Mouse scroll down in inspect mode
    app.update(Action::MouseScrollDown {
        column: 10,
        row: 10,
    })
    .unwrap();
    if let Mode::InspectPane { scroll_offset, .. } = app.mode {
        assert_eq!(scroll_offset, 3);
    } else {
        panic!("Expected InspectPane mode");
    }

    // Mouse scroll up in inspect mode
    app.update(Action::MouseScrollUp {
        column: 10,
        row: 10,
    })
    .unwrap();
    if let Mode::InspectPane { scroll_offset, .. } = app.mode {
        assert_eq!(scroll_offset, 0);
    }
}

#[test]
fn test_selection_clamping_edge_cases() {
    let mock = Box::new(MockTmuxClient::new());
    let mut app = App::new(mock, Config::default(), true);

    // Session 0 has 2 windows
    app.selection.window_idx = 1;
    assert_eq!(app.selection.window_idx, 1);

    // Navigate down to Session 1 ("personal", which only has 1 window)
    app.update(Action::NavigateDown).unwrap();
    assert_eq!(app.selection.session_idx, 1);
    assert_eq!(app.selection.window_idx, 0);
    assert_eq!(app.selection.pane_idx, 0);
}

#[test]
fn test_live_theme_cycling() {
    let mock = Box::new(MockTmuxClient::new());
    let mut app = App::new(mock, Config::default(), true);

    assert_eq!(app.theme.preset, lazytmux::ui::ThemePreset::Default);

    // Cycle theme
    app.update(Action::NextTheme).unwrap();
    assert_eq!(app.theme.preset, lazytmux::ui::ThemePreset::TokyoNight);
    assert!(app.toasts.last().unwrap().message.contains("Tokyo Night"));

    app.update(Action::NextTheme).unwrap();
    assert_eq!(app.theme.preset, lazytmux::ui::ThemePreset::Catppuccin);

    // Prev theme
    app.update(Action::PrevTheme).unwrap();
    assert_eq!(app.theme.preset, lazytmux::ui::ThemePreset::TokyoNight);
}

#[test]
fn test_layout_and_sync_actions() {
    let mock = Box::new(MockTmuxClient::new());
    let mut app = App::new(mock, Config::default(), true);

    app.update(Action::NextLayout).unwrap();
    assert!(app.toasts.last().unwrap().message.contains("Layout:"));

    app.update(Action::ToggleSyncPanes).unwrap();
    assert!(
        app.toasts
            .last()
            .unwrap()
            .message
            .contains("Synchronize panes")
    );
}

#[test]
fn test_swap_pane_and_move_window() {
    let mock = Box::new(MockTmuxClient::new());
    let mut app = App::new(mock, Config::default(), true);

    // Move window
    app.update(Action::MoveWindowRight).unwrap();
    assert!(app.toasts.last().unwrap().message.contains("Moved window"));

    // Swap pane
    app.update(Action::SwapPaneDown).unwrap();
    assert!(app.toasts.last().unwrap().message.contains("Swapped pane"));
}

#[test]
fn test_search_categories_tabbing() {
    let mock = Box::new(MockTmuxClient::new());
    let mut app = App::new(mock, Config::default(), true);

    app.update(Action::ToggleSearch).unwrap();
    assert!(matches!(
        app.mode,
        Mode::Search {
            category: lazytmux::app::SearchCategory::All,
            ..
        }
    ));

    app.update(Action::SearchNextCategory).unwrap();
    assert!(matches!(
        app.mode,
        Mode::Search {
            category: lazytmux::app::SearchCategory::Sessions,
            ..
        }
    ));

    app.update(Action::SearchNextCategory).unwrap();
    assert!(matches!(
        app.mode,
        Mode::Search {
            category: lazytmux::app::SearchCategory::Windows,
            ..
        }
    ));

    app.update(Action::SearchPrevCategory).unwrap();
    assert!(matches!(
        app.mode,
        Mode::Search {
            category: lazytmux::app::SearchCategory::Sessions,
            ..
        }
    ));
}

#[test]
fn test_multiple_panes_different_branches() {
    let mock = Box::new(MockTmuxClient::new());
    let mut app = App::new(mock, Config::default(), true);

    // Give pane 1 branch "main" and pane 2 branch "dev" in window 0
    let win = app.sessions[0].windows.get_mut(0).unwrap();
    win.panes[0].git_branch = Some("main".to_string());
    if win.panes.len() > 1 {
        win.panes[1].git_branch = Some("dev".to_string());
    }

    assert_eq!(win.panes[0].git_branch.as_deref(), Some("main"));
    if win.panes.len() > 1 {
        assert_eq!(win.panes[1].git_branch.as_deref(), Some("dev"));
    }
}

#[test]
fn test_send_keys_to_pane() {
    let mock = Box::new(MockTmuxClient::new());
    let mut app = App::new(mock, Config::default(), true);

    app.focus = FocusColumn::Panes;
    app.update(Action::PromptSendCommand).unwrap();
    assert!(matches!(app.mode, Mode::PromptSendCommand { .. }));

    // 1. Input command and submit with Enter
    for c in "echo hello".chars() {
        app.update(Action::ModalInput(c)).unwrap();
    }
    app.update(Action::ModalSubmit).unwrap();
    assert_eq!(app.mode, Mode::Normal);
    assert!(app.toasts.last().unwrap().message.contains("echo hello"));

    // 2. Submit empty input with Enter (sends pure Enter keypress to pane)
    app.update(Action::PromptSendCommand).unwrap();
    app.update(Action::ModalSubmit).unwrap();
    assert_eq!(app.mode, Mode::Normal);
    assert!(app.toasts.last().unwrap().message.contains("<Enter>"));
}

#[test]
fn test_break_pane() {
    let mock = Box::new(MockTmuxClient::new());
    let mut app = App::new(mock, Config::default(), true);

    app.focus = FocusColumn::Panes;
    let initial_win_count = app.sessions[0].windows.len();
    app.update(Action::BreakPane).unwrap();

    assert!(app.toasts.last().unwrap().message.contains("Broke pane"));
    assert_eq!(app.sessions[0].windows.len(), initial_win_count + 1);
}

#[test]
fn test_inspect_in_buffer_search() {
    let mock = Box::new(MockTmuxClient::new());
    let mut app = App::new(mock, Config::default(), true);

    app.focus = FocusColumn::Panes;
    app.update(Action::ToggleInspect).unwrap();

    // Set sample preview lines
    let s_idx = app.selection.session_idx;
    let w_idx = app.selection.window_idx;
    let p_idx = app.selection.pane_idx;
    app.sessions[s_idx].windows[w_idx].panes[p_idx].preview_lines = vec![
        "Line 0: compiling target".to_string(),
        "Line 1: warning: unused mut".to_string(),
        "Line 2: error[E0308]: mismatched types".to_string(),
        "Line 3: note: expected bool".to_string(),
        "Line 4: error[E0425]: cannot find value".to_string(),
        "Line 5: finished".to_string(),
    ];

    // Start search
    app.update(Action::InspectStartSearch).unwrap();
    if let Mode::InspectPane { is_searching, .. } = &app.mode {
        assert!(is_searching);
    }

    // Type "error"
    for c in "error".chars() {
        app.update(Action::InspectSearchInput(c)).unwrap();
    }

    // First match is Line 2
    if let Mode::InspectPane { scroll_offset, .. } = &app.mode {
        assert_eq!(*scroll_offset, 2);
    }

    // Submit search
    app.update(Action::InspectSearchSubmit).unwrap();
    if let Mode::InspectPane { is_searching, .. } = &app.mode {
        assert!(!is_searching);
    }

    // Next match should jump to Line 4
    app.update(Action::InspectSearchNext).unwrap();
    if let Mode::InspectPane { scroll_offset, .. } = &app.mode {
        assert_eq!(*scroll_offset, 4);
    }

    // Previous match should jump back to Line 2
    app.update(Action::InspectSearchPrev).unwrap();
    if let Mode::InspectPane { scroll_offset, .. } = &app.mode {
        assert_eq!(*scroll_offset, 2);
    }
}

#[test]
fn test_resize_pane_actions() {
    let mock = Box::new(MockTmuxClient::new());
    let mut app = App::new(mock, Config::default(), true);

    app.focus = FocusColumn::Panes;
    let initial_toasts_len = app.toasts.len();

    // Resize Up
    app.update(Action::ResizePane(
        lazytmux::tmux::client::ResizeDirection::Up,
        3,
    ))
    .unwrap();
    assert!(app.toasts.len() > initial_toasts_len);
    assert!(app.toasts.last().unwrap().message.contains("Resized pane"));

    // Resize Down
    app.update(Action::ResizePane(
        lazytmux::tmux::client::ResizeDirection::Down,
        3,
    ))
    .unwrap();
    assert!(app.toasts.last().unwrap().message.contains("down"));

    // Resize Left
    app.update(Action::ResizePane(
        lazytmux::tmux::client::ResizeDirection::Left,
        4,
    ))
    .unwrap();
    assert!(app.toasts.last().unwrap().message.contains("left"));

    // Resize Right
    app.update(Action::ResizePane(
        lazytmux::tmux::client::ResizeDirection::Right,
        4,
    ))
    .unwrap();
    assert!(app.toasts.last().unwrap().message.contains("right"));
}

#[test]
fn test_mouse_drag_resize() {
    let mock = Box::new(MockTmuxClient::new());
    let mut app = App::new(mock, Config::default(), true);

    app.focus = FocusColumn::Panes;
    let p_id = app.selected_pane().unwrap().id.clone();
    app.mouse_drag_start = Some((10, 10, p_id));

    // Drag right by 5 units
    app.update(Action::MouseDrag {
        column: 15,
        row: 10,
    })
    .unwrap();
    assert_eq!(app.mouse_drag_start.as_ref().unwrap().0, 15);

    // Mouse up clears drag anchor
    app.update(Action::MouseUp).unwrap();
    assert!(app.mouse_drag_start.is_none());
}

#[test]
fn test_column_border_detection_and_mouse_drag() {
    let mock = Box::new(MockTmuxClient::new());
    let mut app = App::new(mock, Config::default(), true);

    let area = ratatui::layout::Rect::new(0, 0, 100, 30);
    app.last_area = area;

    let layout = lazytmux::ui::layout::AppLayout::split_with_ratios(area, app.column_ratios);
    // Border 0 is between sessions and windows
    let b0 = layout.sessions_col.x + layout.sessions_col.width;
    assert_eq!(layout.find_column_border_at(b0, 10), Some(0));

    // Click on border 0
    app.update(Action::MouseClick {
        column: b0,
        row: 10,
        double_click: false,
    })
    .unwrap();
    assert_eq!(app.mouse_drag_col_border, Some(0));

    // Drag border right to x = 30
    app.update(Action::MouseDrag {
        column: 30,
        row: 10,
    })
    .unwrap();
    assert_eq!(app.column_ratios.0, 30);
    assert_eq!(
        app.column_ratios.0 + app.column_ratios.1 + app.column_ratios.2,
        100
    );

    // Mouse up clears drag state
    app.update(Action::MouseUp).unwrap();
    assert!(app.mouse_drag_col_border.is_none());
}

#[test]
fn test_keyboard_column_resize() {
    let mock = Box::new(MockTmuxClient::new());
    let mut app = App::new(mock, Config::default(), true);

    let initial_s = app.column_ratios.0;
    app.focus = FocusColumn::Sessions;

    // Expand Sessions column
    app.update(Action::ResizeFocusedColumn(4)).unwrap();
    assert_eq!(app.column_ratios.0, initial_s + 4);
    assert_eq!(
        app.column_ratios.0 + app.column_ratios.1 + app.column_ratios.2,
        100
    );

    // Shrink Sessions column
    app.update(Action::ResizeFocusedColumn(-2)).unwrap();
    assert_eq!(app.column_ratios.0, initial_s + 2);
    assert_eq!(
        app.column_ratios.0 + app.column_ratios.1 + app.column_ratios.2,
        100
    );
}

#[test]
fn test_sidebar_mode_cycle() {
    let mock = Box::new(MockTmuxClient::new());
    let mut app = App::new(mock, Config::default(), true);

    assert_eq!(app.sidebar_mode, lazytmux::ui::SidebarMode::Full);
    app.focus = FocusColumn::Sessions;

    // Toggle 1: Full -> SessionsHidden
    app.update(Action::ToggleSidebarMode).unwrap();
    assert_eq!(app.sidebar_mode, lazytmux::ui::SidebarMode::SessionsHidden);
    assert_eq!(app.focus, FocusColumn::Windows);
    assert!(
        app.toasts
            .last()
            .unwrap()
            .message
            .contains("Sessions collapsed")
    );

    // Toggle 2: SessionsHidden -> PanesOnly
    app.update(Action::ToggleSidebarMode).unwrap();
    assert_eq!(app.sidebar_mode, lazytmux::ui::SidebarMode::PanesOnly);
    assert_eq!(app.focus, FocusColumn::Panes);
    assert!(app.toasts.last().unwrap().message.contains("Wide Panes"));

    // Toggle 3: PanesOnly -> Full
    app.update(Action::ToggleSidebarMode).unwrap();
    assert_eq!(app.sidebar_mode, lazytmux::ui::SidebarMode::Full);
    assert!(app.toasts.last().unwrap().message.contains("Full"));
}

#[test]
fn test_sidebar_mode_layout_geometry() {
    let area = ratatui::layout::Rect::new(0, 0, 100, 30);
    let ratios = (22, 28, 50);

    // Full: all 3 columns have positive width
    let full =
        lazytmux::ui::AppLayout::split_with_mode(area, ratios, lazytmux::ui::SidebarMode::Full);
    assert!(full.sessions_col.width > 0);
    assert!(full.windows_col.width > 0);
    assert!(full.panes_col.width > 0);

    // SessionsHidden: sessions width is 0, windows + panes share full width
    let hidden = lazytmux::ui::AppLayout::split_with_mode(
        area,
        ratios,
        lazytmux::ui::SidebarMode::SessionsHidden,
    );
    assert_eq!(hidden.sessions_col.width, 0);
    assert!(hidden.windows_col.width > 0);
    assert!(hidden.panes_col.width > 0);
    assert_eq!(
        hidden.windows_col.width + hidden.panes_col.width,
        full.columns_area.width
    );

    // PanesOnly: panes occupies 100% of columns area width
    let panes_only = lazytmux::ui::AppLayout::split_with_mode(
        area,
        ratios,
        lazytmux::ui::SidebarMode::PanesOnly,
    );
    assert_eq!(panes_only.sessions_col.width, 0);
    assert_eq!(panes_only.windows_col.width, 0);
    assert_eq!(panes_only.panes_col.width, full.columns_area.width);
}

/// First column showing `control` in a header strip.
fn column_of_header_control(
    strip: &lazytmux::ui::HeaderStrip,
    control: lazytmux::ui::HeaderControl,
    width: u16,
) -> u16 {
    (0..width)
        .find(|c| strip.control_at(*c) == Some(control))
        .unwrap_or_else(|| panic!("{control:?} not present in header of width {width}"))
}

#[test]
fn test_mouse_header_buttons_collapse_expand() {
    let mock = Box::new(MockTmuxClient::new());
    let mut app = App::new(mock, Config::default(), true);

    let area = ratatui::layout::Rect::new(0, 0, 100, 30);
    app.last_area = area;

    let layout =
        lazytmux::ui::AppLayout::split_with_mode(area, app.column_ratios, app.sidebar_mode);

    // 1. Click [◀] in the Sessions header, where it is actually drawn.
    let collapse = column_of_header_control(
        &lazytmux::ui::header::sessions_header(layout.sessions_col.width, app.sidebar_mode),
        lazytmux::ui::HeaderControl::Collapse,
        layout.sessions_col.width,
    );
    app.update(Action::MouseClick {
        column: layout.sessions_col.x + collapse,
        row: layout.sessions_col.y,
        double_click: false,
    })
    .unwrap();
    assert_eq!(app.sidebar_mode, lazytmux::ui::SidebarMode::SessionsHidden);

    // 2. Click [◀] in the Windows header.
    let layout2 =
        lazytmux::ui::AppLayout::split_with_mode(area, app.column_ratios, app.sidebar_mode);
    let collapse = column_of_header_control(
        &lazytmux::ui::header::windows_header(layout2.windows_col.width, app.sidebar_mode),
        lazytmux::ui::HeaderControl::Collapse,
        layout2.windows_col.width,
    );
    app.update(Action::MouseClick {
        column: layout2.windows_col.x + collapse,
        row: layout2.windows_col.y,
        double_click: false,
    })
    .unwrap();
    assert_eq!(app.sidebar_mode, lazytmux::ui::SidebarMode::PanesOnly);

    // 3. Click on [▶ EXPAND SIDEBARS] in Panes header (left side of panes header)
    let layout3 =
        lazytmux::ui::AppLayout::split_with_mode(area, app.column_ratios, app.sidebar_mode);
    app.update(Action::MouseClick {
        column: layout3.panes_col.x + 3,
        row: layout3.panes_col.y,
        double_click: false,
    })
    .unwrap();
    assert_eq!(app.sidebar_mode, lazytmux::ui::SidebarMode::Full);

    // 4. Click [◀] in the Windows header from Full mode (collapses Windows).
    let layout4 =
        lazytmux::ui::AppLayout::split_with_mode(area, app.column_ratios, app.sidebar_mode);
    let collapse = column_of_header_control(
        &lazytmux::ui::header::windows_header(layout4.windows_col.width, app.sidebar_mode),
        lazytmux::ui::HeaderControl::Collapse,
        layout4.windows_col.width,
    );
    app.update(Action::MouseClick {
        column: layout4.windows_col.x + collapse,
        row: layout4.windows_col.y,
        double_click: false,
    })
    .unwrap();
    assert_eq!(app.sidebar_mode, lazytmux::ui::SidebarMode::WindowsHidden);

    // 5. Click [▶ Windows] in the Sessions header (restores Windows).
    let layout5 =
        lazytmux::ui::AppLayout::split_with_mode(area, app.column_ratios, app.sidebar_mode);
    let expand = column_of_header_control(
        &lazytmux::ui::header::sessions_header(layout5.sessions_col.width, app.sidebar_mode),
        lazytmux::ui::HeaderControl::Expand,
        layout5.sessions_col.width,
    );
    app.update(Action::MouseClick {
        column: layout5.sessions_col.x + expand,
        row: layout5.sessions_col.y,
        double_click: false,
    })
    .unwrap();
    assert_eq!(app.sidebar_mode, lazytmux::ui::SidebarMode::Full);
}

#[test]
fn test_mouse_double_click_triggers_enter() {
    let mock = Box::new(MockTmuxClient::new());
    let mut app = App::new(mock, Config::default(), true);

    let area = ratatui::layout::Rect::new(0, 0, 100, 30);
    app.last_area = area;
    let layout = lazytmux::ui::AppLayout::split(area);

    // Double-clicking on a session item (row = sessions_col.y + 1)
    let action = app
        .update(Action::MouseClick {
            column: layout.sessions_col.x + 2,
            row: layout.sessions_col.y + 1,
            double_click: true,
        })
        .unwrap();

    // Verify it triggers Action::Handoff (Action::OpenSelection / Enter equivalent!)
    assert!(action.is_some());
    match action.unwrap() {
        Action::Handoff { .. } => {}
        other => panic!("Expected Handoff action on double click, got {:?}", other),
    }

    // Also verify handle_mouse_event detects two quick clicks as double_click: true
    let m1 = crossterm::event::MouseEvent {
        kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
        column: 10,
        row: 5,
        modifiers: crossterm::event::KeyModifiers::NONE,
    };
    let a1 = app.handle_mouse_event(m1, area);
    assert_eq!(
        a1,
        Some(Action::MouseClick {
            column: 10,
            row: 5,
            double_click: false,
        })
    );

    let m2 = crossterm::event::MouseEvent {
        kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
        column: 10,
        row: 5,
        modifiers: crossterm::event::KeyModifiers::NONE,
    };
    let a2 = app.handle_mouse_event(m2, area);
    assert_eq!(
        a2,
        Some(Action::MouseClick {
            column: 10,
            row: 5,
            double_click: true,
        })
    );
}

// ---------------------------------------------------------------------------
// Regression tests for the code-review fixes.
// ---------------------------------------------------------------------------

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn make_app() -> App {
    App::new(Box::new(MockTmuxClient::new()), Config::default(), true)
}

/// A byte-index length cap used to split multi-byte names mid-character and
/// panic. `PromptRenameSession` pre-fills the existing name, so any session
/// named with >21 CJK characters crashed on rename.
#[test]
fn test_submitting_multibyte_name_does_not_panic() {
    for filler in ["日", "🚀", "é"] {
        let mut app = make_app();
        app.mode = Mode::PromptRenameSession {
            session_id: lazytmux::domain::SessionId::from("$1"),
            input: filler.repeat(40),
        };
        app.update(Action::ModalSubmit)
            .expect("submit must not fail");
        assert_eq!(app.mode, Mode::Normal);

        let mut app = make_app();
        app.mode = Mode::PromptNewSession {
            input: filler.repeat(40),
        };
        app.update(Action::ModalSubmit)
            .expect("submit must not fail");
    }
}

/// Inspect mode holds a deep buffer plus an offset into it. A periodic preview
/// refresh must not leave the offset pointing past the end of a shorter buffer.
#[test]
fn test_inspect_offset_survives_shrinking_buffer() {
    let mut app = make_app();
    let deep: Vec<String> = (0..300).map(|i| format!("needle line {i}")).collect();
    {
        let w = app.sessions[0].windows.get_mut(0).unwrap();
        w.panes[0].set_preview(deep.join("\n").into_bytes());
    }
    let pane_id = app.sessions[0].windows[0].panes[0].id.clone();
    app.mode = Mode::InspectPane {
        pane_id,
        scroll_offset: 250,
        search_query: Some("needle".to_string()),
        is_searching: false,
    };

    // The pane is cleared underneath us.
    {
        let w = app.sessions[0].windows.get_mut(0).unwrap();
        w.panes[0].set_preview(b"needle line 0\nneedle line 1".to_vec());
    }

    app.update(Action::InspectSearchPrev)
        .expect("must not panic");
    app.update(Action::InspectSearchNext)
        .expect("must not panic");
    app.update(Action::InspectScrollDown(5))
        .expect("must not panic");
    if let Mode::InspectPane { scroll_offset, .. } = app.mode {
        assert!(
            scroll_offset < 2,
            "offset {scroll_offset} escaped the buffer"
        );
    } else {
        panic!("should still be in inspect mode");
    }
}

/// A tick refresh used to replace Inspect mode's deep capture with the short
/// preview capture, collapsing the scrollback the user was reading.
#[test]
fn test_tick_keeps_inspect_scrollback_depth() {
    let mut app = make_app();
    app.focus = FocusColumn::Panes;
    app.update(Action::ToggleInspect).unwrap();
    let inspected = match &app.mode {
        Mode::InspectPane { pane_id, .. } => pane_id.clone(),
        _ => panic!("expected inspect mode"),
    };
    let before = app
        .selected_window()
        .and_then(|w| w.get_pane(&inspected))
        .map(|p| p.preview_lines.len())
        .unwrap();

    app.update(Action::Tick).unwrap();

    let after = app
        .selected_window()
        .and_then(|w| w.get_pane(&inspected))
        .map(|p| p.preview_lines.len())
        .unwrap();
    assert_eq!(before, after, "tick shrank the inspect buffer");
}

/// The footer advertises `l` as the layout cycle in the Panes column.
#[test]
fn test_l_cycles_layout_in_panes_and_navigates_elsewhere() {
    let mut app = make_app();

    app.focus = FocusColumn::Panes;
    assert_eq!(
        app.handle_key_event(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE)),
        Some(Action::NextLayout)
    );

    app.focus = FocusColumn::Sessions;
    assert_eq!(
        app.handle_key_event(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE)),
        Some(Action::NavigateRight)
    );

    // Shift+L is a resize again, not a second layout binding.
    app.focus = FocusColumn::Panes;
    assert_eq!(
        app.handle_key_event(KeyEvent::new(KeyCode::Char('L'), KeyModifiers::SHIFT)),
        Some(Action::ResizePane(
            lazytmux::tmux::client::ResizeDirection::Right,
            4
        ))
    );
}

/// Ctrl+R meant "refresh" in two columns and "kill and restart the pane's
/// process" in the third. Refresh is now unconditional; respawn has its own key
/// and a confirmation step.
#[test]
fn test_respawn_requires_confirmation_and_ctrl_r_always_refreshes() {
    let mut app = make_app();
    for focus in [
        FocusColumn::Sessions,
        FocusColumn::Windows,
        FocusColumn::Panes,
    ] {
        app.focus = focus;
        assert_eq!(
            app.handle_key_event(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL)),
            Some(Action::Refresh)
        );
    }

    app.focus = FocusColumn::Panes;
    assert_eq!(
        app.handle_key_event(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL)),
        Some(Action::PromptRespawnPane)
    );
    app.update(Action::PromptRespawnPane).unwrap();
    assert!(matches!(
        app.mode,
        Mode::Confirm(lazytmux::app::ConfirmTarget::RespawnPane(..))
    ));

    // Escaping the modal must not respawn anything.
    app.update(Action::CancelModal).unwrap();
    assert_eq!(app.mode, Mode::Normal);
}

/// `confirm_on_kill = false` was parsed, documented, and never read.
#[test]
fn test_confirm_on_kill_false_skips_the_modal() {
    let config = lazytmux::config::Config {
        confirm_on_kill: false,
        ..Default::default()
    };
    let mut app = App::new(Box::new(MockTmuxClient::new()), config, true);
    app.focus = FocusColumn::Windows;
    let before = app.selected_session().unwrap().windows.len();

    app.update(Action::PromptKill).unwrap();

    assert_eq!(app.mode, Mode::Normal, "modal should have been skipped");
    assert_eq!(app.selected_session().unwrap().windows.len(), before - 1);
}

/// Favorites live outside the tmux tree, so a refresh must not clear them.
#[test]
fn test_favorite_survives_refresh() {
    let mut app = make_app();
    let name = app.selected_session().unwrap().name.clone();
    let starred_before = app.selected_session().unwrap().is_favorite;

    app.update(Action::ToggleFavorite).unwrap();
    let starred = app.selected_session().unwrap().is_favorite;
    assert_ne!(starred, starred_before);
    assert_eq!(app.favorites.contains(&name), starred);

    app.update(Action::Refresh).unwrap();
    assert_eq!(
        app.selected_session().unwrap().is_favorite,
        starred,
        "refresh discarded the favorite"
    );
}

/// With synchronize-panes on, tmux send-keys broadcasts to every pane in the
/// window, so the prompt has to say so.
#[test]
fn test_send_prompt_flags_broadcast_when_synchronized() {
    let mut app = make_app();
    app.focus = FocusColumn::Panes;

    app.update(Action::PromptSendCommand).unwrap();
    assert!(matches!(
        app.mode,
        Mode::PromptSendCommand {
            broadcast: false,
            ..
        }
    ));
    app.update(Action::CancelModal).unwrap();

    app.sessions[0].windows[0].synchronized = true;
    app.update(Action::PromptSendCommand).unwrap();
    assert!(matches!(
        app.mode,
        Mode::PromptSendCommand {
            broadcast: true,
            ..
        }
    ));

    app.update(Action::ModalSubmit).unwrap();
    let toast = app.toasts.last().unwrap();
    assert!(toast.message.contains("ALL panes"), "got {}", toast.message);
    assert_eq!(toast.level, lazytmux::action::ToastLevel::Warning);
}

/// tmux `send-keys` honours `synchronize-panes`, so the flag must be read back
/// from a real server, not just from the mock.
#[test]
fn test_live_synchronized_flag_round_trip() {
    use lazytmux::tmux::{CliTmuxClient, TmuxClient};
    use std::process::Command;

    if Command::new("tmux").arg("-V").output().is_err() {
        println!("tmux CLI not available, skipping live test");
        return;
    }

    let session = "lazytmux_ci_sync_test";
    let _ = Command::new("tmux")
        .args(["new-session", "-d", "-s", session, "-n", "syncwin"])
        .output();
    let _ = Command::new("tmux")
        .args(["split-window", "-t", session])
        .output();

    let client = CliTmuxClient::new();
    let unsynced = client.fetch_full_tree().ok().and_then(|tree| {
        tree.iter()
            .find(|s| s.name == session)
            .and_then(|s| s.windows.first().map(|w| w.synchronized))
    });

    let _ = Command::new("tmux")
        .args([
            "set-window-option",
            "-t",
            session,
            "synchronize-panes",
            "on",
        ])
        .output();

    let synced = client.fetch_full_tree().ok().and_then(|tree| {
        tree.iter()
            .find(|s| s.name == session)
            .and_then(|s| s.windows.first().map(|w| w.synchronized))
    });

    let _ = Command::new("tmux")
        .args(["kill-session", "-t", session])
        .output();

    assert_eq!(
        unsynced,
        Some(false),
        "window reported sync before it was on"
    );
    assert_eq!(synced, Some(true), "sync-panes was not detected");
}

/// `Pane::new` runs for every pane on every refresh; the branch lookup behind it
/// walks the working tree. It must be memoised, not repeated per refresh.
#[test]
fn test_git_branch_lookup_is_cached() {
    use lazytmux::domain::pane::detect_git_branch;
    use std::path::PathBuf;
    use std::time::Instant;

    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let deep = repo.join("src").join("ui");

    // Warm the cache, then time a large number of repeat lookups.
    let first = detect_git_branch(&deep);
    assert!(first.is_some(), "test must run inside the repo");

    let start = Instant::now();
    for _ in 0..5_000 {
        assert_eq!(detect_git_branch(&deep), first);
    }
    let elapsed = start.elapsed();

    // 5,000 uncached walks would be 5,000 * (up to 12 stat + an open). Cached
    // lookups are a hash probe; anything near the uncached cost fails here.
    assert!(
        elapsed.as_millis() < 100,
        "5000 cached lookups took {elapsed:?} — cache is not being hit"
    );
}

/// Batching several `capture-pane` calls into one `tmux` invocation must return
/// exactly what the one-at-a-time path returns. Spawning tmux costs ~4ms, which
/// dwarfs everything this program computes, so this is the refresh hot path.
#[test]
fn test_live_batched_capture_matches_individual() {
    use lazytmux::domain::PaneId;
    use lazytmux::tmux::{CliTmuxClient, TmuxClient};
    use std::process::Command;

    if Command::new("tmux").arg("-V").output().is_err() {
        println!("tmux CLI not available, skipping live test");
        return;
    }

    let session = "lazytmux_ci_batch_test";
    let _ = Command::new("tmux")
        .args(["new-session", "-d", "-s", session, "-n", "batch"])
        .output();
    for msg in ["alpha", "beta"] {
        let _ = Command::new("tmux")
            .args(["split-window", "-t", session])
            .output();
        let _ = Command::new("tmux")
            .args(["send-keys", "-t", session, &format!("echo {msg}"), "Enter"])
            .output();
    }
    std::thread::sleep(std::time::Duration::from_millis(600));

    let client = CliTmuxClient::new();
    let ids: Vec<PaneId> = client
        .fetch_full_tree()
        .unwrap_or_default()
        .iter()
        .find(|s| s.name == session)
        .map(|s| s.windows[0].panes.iter().map(|p| p.id.clone()).collect())
        .unwrap_or_default();

    let individual: Vec<Option<Vec<u8>>> = ids
        .iter()
        .map(|p| client.capture_pane(p, 30, true).ok())
        .collect();
    let batched = client.capture_panes(&ids, 30, true);

    let _ = Command::new("tmux")
        .args(["kill-session", "-t", session])
        .output();

    assert!(
        ids.len() >= 3,
        "expected a multi-pane window, got {}",
        ids.len()
    );
    assert_eq!(batched.len(), ids.len(), "one result per requested pane");
    for (i, (b, s)) in batched.iter().zip(individual.iter()).enumerate() {
        let b = String::from_utf8_lossy(b.as_deref().unwrap_or_default())
            .trim_end()
            .to_string();
        let s = String::from_utf8_lossy(s.as_deref().unwrap_or_default())
            .trim_end()
            .to_string();
        assert_eq!(
            b, s,
            "pane {i} differs between batched and individual capture"
        );
    }
}

/// With a poller attached, `update` must not run tmux queries inline: the whole
/// point is that a slow or wedged server costs staleness, not a frozen UI.
#[test]
fn test_poller_takes_over_refreshing() {
    use lazytmux::event::AppEvent;
    use std::sync::mpsc;
    use std::time::Duration;

    let mut app = make_app();
    let (tx, rx) = mpsc::channel::<AppEvent>();
    // A poller that never answers, standing in for a wedged tmux server.
    app.attach_poller(lazytmux::tmux::poller::spawn(Duration::from_secs(3600), tx));

    let before = app.sessions.len();

    // None of these may block or mutate the tree; the poller owns that now.
    let start = std::time::Instant::now();
    app.update(Action::Tick).unwrap();
    app.update(Action::NavigateDown).unwrap();
    app.update(Action::Refresh).unwrap();
    let elapsed = start.elapsed();

    assert!(
        elapsed < Duration::from_millis(50),
        "update blocked for {elapsed:?} with a poller attached"
    );
    assert_eq!(app.sessions.len(), before);

    // A tree published by the poller is adopted, favourites re-applied.
    let name = app.sessions[0].name.clone();
    app.favorites.toggle(&name);
    let mut fresh = app.sessions.clone();
    fresh.truncate(1);
    fresh[0].is_favorite = false;
    app.apply_tree(fresh);

    assert_eq!(app.sessions.len(), 1);
    assert!(
        app.sessions[0].is_favorite,
        "apply_tree dropped the favourite"
    );
    assert!(
        app.selection.session_idx < app.sessions.len(),
        "selection clamped"
    );
    drop(rx);
}

/// The poller only captures the window that was visible when its pass started.
/// If the selection moved meanwhile, previews already on screen must survive
/// rather than blanking until the next pass.
#[test]
fn test_apply_tree_keeps_previews_the_poller_did_not_capture() {
    let mut app = make_app();
    {
        let pane = &mut app.sessions[0].windows[0].panes[0];
        pane.set_preview(b"existing output\n".to_vec());
    }
    let pane_id = app.sessions[0].windows[0].panes[0].id.clone();

    // A tree with no previews at all, as fetch_full_tree returns it.
    let mut bare = app.sessions.clone();
    for s in bare.iter_mut() {
        for w in s.windows.iter_mut() {
            for p in w.panes.iter_mut() {
                p.preview_raw.clear();
                p.preview_lines.clear();
            }
        }
    }
    app.apply_tree(bare);

    let carried = app
        .sessions
        .iter()
        .flat_map(|s| s.windows.iter())
        .flat_map(|w| w.panes.iter())
        .find(|p| p.id == pane_id)
        .expect("pane still present");
    assert_eq!(
        carried.preview_lines,
        vec!["existing output"],
        "preview was blanked between poller passes"
    );
}

/// The control strip is drawn and hit-tested from one definition, so every
/// button must be clickable exactly where it is painted. This renders a real
/// pane card and checks each control against the actual terminal buffer.
#[test]
fn test_pane_control_hitboxes_match_what_is_drawn() {
    use lazytmux::ui::panes::{PaneControl, control_strip};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::widgets::Block;

    for width in [14u16, 20, 28, 40, 60] {
        let Some(strip) = control_strip(width) else {
            continue;
        };
        let mut terminal = Terminal::new(TestBackend::new(width, 3)).unwrap();
        terminal
            .draw(|f| {
                f.render_widget(
                    Block::bordered().title_bottom(strip.label().to_string()),
                    f.area(),
                );
            })
            .unwrap();
        let buffer = terminal.backend().buffer().clone();
        let bottom: String = (0..width).map(|x| buffer[(x, 2)].symbol()).collect();

        for control in [
            PaneControl::ResizeLeft,
            PaneControl::ResizeDown,
            PaneControl::ResizeUp,
            PaneControl::ResizeRight,
            PaneControl::Swap,
            PaneControl::SplitStacked,
            PaneControl::SplitSideBySide,
            PaneControl::Kill,
        ] {
            // Where does the rendered row actually show this button?
            let Some(byte_idx) = bottom.find(control.label()) else {
                continue;
            };
            let column = bottom[..byte_idx].chars().count() as u16;
            for offset in 0..3u16 {
                assert_eq!(
                    strip.control_at(column + offset),
                    Some(control),
                    "width {width}: column {} of {:?} is painted with {:?} but hit-tests wrong\nrow: {bottom:?}",
                    column + offset,
                    control.label(),
                    control
                );
            }
        }
    }
}

/// Every card wide enough to show anything shows split and close.
#[test]
fn test_split_and_close_buttons_survive_narrow_cards() {
    use lazytmux::ui::panes::{PaneControl, control_strip};

    assert!(control_strip(13).is_none(), "no room for controls at all");
    for width in [14u16, 20, 28, 40] {
        let strip = control_strip(width).expect("controls fit");
        for control in [
            PaneControl::SplitStacked,
            PaneControl::SplitSideBySide,
            PaneControl::Kill,
        ] {
            assert!(
                (0..width).any(|c| strip.control_at(c) == Some(control)),
                "width {width} dropped {control:?}"
            );
        }
    }
}

/// Clicking [-] / [|] splits the pane; clicking [x] asks before killing it.
/// This drives a real mouse event through the same path the terminal uses.
#[test]
fn test_pane_control_clicks_split_and_confirm_close() {
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    use lazytmux::ui::panes::{PaneControl, control_strip};
    use ratatui::layout::Rect;

    // Screen position of a control on the selected pane card, as drawn.
    fn click_at(app: &App, control: PaneControl, area: Rect) -> (u16, u16) {
        let layout =
            lazytmux::ui::AppLayout::split_with_mode(area, app.column_ratios, app.sidebar_mode);
        let inner = ratatui::widgets::Block::default()
            .borders(ratatui::widgets::Borders::ALL)
            .inner(layout.panes_col);
        let window = app.selected_window().expect("a window is selected");
        let pane = app.selected_pane().expect("a pane is selected");
        let root = lazytmux::domain::LayoutNode::parse(&window.layout_str).expect("layout parses");
        let (_, rect) = (inner.y..inner.y + inner.height)
            .flat_map(|y| (inner.x..inner.x + inner.width).map(move |x| (x, y)))
            .find_map(|(x, y)| root.find_pane_rect_at(inner, x, y))
            .filter(|(id, _)| *id == pane.id)
            .expect("selected pane has a rect on screen");
        let strip = control_strip(rect.width).expect("card is wide enough for controls");
        let offset = (0..rect.width)
            .find(|c| strip.control_at(*c) == Some(control))
            .expect("control is on this card");
        (rect.x + offset, rect.y + rect.height - 1)
    }

    let area = Rect::new(0, 0, 200, 50);

    // [x] asks first, and cancelling kills nothing.
    let mut app = make_app();
    app.focus = FocusColumn::Panes;
    app.last_area = area;
    let before = app.selected_window().unwrap().panes.len();
    let (col, row) = click_at(&app, PaneControl::Kill, area);

    let click = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: col,
        row,
        modifiers: crossterm::event::KeyModifiers::NONE,
    };
    // The card is already selected, so its controls are on screen and one
    // click reaches the button.
    if let Some(action) = app.handle_mouse_event(click, area) {
        let mut next = app.update(action).unwrap();
        while let Some(a) = next {
            next = app.update(a).unwrap();
        }
    }

    assert!(
        matches!(
            app.mode,
            Mode::Confirm(lazytmux::app::ConfirmTarget::KillPane(..))
        ),
        "close button must confirm first, mode was {:?}",
        app.mode
    );
    assert_eq!(app.selected_window().unwrap().panes.len(), before);
    app.update(Action::CancelModal).unwrap();
    assert_eq!(
        app.selected_window().unwrap().panes.len(),
        before,
        "cancelling the confirm must not close the pane"
    );
    app.update(Action::ConfirmDestructive).ok();

    // [-] and [|] split immediately, no modal.
    for (control, vertical) in [
        (PaneControl::SplitStacked, false),
        (PaneControl::SplitSideBySide, true),
    ] {
        let mut app = make_app();
        app.focus = FocusColumn::Panes;
        app.last_area = area;
        let before = app.selected_window().unwrap().panes.len();
        assert_eq!(control.action(), Action::SplitPane { vertical });

        let (col, row) = click_at(&app, control, area);
        let click = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: col,
            row,
            modifiers: crossterm::event::KeyModifiers::NONE,
        };
        if let Some(action) = app.handle_mouse_event(click, area) {
            let mut next = app.update(action).unwrap();
            while let Some(a) = next {
                next = app.update(a).unwrap();
            }
        }

        assert_eq!(
            app.mode,
            Mode::Normal,
            "{control:?} should not open a modal"
        );
        assert_eq!(
            app.selected_window().unwrap().panes.len(),
            before + 1,
            "{control:?} did not create a pane"
        );
    }
}

/// A card that is not selected shows no controls, so clicking where a button
/// would be must only select it — never fire an invisible button.
#[test]
fn test_click_on_unselected_card_only_selects() {
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    use ratatui::layout::Rect;

    let mut app = make_app();
    app.focus = FocusColumn::Panes;
    let area = Rect::new(0, 0, 200, 50);
    app.last_area = area;
    app.selection.pane_idx = 1;
    let before = app.selected_window().unwrap().panes.len();

    let layout =
        lazytmux::ui::AppLayout::split_with_mode(area, app.column_ratios, app.sidebar_mode);
    let inner = ratatui::widgets::Block::default()
        .borders(ratatui::widgets::Borders::ALL)
        .inner(layout.panes_col);
    let window = app.selected_window().unwrap();
    let root = lazytmux::domain::LayoutNode::parse(&window.layout_str).unwrap();
    let other = app.selected_window().unwrap().panes[0].id.clone();
    let (_, rect) = (inner.y..inner.y + inner.height)
        .flat_map(|y| (inner.x..inner.x + inner.width).map(move |x| (x, y)))
        .find_map(|(x, y)| root.find_pane_rect_at(inner, x, y))
        .filter(|(id, _)| *id == other)
        .expect("unselected pane has a rect");

    let click = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: rect.x + 2,
        row: rect.y + rect.height - 1,
        modifiers: crossterm::event::KeyModifiers::NONE,
    };
    if let Some(action) = app.handle_mouse_event(click, area) {
        let mut next = app.update(action).unwrap();
        while let Some(a) = next {
            next = app.update(a).unwrap();
        }
    }

    assert_eq!(app.mode, Mode::Normal, "an unseen button fired");
    assert_eq!(app.selected_window().unwrap().panes.len(), before);
}

/// Yes / No must be clickable exactly where they are drawn. This renders the
/// real dialog and checks the hit-test against the terminal buffer.
#[test]
fn test_confirm_button_hitboxes_match_what_is_drawn() {
    use lazytmux::ui::modals::{ConfirmButton, confirm_button_at, confirm_layout};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::layout::Rect;

    for (w, h) in [(60u16, 20u16), (80, 24), (120, 40), (200, 50)] {
        let area = Rect::new(0, 0, w, h);
        let mut app = make_app();
        app.last_area = area;
        app.focus = FocusColumn::Panes;
        app.update(Action::PromptKill).unwrap();
        assert!(matches!(app.mode, Mode::Confirm(_)));

        let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
        terminal.draw(|f| lazytmux::ui::render(&app, f)).unwrap();
        let buffer = terminal.backend().buffer().clone();

        let buttons = confirm_layout(area).buttons;
        assert!(
            buttons.height > 0,
            "{w}x{h}: dialog drawn with no room for its buttons"
        );
        let row = buttons.y;
        let painted: String = (0..w).map(|x| buffer[(x, row)].symbol()).collect();

        for (needle, expected) in [("Yes", ConfirmButton::Yes), ("No", ConfirmButton::No)] {
            let byte_idx = painted
                .find(needle)
                .unwrap_or_else(|| panic!("{needle:?} not drawn at {w}x{h}: {painted:?}"));
            let column = painted[..byte_idx].chars().count() as u16;
            for offset in 0..needle.chars().count() as u16 {
                assert_eq!(
                    confirm_button_at(area, column + offset, row),
                    Some(expected),
                    "{w}x{h}: column {} shows {needle:?} but hit-tests wrong\nrow: {painted:?}",
                    column + offset
                );
            }
        }

        // When a key badge is shown it is part of the same target. Narrow
        // dialogs drop the badge, but never the choice itself.
        if let Some(badge) = painted.find("[y") {
            let badge_col = painted[..badge].chars().count() as u16;
            assert_eq!(
                confirm_button_at(area, badge_col, row),
                Some(ConfirmButton::Yes),
                "{w}x{h}: key badge is not part of the Yes target"
            );
        }
        // Nothing outside the buttons row responds.
        let yes_col = (0..w)
            .find(|c| confirm_button_at(area, *c, row) == Some(ConfirmButton::Yes))
            .expect("Yes is hit-testable");
        assert_eq!(confirm_button_at(area, yes_col, row + 1), None);
    }
}

/// Clicking Yes kills, clicking No does not, and clicking outside cancels.
#[test]
fn test_confirm_dialog_responds_to_clicks() {
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    use lazytmux::ui::modals::{ConfirmButton, confirm_button_at, confirm_layout};
    use ratatui::layout::Rect;

    let area = Rect::new(0, 0, 120, 40);
    let row = confirm_layout(area).buttons.y;
    let column_of = |wanted: ConfirmButton| -> u16 {
        (0..120u16)
            .find(|c| confirm_button_at(area, *c, row) == Some(wanted))
            .expect("button present")
    };

    let click = |app: &mut App, column: u16, row: u16| {
        let event = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column,
            row,
            modifiers: crossterm::event::KeyModifiers::NONE,
        };
        if let Some(action) = app.handle_mouse_event(event, area) {
            let mut next = app.update(action).unwrap();
            while let Some(a) = next {
                next = app.update(a).unwrap();
            }
        }
    };

    // No -> nothing dies.
    let mut app = make_app();
    app.last_area = area;
    app.focus = FocusColumn::Panes;
    let before = app.selected_window().unwrap().panes.len();
    app.update(Action::PromptKill).unwrap();
    click(&mut app, column_of(ConfirmButton::No), row);
    assert_eq!(app.mode, Mode::Normal);
    assert_eq!(app.selected_window().unwrap().panes.len(), before);

    // Outside the dialog -> cancels, still nothing dies.
    app.update(Action::PromptKill).unwrap();
    click(&mut app, 1, 1);
    assert_eq!(app.mode, Mode::Normal);
    assert_eq!(app.selected_window().unwrap().panes.len(), before);

    // Yes -> the pane is killed.
    app.update(Action::PromptKill).unwrap();
    click(&mut app, column_of(ConfirmButton::Yes), row);
    assert_eq!(app.mode, Mode::Normal);
    assert_eq!(app.selected_window().unwrap().panes.len(), before - 1);
}

/// A click while any modal is open must not reach the workspace behind it.
/// Before this, a click could move the selection under the dialog — or hit a
/// pane-card button and retarget the very confirmation being shown.
#[test]
fn test_modal_clicks_do_not_fall_through() {
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    use ratatui::layout::Rect;

    let area = Rect::new(0, 0, 200, 50);

    for open in [
        Action::PromptKill,
        Action::PromptNewSession,
        Action::PromptNewWindow,
        Action::PromptRenameSession,
        Action::PromptSendCommand,
        Action::PromptNewPane,
    ] {
        let mut app = make_app();
        app.last_area = area;
        app.focus = FocusColumn::Panes;
        app.selection.pane_idx = 0;
        app.update(open.clone()).unwrap();
        assert_ne!(app.mode, Mode::Normal, "{open:?} did not open a modal");

        let panes_before = app.selected_window().unwrap().panes.len();
        let session_before = app.selection.session_idx;

        // Bottom-left of the sessions column: selection territory, far from
        // every dialog.
        let event = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 4,
            row: 3,
            modifiers: crossterm::event::KeyModifiers::NONE,
        };
        if let Some(action) = app.handle_mouse_event(event, area) {
            let mut next = app.update(action).unwrap();
            while let Some(a) = next {
                next = app.update(a).unwrap();
            }
        }

        assert_eq!(
            app.selection.session_idx, session_before,
            "{open:?}: click moved the selection behind the modal"
        );
        assert_eq!(
            app.selected_window().unwrap().panes.len(),
            panes_before,
            "{open:?}: click behind the modal changed panes"
        );
    }
}

/// Header buttons must be clickable exactly where they are painted. This
/// renders the real widgets and checks the hit-test against the terminal
/// buffer — the strip's own string is not evidence, because `Theme::block`
/// pads every title and would silently shift the columns.
#[test]
fn test_header_control_hitboxes_match_what_is_drawn() {
    use lazytmux::ui::header::{sessions_header, windows_header};
    use lazytmux::ui::{AppLayout, HeaderControl, SidebarMode};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::layout::Rect;

    for w in [80u16, 100, 150, 200] {
        for mode in [
            SidebarMode::Full,
            SidebarMode::SessionsHidden,
            SidebarMode::WindowsHidden,
        ] {
            let area = Rect::new(0, 0, w, 30);
            let mut app = make_app();
            app.last_area = area;
            app.sidebar_mode = mode;

            let mut terminal = Terminal::new(TestBackend::new(w, 30)).unwrap();
            terminal.draw(|f| lazytmux::ui::render(&app, f)).unwrap();
            let buffer = terminal.backend().buffer().clone();

            let layout = AppLayout::split_with_mode(area, app.column_ratios, mode);
            for (col, strip) in [
                (
                    layout.sessions_col,
                    sessions_header(layout.sessions_col.width, mode),
                ),
                (
                    layout.windows_col,
                    windows_header(layout.windows_col.width, mode),
                ),
            ] {
                if col.width == 0 || strip.title().is_empty() {
                    continue;
                }
                let painted: String = (col.x..col.x + col.width)
                    .map(|x| buffer[(x, col.y)].symbol())
                    .collect();

                for (label, control) in [
                    ("[+]", HeaderControl::New),
                    ("[r]", HeaderControl::Rename),
                    ("[x]", HeaderControl::Kill),
                ] {
                    let byte_idx = painted.find(label).unwrap_or_else(|| {
                        panic!("{w} {mode:?}: {label} not drawn in header {painted:?}")
                    });
                    let column = painted[..byte_idx].chars().count() as u16;
                    for offset in 0..3u16 {
                        assert_eq!(
                            strip.control_at(column + offset),
                            Some(control),
                            "{w} {mode:?}: {label} painted at column {} but hit-tests wrong\nheader: {painted:?}",
                            column + offset
                        );
                    }
                }

                // The header never spills past its column.
                assert!(
                    painted.chars().count() as u16 <= col.width,
                    "{w} {mode:?}: header overflows its column"
                );
            }
        }
    }
}

/// [+] creates, [x] confirms before killing — for both sessions and windows.
#[test]
fn test_header_buttons_create_rename_and_confirm_kill() {
    use lazytmux::ui::header::{sessions_header, windows_header};
    use lazytmux::ui::{AppLayout, HeaderControl};
    use ratatui::layout::Rect;

    let area = Rect::new(0, 0, 120, 40);

    // Sessions [+] opens the new-session prompt.
    let mut app = make_app();
    app.last_area = area;
    let layout = AppLayout::split_with_mode(area, app.column_ratios, app.sidebar_mode);
    let strip = sessions_header(layout.sessions_col.width, app.sidebar_mode);
    let new_col = column_of_header_control(&strip, HeaderControl::New, layout.sessions_col.width);
    app.update(Action::MouseClick {
        column: layout.sessions_col.x + new_col,
        row: layout.sessions_col.y,
        double_click: false,
    })
    .unwrap();
    assert!(matches!(app.mode, Mode::PromptNewSession { .. }));
    app.update(Action::CancelModal).unwrap();

    // Sessions [r] opens the rename prompt, pre-filled with the current name.
    let rename_col =
        column_of_header_control(&strip, HeaderControl::Rename, layout.sessions_col.width);
    let current = app.selected_session().unwrap().name.clone();
    app.update(Action::MouseClick {
        column: layout.sessions_col.x + rename_col,
        row: layout.sessions_col.y,
        double_click: false,
    })
    .unwrap();
    match &app.mode {
        Mode::PromptRenameSession { input, .. } => assert_eq!(*input, current),
        other => panic!("header [r] should rename the session, got {other:?}"),
    }
    app.update(Action::CancelModal).unwrap();

    // Sessions [x] confirms against the selected session, killing nothing yet.
    let before = app.sessions.len();
    let kill_col = column_of_header_control(&strip, HeaderControl::Kill, layout.sessions_col.width);
    app.update(Action::MouseClick {
        column: layout.sessions_col.x + kill_col,
        row: layout.sessions_col.y,
        double_click: false,
    })
    .unwrap();
    assert!(
        matches!(
            app.mode,
            Mode::Confirm(lazytmux::app::ConfirmTarget::KillSession(..))
        ),
        "header [x] must confirm a session kill, got {:?}",
        app.mode
    );
    assert_eq!(app.sessions.len(), before);
    app.update(Action::ConfirmDestructive).unwrap();
    assert_eq!(app.sessions.len(), before - 1);

    // Windows [+] and [x] target windows, not sessions.
    let mut app = make_app();
    app.last_area = area;
    let layout = AppLayout::split_with_mode(area, app.column_ratios, app.sidebar_mode);
    let strip = windows_header(layout.windows_col.width, app.sidebar_mode);
    let new_col = column_of_header_control(&strip, HeaderControl::New, layout.windows_col.width);
    app.update(Action::MouseClick {
        column: layout.windows_col.x + new_col,
        row: layout.windows_col.y,
        double_click: false,
    })
    .unwrap();
    assert!(matches!(app.mode, Mode::PromptNewWindow { .. }));
    app.update(Action::CancelModal).unwrap();

    // Windows [r] renames the window, not the session.
    let rename_col =
        column_of_header_control(&strip, HeaderControl::Rename, layout.windows_col.width);
    let current = app.selected_window().unwrap().name.clone();
    app.update(Action::MouseClick {
        column: layout.windows_col.x + rename_col,
        row: layout.windows_col.y,
        double_click: false,
    })
    .unwrap();
    match &app.mode {
        Mode::PromptRenameWindow { input, .. } => assert_eq!(*input, current),
        other => panic!("header [r] should rename the window, got {other:?}"),
    }
    app.update(Action::CancelModal).unwrap();

    let before = app.selected_session().unwrap().windows.len();
    let kill_col = column_of_header_control(&strip, HeaderControl::Kill, layout.windows_col.width);
    app.update(Action::MouseClick {
        column: layout.windows_col.x + kill_col,
        row: layout.windows_col.y,
        double_click: false,
    })
    .unwrap();
    assert!(
        matches!(
            app.mode,
            Mode::Confirm(lazytmux::app::ConfirmTarget::KillWindow(..))
        ),
        "header [x] must confirm a window kill, got {:?}",
        app.mode
    );
    app.update(Action::ConfirmDestructive).unwrap();
    assert_eq!(app.selected_session().unwrap().windows.len(), before - 1);
}

/// Submit / Cancel and the split choices must be clickable exactly where they
/// are painted. Checked against the rendered terminal buffer, not against the
/// button definitions themselves.
#[test]
fn test_dialog_button_hitboxes_match_what_is_drawn() {
    use lazytmux::ui::modals::{
        PromptButton, SplitButton, prompt_button_at, prompt_layout, split_button_at, split_layout,
    };
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::layout::Rect;

    for (w, h) in [(60u16, 20u16), (80, 24), (120, 40), (200, 50)] {
        let area = Rect::new(0, 0, w, h);

        // Text-entry dialog.
        let mut app = make_app();
        app.last_area = area;
        app.update(Action::PromptNewSession).unwrap();
        let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
        terminal.draw(|f| lazytmux::ui::render(&app, f)).unwrap();
        let buffer = terminal.backend().buffer().clone();

        let row = prompt_layout(area).buttons.y;
        let painted: String = (0..w).map(|x| buffer[(x, row)].symbol()).collect();
        for (needle, expected) in [
            ("Submit", PromptButton::Submit),
            ("Cancel", PromptButton::Cancel),
        ] {
            let idx = painted
                .find(needle)
                .unwrap_or_else(|| panic!("{w}x{h}: {needle} not drawn: {painted:?}"));
            let column = painted[..idx].chars().count() as u16;
            for offset in 0..needle.len() as u16 {
                assert_eq!(
                    prompt_button_at(area, column + offset, row),
                    Some(expected),
                    "{w}x{h}: {needle} painted at {} but hit-tests wrong\nrow: {painted:?}",
                    column + offset
                );
            }
        }

        // New-pane dialog.
        let mut app = make_app();
        app.last_area = area;
        app.focus = FocusColumn::Panes;
        app.update(Action::PromptNewPane).unwrap();
        let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
        terminal.draw(|f| lazytmux::ui::render(&app, f)).unwrap();
        let buffer = terminal.backend().buffer().clone();

        let row = split_layout(area).buttons.y;
        let painted: String = (0..w).map(|x| buffer[(x, row)].symbol()).collect();
        for (needle, expected) in [
            ("[v]", SplitButton::SideBySide),
            ("[h]", SplitButton::Stacked),
            ("[Esc]", SplitButton::Cancel),
        ] {
            let idx = painted
                .find(needle)
                .unwrap_or_else(|| panic!("{w}x{h}: {needle} not drawn: {painted:?}"));
            let column = painted[..idx].chars().count() as u16;
            for offset in 0..needle.len() as u16 {
                assert_eq!(
                    split_button_at(area, column + offset, row),
                    Some(expected),
                    "{w}x{h}: {needle} painted at {} but hit-tests wrong\nrow: {painted:?}",
                    column + offset
                );
            }
        }
    }
}

/// Clicking Submit creates; clicking Cancel does not. Same for the split
/// choices. Driven through real mouse events.
#[test]
fn test_dialog_buttons_respond_to_clicks() {
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    use lazytmux::ui::modals::{
        PromptButton, SplitButton, prompt_button_at, prompt_layout, split_button_at, split_layout,
    };
    use ratatui::layout::Rect;

    let area = Rect::new(0, 0, 120, 40);
    let click = |app: &mut App, column: u16, row: u16| {
        let event = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column,
            row,
            modifiers: crossterm::event::KeyModifiers::NONE,
        };
        if let Some(action) = app.handle_mouse_event(event, area) {
            let mut next = app.update(action).unwrap();
            while let Some(a) = next {
                next = app.update(a).unwrap();
            }
        }
    };

    let prompt_row = prompt_layout(area).buttons.y;
    let prompt_col = |wanted: PromptButton| -> u16 {
        (0..120u16)
            .find(|c| prompt_button_at(area, *c, prompt_row) == Some(wanted))
            .expect("button present")
    };

    // Cancel creates nothing.
    let mut app = make_app();
    app.last_area = area;
    let before = app.sessions.len();
    app.update(Action::PromptNewSession).unwrap();
    for c in "scratch".chars() {
        app.update(Action::ModalInput(c)).unwrap();
    }
    click(&mut app, prompt_col(PromptButton::Cancel), prompt_row);
    assert_eq!(app.mode, Mode::Normal);
    assert_eq!(app.sessions.len(), before, "Cancel created a session");

    // Submit creates the session that was typed.
    app.update(Action::PromptNewSession).unwrap();
    for c in "scratch".chars() {
        app.update(Action::ModalInput(c)).unwrap();
    }
    click(&mut app, prompt_col(PromptButton::Submit), prompt_row);
    assert_eq!(app.mode, Mode::Normal);
    assert_eq!(app.sessions.len(), before + 1, "Submit did not create");
    assert!(app.sessions.iter().any(|s| s.name == "scratch"));

    // Split choices.
    let split_row = split_layout(area).buttons.y;
    let split_col = |wanted: SplitButton| -> u16 {
        (0..120u16)
            .find(|c| split_button_at(area, *c, split_row) == Some(wanted))
            .expect("button present")
    };

    for (button, expected_growth) in [
        (SplitButton::Cancel, 0usize),
        (SplitButton::SideBySide, 1),
        (SplitButton::Stacked, 1),
    ] {
        let mut app = make_app();
        app.last_area = area;
        app.focus = FocusColumn::Panes;
        let before = app.selected_window().unwrap().panes.len();
        app.update(Action::PromptNewPane).unwrap();
        click(&mut app, split_col(button), split_row);
        assert_eq!(app.mode, Mode::Normal, "{button:?} left a modal open");
        assert_eq!(
            app.selected_window().unwrap().panes.len(),
            before + expected_growth,
            "{button:?} produced the wrong pane count"
        );
    }
}
