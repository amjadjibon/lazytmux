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
    assert!(session.is_favorite);

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

    // Initial state: session 0 ("work") is favorite
    assert!(app.selected_session().unwrap().is_favorite);

    // Toggle favorite
    app.update(Action::ToggleFavorite).unwrap();
    assert!(!app.selected_session().unwrap().is_favorite);

    // Toggle back
    app.update(Action::ToggleFavorite).unwrap();
    assert!(app.selected_session().unwrap().is_favorite);
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

    // Input "echo hello"
    for c in "echo hello".chars() {
        app.update(Action::ModalInput(c)).unwrap();
    }
    app.update(Action::ModalSubmit).unwrap();
    assert_eq!(app.mode, Mode::Normal);
    assert!(app.toasts.last().unwrap().message.contains("echo hello"));
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

#[test]
fn test_mouse_header_buttons_collapse_expand() {
    let mock = Box::new(MockTmuxClient::new());
    let mut app = App::new(mock, Config::default(), true);

    let area = ratatui::layout::Rect::new(0, 0, 100, 30);
    app.last_area = area;

    let layout =
        lazytmux::ui::AppLayout::split_with_mode(area, app.column_ratios, app.sidebar_mode);

    // 1. Click on [◀] in Sessions header (right side of sessions header)
    app.update(Action::MouseClick {
        column: layout.sessions_col.x + layout.sessions_col.width - 2,
        row: layout.sessions_col.y,
        double_click: false,
    })
    .unwrap();
    assert_eq!(app.sidebar_mode, lazytmux::ui::SidebarMode::SessionsHidden);

    // 2. Click on [◀] in Windows header (right side of windows header)
    let layout2 =
        lazytmux::ui::AppLayout::split_with_mode(area, app.column_ratios, app.sidebar_mode);
    app.update(Action::MouseClick {
        column: layout2.windows_col.x + layout2.windows_col.width - 2,
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

    // 4. Click on [◀] in Windows header directly from Full mode (collapses Windows!)
    let layout4 =
        lazytmux::ui::AppLayout::split_with_mode(area, app.column_ratios, app.sidebar_mode);
    app.update(Action::MouseClick {
        column: layout4.windows_col.x + 10, // right on "[◀]" next to "WINDOWS"
        row: layout4.windows_col.y,
        double_click: false,
    })
    .unwrap();
    assert_eq!(app.sidebar_mode, lazytmux::ui::SidebarMode::WindowsHidden);

    // 5. Click on [▶ Windows] in Sessions header (restores Windows!)
    let layout5 =
        lazytmux::ui::AppLayout::split_with_mode(area, app.column_ratios, app.sidebar_mode);
    app.update(Action::MouseClick {
        column: layout5.sessions_col.x + 16,
        row: layout5.sessions_col.y,
        double_click: false,
    })
    .unwrap();
    assert_eq!(app.sidebar_mode, lazytmux::ui::SidebarMode::Full);
}
