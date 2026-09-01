use color_eyre::eyre::eyre;
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use lazytmux::action::Action;
use lazytmux::app::App;
use lazytmux::config::Config;
use lazytmux::event::{AppEvent, EventHandler};
use lazytmux::tmux::{execute_handoff, CliTmuxClient, MockTmuxClient, TmuxClient};
use lazytmux::ui;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::{self, stdout};

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    init_panic_hook();

    let args: Vec<String> = std::env::args().collect();
    let mut is_mock = false;

    for arg in &args[1..] {
        match arg.as_str() {
            "--mock" | "-m" => is_mock = true,
            "--help" | "-h" => {
                print_help();
                return Ok(());
            }
            "--version" | "-v" => {
                println!("lazytmux v{}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            _ => {}
        }
    }

    let config = Config::load_or_default();

    // Check if tmux CLI is available when not in mock mode
    if !is_mock && std::process::Command::new("tmux").arg("-V").output().is_err() {
        eprintln!("Error: 'tmux' command not found in PATH.");
        eprintln!("Tip: You can test LazyTmux without a live tmux server by running: lazytmux --mock");
        std::process::exit(1);
    }

    // Terminal initialization
    enable_raw_mode()?;
    let mut stdout = stdout();
    if config.enable_mouse {
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    } else {
        execute!(stdout, EnterAlternateScreen)?;
    }

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let client: Box<dyn TmuxClient> = if is_mock {
        Box::new(MockTmuxClient::new())
    } else {
        Box::new(CliTmuxClient::new())
    };

    let mut app = App::new(client, config.clone(), is_mock);
    let event_handler = EventHandler::new(config.refresh_interval_ms);

    // Main event loop
    while !app.should_quit {
        terminal.draw(|frame| ui::render(&app, frame))?;

        if let Ok(event) = event_handler.next() {
            match event {
                AppEvent::Key(key) => {
                    if let Some(action) = app.handle_key_event(key) {
                        let mut next_action = app.update(action).map_err(|e| eyre!("{e}"))?;
                        while let Some(act) = next_action {
                            next_action = app.update(act).map_err(|e| eyre!("{e}"))?;
                        }
                    }
                }
                AppEvent::Tick => {
                    let _ = app.update(Action::Tick);
                }
                AppEvent::Resize(_, _) => {}
                _ => {}
            }
        }
    }

    // Clean terminal restoration
    disable_raw_mode()?;
    if config.enable_mouse {
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture,
            crossterm::cursor::Show
        )?;
    } else {
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            crossterm::cursor::Show
        )?;
    }

    // Execute handoff if a pane/session was selected
    if let Some((session_id, session_name, window_id, pane_id)) = app.pending_handoff {
        execute_handoff(&session_id, &session_name, &window_id, &pane_id, is_mock)
            .map_err(|e| eyre!("{e}"))?;
    }

    Ok(())
}

fn init_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(
            io::stdout(),
            LeaveAlternateScreen,
            DisableMouseCapture,
            crossterm::cursor::Show
        );
        original_hook(panic_info);
    }));
}

fn print_help() {
    println!("LazyTmux - Visual Terminal Workspace Explorer for tmux");
    println!();
    println!("USAGE:");
    println!("    lazytmux [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("    -m, --mock       Run with simulated mock sessions (no tmux required)");
    println!("    -h, --help       Print help information");
    println!("    -v, --version    Print version information");
    println!();
    println!("KEYBINDINGS:");
    println!("    h/l, Tab         Switch column focus (Sessions, Windows, Panes)");
    println!("    j/k, Up/Down     Navigate items");
    println!("    Enter            Attach / Focus selection");
    println!("    /                Global fuzzy search");
    println!("    Space            Fullscreen Inspect Mode with scrollback");
    println!("    n / R / x        Create, Rename, Kill");
    println!("    ?                Help cheatsheet");
    println!("    q, Esc           Quit");
}
