# LazyTmux

A modern terminal UI for navigating and managing **tmux sessions, windows, and panes**.

Think **lazygit for tmux**: instead of remembering tmux commands or cycling blindly through sessions, LazyTmux gives you a visual workspace explorer with live pane previews and fast keyboard navigation.

---

## 1. Vision

tmux is powerful, but discovering and switching between existing sessions, windows, and panes is not visual.

LazyTmux makes this hierarchy obvious:

```text
Session
└── Window
    └── Pane
```

The user should be able to:

1. Open `lazytmux` (standalone or via tmux popup `display-popup`).
2. See every tmux session.
3. See every window in the selected session.
4. See every pane in the selected window.
5. Preview what is currently happening inside each pane with live colors.
6. Select any pane.
7. Press `Enter`.
8. Land directly in that pane.

The goal is to make navigating tmux require **2–3 keystrokes instead of remembering commands**.

---

## 2. Core UX

### Main layout

```text
┌─ LazyTmux ─────────────────────────────────────────────────────────────────────────────┐
│ 3 sessions · 8 windows · 17 panes                                      localhost ●    │
├──────────────────┬────────────────────────┬────────────────────────────────────────────┤
│ SESSIONS         │ WINDOWS                │ PANES                                      │
│                  │                        │                                            │
│ ▶ ● work       4 │ ▶ * 1 editor         2 │ ┌─ %1 nvim (active) ─────────────────────┐ │
│     personal   2 │     2 backend        3 │ │ ~/code/api                             │ │
│   ★ infra      2 │     3 logs           2 │ │                                        │ │
│                  │     4 shell          1 │ │ fn main() {                            │ │
│                  │                        │ │     println!("hello");                 │ │
│                  │                        │ │ }                                      │ │
│                  │                        │ └────────────────────────────────────────┘ │
│                  │                        │                                            │
│                  │                        │ ┌─ %2 cargo ─────────────────────────────┐ │
│                  │                        │ │ ~/code/api                             │ │
│                  │                        │ │                                        │ │
│                  │                        │ │ $ cargo test                           │ │
│                  │                        │ │ test result: ok                        │ │
│                  │                        │ └────────────────────────────────────────┘ │
├──────────────────┴────────────────────────┴────────────────────────────────────────────┤
│ work › editor › %1         ~/code/api         nvim                         attached ● │
├─────────────────────────────────────────────────────────────────────────────────────────┤
│ h/l column   j/k move   Enter focus   Tab next   / search   n new   x kill   ? help   │
└─────────────────────────────────────────────────────────────────────────────────────────┘
```

The UI has three primary columns:

```text
Sessions → Windows → Panes
```

The panes view displays actual pane content with syntax highlighting and terminal colors.

---

## 3. Navigation & Keybindings

LazyTmux feels familiar to users of Vim, lazygit, yazi, and other keyboard-first terminal applications.

### Global navigation

| Key         | Action                 | Context / Notes |
| ----------- | ---------------------- | --------------- |
| `j` / `↓`   | Move down              | List navigation |
| `k` / `↑`   | Move up                | List navigation |
| `h` / `←`   | Move one column left   | Column focus    |
| `l` / `→`   | Move one column right  | Column focus    |
| `Tab`       | Next column            | Cycles focus    |
| `Shift+Tab` | Previous column        | Cycles focus    |
| `Enter`     | Open / focus selection | Switches/attaches to target |
| `/`         | Fuzzy search           | Searches sessions, windows, panes |
| `r`         | Force refresh          | Re-queries tmux CLI immediately |
| `?`         | Help overlay           | Opens shortcut cheat sheet |
| `q` / `Esc` | Quit LazyTmux          | Restores terminal cleanly |

### Session actions (`Sessions` column focused)

| Key     | Action            | Details |
| ------- | ----------------- | ------- |
| `Enter` | Attach to session | Handoff to tmux session |
| `n`     | Create session    | Opens modal input prompt |
| `R`     | Rename session    | Opens modal rename prompt |
| `x`     | Kill session      | Triggers confirmation dialog |
| `f`     | Favorite session  | Pins session to top |

### Window actions (`Windows` column focused)

| Key     | Action           | Details |
| ------- | ---------------- | ------- |
| `Enter` | Switch to window | Selects window and focuses active pane |
| `n`     | Create window    | Prompts for window name |
| `R`     | Rename window    | Prompts for new name |
| `x`     | Kill window      | Triggers confirmation dialog |

