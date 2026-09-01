use lazytmux::action::Action;
use lazytmux::app::{App, KillTarget, Mode};
use lazytmux::config::Config;
use lazytmux::domain::SessionId;
use lazytmux::tmux::MockTmuxClient;

#[test]
fn test_create_session() {
    let mock = Box::new(MockTmuxClient::new());
    let mut app = App::new(mock, Config::default(), true);

    let initial_count = app.sessions.len();

    // Trigger prompt
    app.update(Action::PromptNewSession).unwrap();
    assert_eq!(
        app.mode,
        Mode::PromptNewSession {
            input: String::new()
        }
    );

    // Type name
    app.update(Action::ModalInput('d')).unwrap();
    app.update(Action::ModalInput('e')).unwrap();
    app.update(Action::ModalInput('v')).unwrap();

    // Submit
    app.update(Action::ModalSubmit).unwrap();
    assert_eq!(app.mode, Mode::Normal);
    assert_eq!(app.sessions.len(), initial_count + 1);
    assert_eq!(app.sessions.last().unwrap().name, "dev");
}

#[test]
fn test_rename_session() {
    let mock = Box::new(MockTmuxClient::new());
    let mut app = App::new(mock, Config::default(), true);

    app.update(Action::PromptRenameSession).unwrap();
    if let Mode::PromptRenameSession { input, .. } = &app.mode {
        assert_eq!(input, "work");
    } else {
        panic!("Expected PromptRenameSession");
    }

    // Clear and set new name
    app.mode = Mode::PromptRenameSession {
        session_id: SessionId::from("$1"),
        input: "company".to_string(),
    };
    app.update(Action::ModalSubmit).unwrap();

    assert_eq!(app.selected_session().unwrap().name, "company");
}

#[test]
fn test_kill_session_with_confirmation() {
    let mock = Box::new(MockTmuxClient::new());
    let mut app = App::new(mock, Config::default(), true);

    let initial_count = app.sessions.len();

    // Trigger prompt kill on first session ("work")
    app.update(Action::PromptKill).unwrap();
    match &app.mode {
        Mode::ConfirmKill(KillTarget::Session(id, name)) => {
            assert_eq!(id.0, "$1");
            assert_eq!(name, "work");
        }
        _ => panic!("Expected ConfirmKill mode"),
    }

    // Cancel modal first to test cancel
    app.update(Action::CancelModal).unwrap();
    assert_eq!(app.mode, Mode::Normal);
    assert_eq!(app.sessions.len(), initial_count);

    // Prompt kill and confirm
    app.update(Action::PromptKill).unwrap();
    app.update(Action::ConfirmKill).unwrap();

    assert_eq!(app.mode, Mode::Normal);
    assert_eq!(app.sessions.len(), initial_count - 1);
    assert_ne!(app.selected_session().unwrap().name, "work");
}

#[test]
fn test_create_window() {
    let mock = Box::new(MockTmuxClient::new());
    let mut app = App::new(mock, Config::default(), true);

    let initial_windows = app.selected_session().unwrap().windows.len();

    // Trigger prompt
    app.update(Action::PromptNewWindow).unwrap();
    if let Mode::PromptNewWindow { input, .. } = &app.mode {
        assert_eq!(input, "");
    } else {
        panic!("Expected PromptNewWindow mode");
    }

    // Type name and submit
    app.update(Action::ModalInput('a')).unwrap();
    app.update(Action::ModalInput('p')).unwrap();
    app.update(Action::ModalInput('i')).unwrap();
    app.update(Action::ModalSubmit).unwrap();

    assert_eq!(app.mode, Mode::Normal);
    assert_eq!(
        app.selected_session().unwrap().windows.len(),
        initial_windows + 1
    );
    assert_eq!(
        app.selected_session().unwrap().windows.last().unwrap().name,
        "api"
    );
}

#[test]
fn test_create_and_split_pane() {
    let mock = Box::new(MockTmuxClient::new());
    let mut app = App::new(mock, Config::default(), true);

    let initial_panes = app.selected_window().unwrap().panes.len();

    // Prompt new pane
    app.update(Action::PromptNewPane).unwrap();
    if let Mode::PromptNewPane { pane_id } = &app.mode {
        assert_eq!(pane_id.0, "%1");
    } else {
        panic!("Expected PromptNewPane mode");
    }

    // Split vertically
    app.update(Action::SplitPane { vertical: true }).unwrap();
    assert_eq!(app.mode, Mode::Normal);
    assert_eq!(
        app.selected_window().unwrap().panes.len(),
        initial_panes + 1
    );
}

#[test]
fn test_rename_window() {
    let mock = Box::new(MockTmuxClient::new());
    let mut app = App::new(mock, Config::default(), true);

    // Focus windows column
    app.focus = lazytmux::app::FocusColumn::Windows;
    app.update(Action::PromptRenameWindow).unwrap();

    if let Mode::PromptRenameWindow { input, .. } = &app.mode {
        assert_eq!(input, "editor");
    } else {
        panic!("Expected PromptRenameWindow mode");
    }

    // Clear and rename to "code"
    app.mode = Mode::PromptRenameWindow {
        window_id: lazytmux::domain::WindowId::from("@1"),
        input: "code".to_string(),
    };
    app.update(Action::ModalSubmit).unwrap();
    assert_eq!(app.selected_window().unwrap().name, "code");
}

#[test]
fn test_kill_window_with_confirmation() {
    let mock = Box::new(MockTmuxClient::new());
    let mut app = App::new(mock, Config::default(), true);

    app.focus = lazytmux::app::FocusColumn::Windows;
    let initial_count = app.selected_session().unwrap().windows.len();

    // Trigger prompt kill
    app.update(Action::PromptKill).unwrap();
    assert!(matches!(
        app.mode,
        Mode::ConfirmKill(KillTarget::Window(..))
    ));

    // Confirm kill
    app.update(Action::ConfirmKill).unwrap();
    assert_eq!(app.mode, Mode::Normal);
    assert_eq!(
        app.selected_session().unwrap().windows.len(),
        initial_count - 1
    );
}

#[test]
fn test_kill_pane_with_confirmation() {
    let mock = Box::new(MockTmuxClient::new());
    let mut app = App::new(mock, Config::default(), true);

    app.focus = lazytmux::app::FocusColumn::Panes;
    let initial_panes = app.selected_window().unwrap().panes.len();

    // Trigger prompt kill
    app.update(Action::PromptKill).unwrap();
    assert!(matches!(app.mode, Mode::ConfirmKill(KillTarget::Pane(..))));

    // Confirm kill
    app.update(Action::ConfirmKill).unwrap();
    assert_eq!(app.mode, Mode::Normal);
    assert_eq!(
        app.selected_window().unwrap().panes.len(),
        initial_panes - 1
    );
}

#[test]
fn test_respawn_pane() {
    let mock = Box::new(MockTmuxClient::new());
    let mut app = App::new(mock, Config::default(), true);
    app.focus = lazytmux::app::FocusColumn::Panes;

    app.update(Action::RespawnPane).unwrap();
    assert!(
        app.toasts
            .last()
            .unwrap()
            .message
            .contains("Respawned pane")
    );
}
