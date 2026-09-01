use lazytmux::action::{Action, ToastLevel};
use lazytmux::app::{App, FocusColumn, Mode};
use lazytmux::config::Config;
use lazytmux::tmux::MockTmuxClient;

#[test]
fn test_live_cli_client() {
    use lazytmux::tmux::{CliTmuxClient, TmuxClient};
    let client = CliTmuxClient::new();
    let tree = client.fetch_full_tree().expect("fetch_full_tree should succeed");
    println!("FETCHED SESSIONS COUNT: {}", tree.len());
    for s in &tree {
        println!("Session: {} (id: {}, attached: {}, windows: {})", s.name, s.id, s.attached, s.windows.len());
        for w in &s.windows {
            println!("  Window: {} (id: {}, active: {}, panes: {})", w.name, w.id, w.active, w.panes.len());
            for p in &w.panes {
                println!("    Pane: {} (cmd: {}, path: {:?})", p.id, p.current_command, p.current_path);
            }
        }
    }
    assert!(!tree.is_empty(), "Live tmux session should be discovered");
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

    let results = app.filtered_search_results("nvim");
    assert!(!results.is_empty());
    assert_eq!(results[0].command, "nvim");

    let results_blog = app.filtered_search_results("blog");
    assert!(!results_blog.is_empty());
    assert_eq!(results_blog[0].window_name, "blog");
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