### Pane actions (`Panes` column focused)

| Key     | Action                    | Details |
| ------- | ------------------------- | ------- |
| `Enter` | Focus pane                | Jumps directly to pane in tmux |
| `Space` | Inspect pane              | Full-screen scrollable preview |
| `z`     | Zoom pane                 | Toggles pane zoom (`resize-pane -Z`) |
| `x`     | Kill pane                 | Triggers confirmation dialog |
| `c`     | Copy captured pane output | Copies visible buffer to system clipboard |

### Inspect Mode (`Space` on a pane)

| Key             | Action                     |
| --------------- | -------------------------- |
| `j` / `k`       | Scroll down / up 1 line    |
| `Ctrl+d` / `Ctrl+u` | Half-page down / up   |
| `g` / `G`       | Jump to top / bottom       |
| `c`             | Copy entire buffer         |
| `/`             | Search within buffer       |
| `Enter`         | Focus this pane in tmux    |
| `Esc` / `Space` | Exit inspect mode          |

---

## 4. Live Pane Previews & ANSI Color Support

Pane previews differentiate LazyTmux from simple command wrappers.

### Fetching Pane Content with ANSI Colors

To capture both text and syntax/ANSI styling without stripping escape codes:

```bash
tmux capture-pane -e -p -t %3 -S -50
```

- `-e`: Preserves ANSI color escape sequences.
- `-p`: Outputs captured buffer to stdout.
- `-S -50`: Captures the last 50 lines of history.

### Rendering ANSI in Ratatui

Raw escape sequences (`\x1b[32m...`) must be translated into Ratatui `Text` / `Span` structs so colors render properly without escaping artifacts.

