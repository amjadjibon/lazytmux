use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use lazytmux::action::Action;
use lazytmux::app::App;
use lazytmux::config::Config;
use lazytmux::event::{AppEvent, EventHandler};
use lazytmux::tmux::{CliTmuxClient, MockTmuxClient, TmuxClient, execute_handoff};
use lazytmux::ui;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::io::{self, stdout};
use std::sync::OnceLock;

/// Previous value of the tmux server option `extended-keys`, recorded when we
/// change it. Read by both the normal exit path and the panic hook, so a crash
/// cannot leave the user's tmux server reconfigured.
static PREVIOUS_EXTENDED_KEYS: OnceLock<String> = OnceLock::new();

fn main() -> anyhow::Result<()> {
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
            unknown => {
                // Silently ignoring this would start against the live tmux
                // server when the user meant to pass --mock.
                eprintln!("Error: unknown argument '{unknown}'.");
                eprintln!("Run 'lazytmux --help' to see available options.");
                std::process::exit(2);
            }
        }
    }

    let config = Config::load_or_default();

    // Check if tmux CLI is available when not in mock mode
    if !is_mock
        && std::process::Command::new("tmux")
            .arg("-V")
            .output()
            .is_err()
    {
        eprintln!("Error: 'tmux' command not found in PATH.");
        eprintln!(
            "Tip: You can test LazyTmux without a live tmux server by running: lazytmux --mock"
        );
        std::process::exit(1);
    }

    // Enable extended-keys so modifiers like Ctrl+Enter reach us. This is a
    // *server*-wide tmux option affecting every session and every other client,
    // so the previous value is restored before we exit.
    let extended_keys = if is_mock {
        None
    } else {
        ExtendedKeys::enable()
    };

    // Terminal initialization
    enable_raw_mode()?;
    let mut stdout = stdout();
    let supports_enhancement =
        crossterm::terminal::supports_keyboard_enhancement().unwrap_or(false);
    if supports_enhancement {
        let _ = execute!(
            stdout,
            crossterm::event::PushKeyboardEnhancementFlags(
                crossterm::event::KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
            )
        );
    }

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

    // Real tmux queries run on their own thread so a slow or wedged server
    // costs staleness rather than a frozen UI. The mock client is in-memory,
    // so mock mode keeps refreshing inline.
    if !is_mock {
        app.attach_poller(lazytmux::tmux::poller::spawn(
            std::time::Duration::from_millis(config.refresh_interval_ms),
            event_handler.sender(),
        ));
    }

    // Main event loop
    while !app.should_quit {
        terminal.draw(|frame| {
            app.last_area = frame.area();
            ui::render(&app, frame);
        })?;

        // A disconnected channel means both producer threads are gone: without
        // this the loop would spin on terminal.draw() at 100% CPU forever.
        let Ok(event) = event_handler.next() else {
            break;
        };
        {
            match event {
                AppEvent::Key(key) => {
                    if let Some(action) = app.handle_key_event(key) {
                        let mut next_action = app.update(action)?;
                        while let Some(act) = next_action {
                            next_action = app.update(act)?;
                        }
                    }
                }
                AppEvent::Mouse(mouse) => {
                    let current_area = app.last_area;
                    if let Some(action) = app.handle_mouse_event(mouse, current_area) {
                        let mut next_action = app.update(action)?;
                        while let Some(act) = next_action {
                            next_action = app.update(act)?;
                        }
                    }
                }
                AppEvent::Tick => {
                    let _ = app.update(Action::Tick);
                }
                AppEvent::Data(tree) => {
                    app.apply_tree(tree);
                }
                AppEvent::Resize(_, _) => {}
            }
        }
    }

    // Clean terminal restoration
    if supports_enhancement {
        let _ = execute!(
            terminal.backend_mut(),
            crossterm::event::PopKeyboardEnhancementFlags
        );
    }
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

    // execute_handoff never returns (it execs or exits), so restore first.
    if let Some(guard) = extended_keys {
        guard.restore();
    }

    // Execute handoff if a pane/session was selected
    if let Some((session_id, session_name, window_id, pane_id)) = app.pending_handoff {
        execute_handoff(&session_id, &session_name, &window_id, &pane_id, is_mock)?;
    }

    Ok(())
}

/// Restores the tmux server's `extended-keys` option to whatever it was before
/// lazytmux changed it.
struct ExtendedKeys {
    previous: String,
}

impl ExtendedKeys {
    /// Turn the option on, remembering the old value. Returns `None` when the
    /// value is already what we need, or when tmux refuses the query, so we
    /// never restore a value we did not actually observe.
    fn enable() -> Option<Self> {
        let output = std::process::Command::new("tmux")
            .args(["show", "-sv", "extended-keys"])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let previous = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if previous == "on" {
            return None;
        }

        let set = std::process::Command::new("tmux")
            .args(["set", "-s", "extended-keys", "on"])
            .output()
            .ok()?;
        if !set.status.success() {
            return None;
        }
        let _ = PREVIOUS_EXTENDED_KEYS.set(previous.clone());
        Some(Self { previous })
    }

    fn restore(&self) {
        restore_extended_keys(&self.previous);
    }
}

fn restore_extended_keys(previous: &str) {
    if previous.is_empty() {
        // No explicit value was recorded: unset ours rather than invent one.
        let _ = std::process::Command::new("tmux")
            .args(["set", "-su", "extended-keys"])
            .output();
    } else {
        let _ = std::process::Command::new("tmux")
            .args(["set", "-s", "extended-keys", previous])
            .output();
    }
}

fn init_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        if let Some(previous) = PREVIOUS_EXTENDED_KEYS.get() {
            restore_extended_keys(previous);
        }
        let _ = execute!(io::stdout(), crossterm::event::PopKeyboardEnhancementFlags);
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
    println!("    h/l, Tab         Switch column focus (l cycles layout in Panes)");
    println!("    j/k, Up/Down     Navigate items");
    println!("    Enter            Attach / Focus selection");
    println!("    /                Global fuzzy search");
    println!("    Space            Fullscreen Inspect Mode with scrollback");
    println!("    n / r / x        Create, Rename, Kill");
    println!("    Ctrl+r / Ctrl+x  Refresh / Respawn pane (asks first)");
    println!("    ?                Help cheatsheet");
    println!("    q, Esc           Quit");
}
