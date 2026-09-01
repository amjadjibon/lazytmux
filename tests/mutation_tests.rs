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
    assert_eq!(app.mode, Mode::PromptNewSession { input: String::new() });

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