We use [`ansi-to-tui`](https://crates.io/crates/ansi-to-tui) or [`tui-term`](https://crates.io/crates/tui-term):

```rust
use ansi_to_tui::IntoText;

pub fn format_pane_preview(raw_output: &[u8]) -> ratatui::text::Text<'static> {
    raw_output.into_text().unwrap_or_else(|_| ratatui::text::Text::raw(String::from_utf8_lossy(raw_output)))
}
```

### Lazy Preview Loading & Performance

To prevent spawning dozens of subprocesses per second:
1. **Viewport Caching**: Only capture previews for panes belonging to the *currently selected window* (or visible in the viewport).
2. **Debounced Refresh**: Previews update on a background thread interval (500–1000 ms) or immediately upon selection change.

---

## 5. Mirroring Real Tmux Layouts (2D Topology)

In addition to vertical card lists, LazyTmux can render the authentic 2D layout geometry of the selected window.

### Parsing `window_layout`

tmux exports geometry as an serialized AST string:

```bash
tmux display-message -p -t @2 '#{window_layout}'
```

Example output:
```text
bb62,204x50,0,0{101x50,0,0,1,102x50,102,0[102x24,102,0,2,102x25,102,25,3]}
```

Syntax rules:
- `{...}`: Horizontal split (columns side by side).
- `[...]`: Vertical split (rows stacked on top of each other).
- `width x height, x_off, y_off, pane_id`: Leaf pane node with coordinate geometry.

### AST Translation to Ratatui Layouts

```text
               ┌─────────────────────────┐
               │    Horizontal Split {}  │
               └───────────┬─────────────┘
                           │
             ┌─────────────┴─────────────┐
             ▼                           ▼
   ┌───────────────────┐       ┌───────────────────┐
   │ Leaf: Pane %1     │       │ Vertical Split [] │
   │ (50% width)       │       └─────────┬─────────┘
   └───────────────────┘                 │
                               ┌─────────┴─────────┐
                               ▼                   ▼
                     ┌───────────────────┐ ┌───────────────────┐
                     │ Leaf: Pane %2     │ │ Leaf: Pane %3     │
                     │ (50% w, 50% h)    │ │ (50% w, 50% h)    │
                     └───────────────────┘ └───────────────────┘
```

The layout parser transforms this tree into nested Ratatui `Layout::horizontal` and `Layout::vertical` constraint slices to mirror the actual terminal arrangement.

---

## 6. Pane Inspect Mode

Pressing `Space` on any pane opens a full-screen, high-resolution preview with scrollback navigation.

```text
┌─ Inspect: work › backend › %3 (cargo) ────────────────────────────────────────────────┐
│ ~/code/api                                                                             │
│                                                                                        │
│ $ cargo run                                                                            │
│    Compiling lazytmux v0.1.0 (/Users/.../lazytmux)                                     │
│     Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.42s               │
│      Running `target/debug/lazytmux`                                                   │
│ [2026-09-01T16:50:00Z INFO] Server listening on 127.0.0.1:8080                         │
│ [2026-09-01T16:50:02Z INFO] Connection accepted from 127.0.0.1:54210                   │
│ [2026-09-01T16:50:02Z DEBUG] Route matched: GET /api/v1/sessions                       │
│                                                                                        │
├───────────────────────────────────────────────────────────────────────────── [Line 8/8]┤
│ Esc Back    j/k Scroll    Ctrl+d/u Page    c Copy All    Enter Focus    / Filter       │
└────────────────────────────────────────────────────────────────────────────────────────┘
```

- Captures up to 2,000 lines (`tmux capture-pane -e -p -t %3 -S -2000`).
- Provides Vim-style buffer navigation (`j`, `k`, `Ctrl+d`, `Ctrl+u`, `g`, `G`).
- `c` copies the entire buffer to system clipboard via `arboard` or `pbcopy`/`wl-copy`/`xclip`.

---

## 7. Global Fuzzy Search

Search spans all sessions, windows, panes, current paths, and active commands.

Pressing `/` opens an interactive fuzzy finder powered by [`nucleo-matcher`](https://crates.io/crates/nucleo-matcher) or [`fuzzy-matcher`](https://crates.io/crates/fuzzy-matcher):

```text
┌─ Search: api ─────────────────────────────────────────────────────────────────────────┐
│ > api█                                                                                 │
├────────────────────────────────────────────────────────────────────────────────────────┤
│ SESSION      WINDOW       PANE   COMMAND   PATH                                        │
│ work         backend      %3     ./api     ~/code/api                                  │
│ work         logs         %7     docker    ~/code/infra/docker                         │
│ infra        api-prod     %12    ssh       ~                                           │
└────────────────────────────────────────────────────────────────────────────────────────┘
```

Selecting any result and pressing `Enter` directly navigates to and focuses that pane.

---

## 8. Breadcrumbs & Status Indicators

### Breadcrumbs
The breadcrumb bar sits directly above the footer:

```text
work › backend › %3        cargo        ~/code/api        nvim        attached ●
```

### Visual Indicators

To avoid visual noise, indicators have distinct semantics:

| Symbol | Meaning | Example |
| :--- | :--- | :--- |
| `▶` | **LazyTmux Cursor Focus** (Where your cursor currently is) | `▶ ● work` |
| `●` | **Tmux Attached Session** (Session currently attached to a client) | `● work` |
| `○` | **Tmux Detached Session** (Session running in background) | `○ infra` |
| `*` | **Tmux Active Window/Pane** (The active item inside tmux) | `* 1 editor` |
| `★` | **Favorite Session** (Pinned to top) | `★ work` |
| `!` | **Dead / Exited Process** | `! 4 worker` |

---

## 9. Context-Aware Footer

Shortcuts in the footer automatically adapt to the focused column:

### Sessions Column
```text
Enter Attach   n New Session   R Rename   x Kill   f Favorite   / Search   ? Help
```

### Windows Column
```text
Enter Select   n New Window    R Rename   x Kill   / Search     ? Help
```

### Panes Column
```text
Enter Focus    Space Inspect   z Zoom     x Kill   c Copy       ? Help
```

### Confirm Dialog Mode
```text
y Confirm Kill     n / Esc Cancel
```

---

## 10. Technology Stack

```text
Rust (2024 Edition)
├── ratatui         (Terminal UI layout and rendering)
├── crossterm       (Terminal raw mode, event polling, input handling)
├── ansi-to-tui     (ANSI escape sequence parser for live pane colors)
├── nucleo-matcher  (Fast fuzzy search engine)
├── serde + toml    (Configuration serialization)
├── arboard         (Cross-platform system clipboard integration)
├── color-eyre      (Rich error handling & terminal cleanup panic hooks)
├── thiserror       (Domain-specific typed errors)
└── directories     (Standard XDG / macOS config path resolution)
```

---

## 11. Architecture & Threading Model

To keep the UI running smoothly at 60 FPS without stutters from subprocess executions, LazyTmux uses a decoupled event-driven architecture with a background worker thread.

```text
┌───────────────────────────────────────────────────────────────────────────┐
│                                Main Thread                                │
│                                                                           │
│  ┌──────────────┐         ┌───────────────┐         ┌──────────────────┐  │
│  │  Crossterm   │ ──────► │   App State   │ ──────► │  Ratatui Render  │  │
│  │ Input Events │         │ (Redux / TEA) │         │ (Frame at 60fps) │  │
│  └──────────────┘         └───────▲───────┘         └──────────────────┘  │
│                                   │                                       │
│                           mpsc rx │                                       │
└───────────────────────────────────┼───────────────────────────────────────┘
                                    │
                            mpsc tx │ (Data Refresh / Mutation Results)
┌───────────────────────────────────┼───────────────────────────────────────┐
│  Background Worker Thread         │                                       │
│                                   │                                       │
│                           ┌───────┴───────┐         ┌──────────────────┐  │
│                           │  TmuxClient   │ ──────► │     tmux CLI     │  │
│                           │  (Poller / Tx)│         │   (Subprocess)   │  │
│                           └───────────────┘         └──────────────────┘  │
└───────────────────────────────────────────────────────────────────────────┘
```

Implemented in `src/tmux/poller.rs`. The worker owns its own `CliTmuxClient`
(a unit struct, so there is no shared state and no lock), polls on the configured
interval, and publishes each tree as `AppEvent::Data` into the same mpsc queue
the main loop already drains. The UI sends it a `PreviewContext` naming the
visible panes and the Inspect depth, so only what is on screen is captured, and
a burst of navigation collapses into a single refresh.

Mutations (kill, rename, split, send-keys) still run inline on the main thread:
they are discrete user actions that need their error reported synchronously, and
`SplitPane` reads the refreshed tree to select the pane it just created. Their
follow-up *refresh* is delegated to the poller. Mock mode attaches no poller and
refreshes inline, which keeps tests deterministic.

### Action / Event Pattern
1. **Events**: Key presses, window resizes, and background tick timers.
2. **Actions**: Intentions emitted by event handlers (`Action::FocusPane(PaneId)`, `Action::KillSession(SessionId)`).
3. **App State**: Updates the state immutably in response to actions.
4. **Renderer**: Pure function `render(app: &App, frame: &mut Frame)`.

---

## 12. Project Structure

```text
lazytmux/
├── Cargo.toml
├── README.md
├── LICENSE
└── src/
    ├── main.rs                 # Entry point, terminal initialization, panic hook
    ├── app.rs                  # App state, mode handling, selection transitions
    ├── event.rs                # Crossterm event stream & tick generator
    ├── action.rs               # Action enum definitions
    ├── config.rs               # TOML user configuration
    │
    ├── domain/                 # Core domain models & strongly typed IDs
    │   ├── mod.rs
    │   ├── id.rs               # SessionId, WindowId, PaneId
    │   ├── session.rs          # Session model
    │   ├── window.rs           # Window model
    │   ├── pane.rs             # Pane model & preview buffers
    │   └── layout.rs           # Tmux window_layout AST & parser
    │
    ├── tmux/                   # Tmux integration layer
    │   ├── mod.rs
    │   ├── client.rs           # TmuxClient trait
    │   ├── cli.rs              # Subprocess implementation of TmuxClient
    │   ├── mock.rs             # MockTmuxClient for tests & screenshot generation
    │   ├── parser.rs           # Unit separator (\x1F) formatted string parser
    │   └── handoff.rs          # TTY handoff, exec, switch-client, popup handling
    │
    └── ui/                     # Ratatui rendering components
        ├── mod.rs              # Root renderer & modal dispatch
        ├── layout.rs           # 3-column & 2D layout engine
        ├── sessions.rs         # Sessions list widget
        ├── windows.rs          # Windows list widget
        ├── panes.rs            # Panes list & preview card widgets
        ├── inspect.rs          # Fullscreen inspect modal widget
        ├── search.rs           # Global fuzzy finder widget
        ├── modals.rs           # Confirmation dialogs & input prompts
        ├── footer.rs           # Contextual keybinding footer
        └── theme.rs            # Color palettes & style definitions
```

---

## 13. Domain Models

```rust
use std::path::PathBuf;
use ratatui::text::Text;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionId(pub String); // e.g. "$0", "$1"

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WindowId(pub String);  // e.g. "@1", "@2"

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PaneId(pub String);    // e.g. "%0", "%1"

#[derive(Debug, Clone)]
pub struct Session {
    pub id: SessionId,
    pub name: String,
    pub window_count: usize,
    pub attached: bool,
    pub is_favorite: bool,
    pub windows: Vec<Window>,
}

#[derive(Debug, Clone)]
pub struct Window {
    pub id: WindowId,
    pub session_id: SessionId,
    pub index: u32,
    pub name: String,
    pub active: bool,
    pub layout_str: String,
    pub panes: Vec<Pane>,
}

#[derive(Debug, Clone)]
pub struct Pane {
    pub id: PaneId,
    pub window_id: WindowId,
    pub session_id: SessionId,
    pub index: u32,
    pub active: bool,
    pub current_command: String,
    pub current_path: PathBuf,
    pub width: u16,
    pub height: u16,
    pub preview_lines: Vec<String>,
    pub preview_text: Option<Text<'static>>,
}
```

---

## 14. UI State & State Machine

Explicit states prevent boolean flags from scattering across the codebase:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusColumn {
    Sessions,
    Windows,
    Panes,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Search { query: String, selected_index: usize },
    InspectPane { pane_id: PaneId, scroll_offset: usize },
    PromptNewSession { input: String },
    PromptNewWindow { session_id: SessionId, input: String },
    PromptRenameSession { session_id: SessionId, input: String },
    PromptRenameWindow { window_id: WindowId, input: String },
    ConfirmKill(KillTarget),
    Help,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KillTarget {
    Session(SessionId, String),
    Window(WindowId, String),
    Pane(PaneId, String),
}

#[derive(Debug, Default, Clone)]
pub struct SelectionState {
    pub session_idx: usize,
    pub window_idx: usize,
    pub pane_idx: usize,
}
```

---

## 15. Tmux Client Abstraction

```rust
use anyhow::Result;

pub trait TmuxClient: Send + Sync {
    // Queries
    fn list_sessions(&self) -> Result<Vec<Session>>;
    fn list_windows(&self, session: &SessionId) -> Result<Vec<Window>>;
    fn list_panes(&self, window: &WindowId) -> Result<Vec<Pane>>;
    fn fetch_full_tree(&self) -> Result<Vec<Session>>;
    fn capture_pane(&self, pane: &PaneId, lines: usize, preserve_ansi: bool) -> Result<Vec<u8>>;

    // Mutations
    fn create_session(&self, name: &str) -> Result<SessionId>;
    fn rename_session(&self, session: &SessionId, new_name: &str) -> Result<()>;
    fn kill_session(&self, session: &SessionId) -> Result<()>;

    fn create_window(&self, session: &SessionId, name: &str) -> Result<WindowId>;
    fn rename_window(&self, window: &WindowId, new_name: &str) -> Result<()>;
    fn kill_window(&self, window: &WindowId) -> Result<()>;

    fn kill_pane(&self, pane: &PaneId) -> Result<()>;
    fn zoom_pane(&self, pane: &PaneId) -> Result<()>;

    // Navigation & Focus
    fn focus_pane(&self, session: &SessionId, window: &WindowId, pane: &PaneId) -> Result<()>;
}
```

---

## 16. Reliable Tmux Parsing (Unit Separator Delimited)

Default human-readable tmux output is fragile. To prevent collisions with spaces, pipes, or special characters in session names, window names, or directory paths, format strings use the **ASCII Unit Separator (`\x1F`)**:

### Sessions
```bash
tmux list-sessions -F "#{session_id}\x1F#{session_name}\x1F#{session_windows}\x1F#{session_attached}"
```

### Windows
```bash
tmux list-windows -a -F "#{session_id}\x1F#{window_id}\x1F#{window_index}\x1F#{window_name}\x1F#{window_active}\x1F#{window_panes}\x1F#{window_layout}"
```

### Panes
```bash
tmux list-panes -a -F "#{session_id}\x1F#{window_id}\x1F#{pane_id}\x1F#{pane_index}\x1F#{pane_active}\x1F#{pane_current_command}\x1F#{pane_current_path}\x1F#{pane_width}\x1F#{pane_height}"
```

### Safe Parser Implementation
Splitting on `\x1F` guarantees that characters like `|`, `:`, `/`, or spaces inside paths or names will never break field alignment.

---

## 17. Terminal Lifecycle & Pane Focusing (TTY Handoff)

Jumping directly to a tmux pane requires careful terminal state management:

### Detection of Runtime Context

```rust
pub enum TmuxEnvironment {
    /// Running standalone in a regular terminal outside tmux
    OutsideTmux,
    /// Running inside an existing tmux session/pane
    InsideTmux { current_pane: String },
    /// Running inside a tmux popup window (`tmux display-popup`)
    PopupMode,
}

pub fn detect_environment() -> TmuxEnvironment {
    if std::env::var("TMUX_POPUP").is_ok() {
        TmuxEnvironment::PopupMode
    } else if std::env::var("TMUX").is_ok() {
        TmuxEnvironment::InsideTmux {
            current_pane: std::env::var("TMUX_PANE").unwrap_or_default(),
        }
    } else {
        TmuxEnvironment::OutsideTmux
    }
}
```

### Focusing Behavior by Environment

1. **Inside Popup Mode (`display-popup -E "lazytmux"`)**:
   - Run `tmux select-window -t <window_id>` and `tmux select-pane -t <pane_id>`.
   - Restore terminal and exit LazyTmux process (`exit 0`). The popup closes and the user is instantly focused on the target pane.
2. **Inside Existing Tmux Session**:
   - If switching to another session: `tmux switch-client -t <session_id>`.
   - If switching within current session: `tmux select-window -t <window_id> \; select-pane -t <pane_id>`.
   - Exit or background LazyTmux cleanly.
3. **Outside Tmux (Standalone Terminal)**:
   - Must restore terminal raw mode, disable alternate screen, and show cursor:
     ```rust
     crossterm::terminal::disable_raw_mode()?;
     crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen, crossterm::cursor::Show)?;
     ```
   - On Unix, `exec` replaces the process:
     ```rust
     use std::os::unix::process::CommandExt;
     let err = std::process::Command::new("tmux")
         .args(["attach-session", "-t", &session.name])
         .exec();
     ```

### Panic Hook Safety

To prevent leaving the terminal in a broken raw state on unexpected crashes:

```rust
pub fn init_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(
            std::io::stdout(),
            crossterm::terminal::LeaveAlternateScreen,
            crossterm::cursor::Show
        );
        original_hook(panic_info);
    }));
}
```

---

## 18. Error Handling & Safe Destructive UX

### Toast / Notification System
Non-fatal errors (e.g. failed command, pane closed) appear as temporary toast messages rather than crashing or clearing the UI:

```text
┌────────────────────────────────────────────────────────┐
│ ⚠ Could not kill pane %3: pane is already dead         │
└────────────────────────────────────────────────────────┘
```

```rust
pub struct Toast {
    pub message: String,
    pub level: ToastLevel,
    pub created_at: std::time::Instant,
    pub ttl: std::time::Duration,
}

pub enum ToastLevel {
    Info,
    Success,
    Warning,
    Error,
}
```

### Confirmation Modal for Deletions
Pressing `x` triggers an explicit confirmation modal before sending `kill-*` commands:

```text
┌─ Kill Session ─────────────────────────────────────────┐
│ Are you sure you want to kill session "backend"?       │
│ This will close 3 windows and terminate 7 panes.       │
│                                                        │
│ [y] Confirm Kill               [n/Esc] Cancel          │
└────────────────────────────────────────────────────────┘
```

---

## 19. Empty States

LazyTmux handles empty states cleanly:

### No Tmux Server Running
```text
┌─ LazyTmux ─────────────────────────────────────────────┐
│                                                        │
│                   No Active Tmux Sessions              │
│                                                        │
│           Press 'n' to create your first session       │
│           Press '?' to view help & keybindings         │
│           Press 'q' to quit                            │
│                                                        │
└────────────────────────────────────────────────────────┘
```

### No Windows / Panes Found
```text
Selected session has no active windows.
Press 'n' to create a new window.
```

---

## 20. Version Milestones & Roadmap

### v0.1 (MVP)
- [x] Panic hooks and clean TTY raw mode handoff.
- [x] `TmuxClient` trait with `MockTmuxClient` and `CliTmuxClient`.
- [x] Three-column layout (`Sessions → Windows → Panes`).
- [x] Live pane previews with ANSI escape sequence parsing (`ansi-to-tui`).
- [x] Smooth keyboard navigation (`h/j/k/l`, `Tab`, `Enter`).
- [x] Attach / Focus handoff (inside tmux, popup mode, and standalone terminal).
- [x] Create / rename / kill session, window, and pane (with confirmation dialog).
- [x] Background thread polling to keep UI at 60 FPS.
- [x] Context-aware footer & help overlay modal (`?`).

### v0.2 (Polish & Exploration)
- [ ] Global fuzzy search (`/`) across sessions, windows, panes, and paths (`nucleo-matcher`).
- [ ] Fullscreen Inspect Mode (`Space`) with 2,000 line scrollback buffer and Vim scrolling.
- [ ] Copy preview/inspect buffer to clipboard (`c`).
- [ ] Zoom pane toggle (`z`).
- [ ] Session favorites / pinning (`f`).
- [ ] Configurable keybindings and TOML configuration (`~/.config/lazytmux/config.toml`).

### v0.3 (Layouts & Workspaces)
- [ ] 2D layout parser reproducing tmux `window_layout` geometry in the Panes view.
- [ ] Declarative workspace configuration (e.g. `lazytmux start workspace.toml`).
- [ ] Session snapshots and restore capabilities.

### vFuture (Multi-Host & Remote)
- [ ] Multi-host SSH support (`local`, `devbox`, `prod`).
- [ ] Asynchronous Tokio backend for remote machine discovery.
- [ ] Process metrics (CPU %, memory consumption per pane PID).

---

## 21. Configuration (`~/.config/lazytmux/config.toml`)

```toml
# LazyTmux Configuration
refresh_interval_ms = 750
pane_preview_lines = 30
confirm_on_kill = true
enable_mouse = true

[theme]
accent_color = "cyan"
border_style = "rounded"

[keys]
quit = ["q", "Esc"]
search = "/"
inspect = "Space"
help = "?"
new = "n"
rename = "R"
kill = "x"
zoom = "z"
copy = "c"
favorite = "f"
```

---

## 22. Recommended Development Order

1. **Step 1: Terminal Setup & Panic Hook**:
   Initialize Crossterm alternate screen and register panic hook with `color-eyre` to guarantee terminal restoration.
2. **Step 2: Domain Models & TmuxClient Trait**:
   Define `SessionId`, `WindowId`, `PaneId`, and domain structs. Implement `MockTmuxClient` with realistic test fixture data.
3. **Step 3: Ratatui 3-Column UI & State Machine**:
   Build the 3-column layout (`Sessions`, `Windows`, `Panes`), breadcrumbs, and footer using the mock client.
4. **Step 4: Keyboard Navigation & State Transitions**:
   Implement `FocusColumn` navigation, list indexing, and modal popups.
5. **Step 5: Real `CliTmuxClient` & Delimited Parsing**:
   Implement subprocess execution with ASCII Unit Separator (`\x1F`) parsing.
6. **Step 6: Live Pane Previews with ANSI Colors**:
   Integrate `capture-pane -e -p` and `ansi-to-tui` for live color rendering.
7. **Step 7: TTY Handoff & Focus Actions**:
   Implement clean terminal handoffs for outside tmux, inside tmux, and popup mode.
8. **Step 8: Mutations & Confirmation Dialogs**:
   Implement session/window/pane creation, renaming, and killing with confirmation modals.
9. **Step 9: Background Worker Polling**:
   Connect background polling thread with `mpsc` channel for non-blocking UI updates.
10. **Step 10: Inspect Mode & Fuzzy Search**:
    Add fullscreen scrollback viewer (`Space`) and fuzzy finder (`/`).

---

## 23. Cargo Dependencies

```toml
[package]
name = "lazytmux"
version = "0.1.0"
edition = "2024"

[dependencies]
# UI & Terminal
ratatui = "0.30"
crossterm = "0.29"
ansi-to-tui = "8.0"

# Fuzzy Finding & Search
nucleo-matcher = "0.3"

# Serialization & Config
serde = { version = "1.0", features = ["derive"] }
toml = "1.1"
directories = "6.0"

# Error Handling & Utilities
anyhow = "1.0"
thiserror = "2.0"
color-eyre = "0.6"
arboard = "3.6"
```

---

## 24. Summary Pitch

> **LazyTmux is a keyboard-first TUI for visually navigating tmux sessions, windows, and panes with live previews.**
>
> Browse your entire tmux hierarchy from a single terminal screen, preview active pane output in color, and jump directly to your workspace in 2–3 keystrokes.
