# LazyTmux 🦥

[![Rust](https://img.shields.io/badge/rust-2024%20edition-orange.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Ratatui](https://img.shields.io/badge/ratatui-0.30-blueviolet.svg)](https://github.com/ratatui/ratatui)

> **LazyTmux** is a high-performance, keyboard-first visual workspace explorer for [tmux](https://github.com/tmux/tmux), built in Rust with [Ratatui](https://github.com/ratatui/ratatui).
>
> Inspired by the ergonomics of *lazygit* and *lazydocker*, **LazyTmux** replaces arcane tmux session/window key sequences with an interactive, instant 3-column TUI.

---

## ✨ Features

- 🗂️ **3-Column Workspace Explorer**: Seamlessly navigate `Sessions` $\rightarrow$ `Windows` $\rightarrow$ `Panes` with Vim keys (`h/j/k/l` or arrows) and column cycling (`Tab` / `Shift+Tab`).
- 🎨 **Live Syntax-Colored Previews**: Real-time terminal output rendering with complete ANSI color translation powered by `ansi-to-tui`.
- 📐 **Authentic 2D Layout Geometry**: Window pane previews mirror actual tmux split geometries (side-by-side vertical splits and stacked horizontal splits).
- 🔍 **Interactive Fuzzy Finder (`/`)**: Blazing-fast search across sessions, windows, active commands, and working paths powered by `nucleo-matcher`.
- 📜 **Fullscreen Scrollback Inspector (`Space`)**: Zoom into any pane's scrollback history (up to 2,000 lines) with Vim navigation (`Ctrl+d`/`Ctrl+u`, `g`/`G`) and one-key clipboard copy (`c`).
- ⚡ **Workspace Mutations**: Create (`n`), rename (`R`), and kill (`x`) sessions, windows, and panes directly from the TUI with safety confirmation dialogs.
- 🖱️ **Full Mouse & Touchpad Support**: Click to focus/select columns, 2D pane hit testing, scroll wheel navigation, and double-click to attach/switch.
- 🚀 **Smart TTY Handoff & Floating Popup Support**: Automatically detects whether running standalone, inside tmux, or inside a `display-popup`.
- 🧪 **Built-in Mock Mode (`--mock`)**: Test and evaluate the full UI without requiring an active tmux server.

---

## 🚀 Quick Start

### Installation

#### From Source (Recommended)
```bash
git clone https://github.com/amjadjibon/lazytmux.git
cd lazytmux
cargo build --release
cp target/release/lazytmux ~/.local/bin/ # or any directory in your $PATH
```

### Usage

```bash
# Run against live tmux server
lazytmux

# Run in simulated mock mode (no tmux server required)
lazytmux --mock

# View CLI options
lazytmux --help
```

---

## 🪟 tmux.conf Integration

For the ultimate workflow, bind **LazyTmux** to a floating popup in your `~/.tmux.conf`:

```tmux
# Open LazyTmux in a floating modal popup (tmux 3.2+)
bind-key C-w display-popup -E -w 85% -h 85% "lazytmux"

# Quick Session Switcher
bind-key C-s display-popup -E -w 80% -h 80% "lazytmux"
```

*When running inside a popup, selecting any session or pane automatically switches your client and dismisses the popup seamlessly.*

---

## ⌨️ Keybindings

### Global & Navigation

| Key | Action |
| --- | --- |
| `h` / `Left` | Move focus to left column |
| `l` / `Right` | Move focus to right column |
| `j` / `Down` | Move selection down |
| `k` / `Up` | Move selection up |
| `Tab` | Cycle to next column |
| `Shift + Tab` | Cycle to previous column |
| `Enter` | Attach to session / Focus window / Handoff to pane |
| `/` | Open interactive fuzzy finder |
| `z` / `Space` | Toggle fullscreen zoom / inspect mode on selected pane |
| `?` | Open keybinding help overlay |
| `q` / `Ctrl + c` | Quit LazyTmux |

### Workspace Mutations

| Key | Context | Action |
| --- | --- | --- |
| `n` | **Sessions** | Create new session (prompts for name) |
| `n` | **Windows** | Create new window in selected session |
| `n` | **Panes** | Create new split pane (`v` for vertical, `h` for horizontal) |
| `r` / `R` / `F2` | **Sessions / Windows** | Rename active session or window |
| `x` | **Any column** | Kill selected session, window, or pane (with confirmation) |
| `f` | **Sessions** | Toggle session favorite bookmark (`★`) |
| `c` | **Panes / Inspect** | Copy captured pane buffer to system clipboard |

### Inspect / Zoom Mode (`z` / `Space`)

| Key | Action |
| --- | --- |
| `j` / `Down` | Scroll down 1 line |
| `k` / `Up` | Scroll up 1 line |
| `Ctrl + d` | Scroll down 10 lines |
| `Ctrl + u` | Scroll up 10 lines |
| `g` | Jump to top of scrollback |
| `G` | Jump to bottom of scrollback |
| `c` | Copy visible scrollback to clipboard |
| `Esc` / `q` / `z` / `Space` | Exit inspect / zoom mode |

---

## ⚙️ Configuration

LazyTmux looks for an optional configuration file at `~/.config/lazytmux/config.toml` (or `$XDG_CONFIG_HOME/lazytmux/config.toml`):

```toml
# Refresh interval for polling live tmux state (in milliseconds)
refresh_interval_ms = 750

# Maximum lines of scrollback captured for pane preview cards
pane_preview_lines = 30

# Enable safety confirmation modal before killing sessions/windows/panes
confirm_on_kill = true

# Enable mouse clicks, column selection, and scroll wheel support
enable_mouse = true

[theme]
accent_color = "cyan"        # Options: cyan, blue, green, yellow, magenta, red, white
border_style = "rounded"     # Options: rounded, plain, double, thick
```

---

## 🏗️ Architecture

```text
┌────────────────────────────────────────────────────────┐
│                      LazyTmux TUI                      │
│      (Sessions List  │  Windows List  │  2D Panes)     │
└───────────────────────────▲────────────────────────────┘
                            │
┌───────────────────────────┴────────────────────────────┐
│                    App State Machine                   │
│          Reducer • Fuzzy Matcher • Inspect Buffer      │
└───────────────────────────▲────────────────────────────┘
                            │
              ┌─────────────┴─────────────┐
              │                           │
   ┌──────────┴──────────┐     ┌──────────┴──────────┐
   │    CliTmuxClient    │     │    MockTmuxClient   │
   │  (Live Subprocesses)│     │  (In-Memory State)  │
   └─────────────────────┘     └─────────────────────┘
```

- **Domain Layer (`src/domain/`)**: Strongly typed domain models (`SessionId`, `WindowId`, `PaneId`) and recursive-descent tmux `window_layout` AST parser.
- **Tmux Integration (`src/tmux/`)**: Resilient `\t`-delimited parser, subprocess query executor, mock test runner, and zero-flicker TTY handoff.
- **UI & Widgets (`src/ui/`)**: Ratatui 3-column workspace, 2D layout geometry visualizer, theme tokens, and modal dialogs.

---

## 🧪 Testing

Run the full automated test suite (40 tests):

```bash
# Run unit & integration tests
cargo test

# Run linter
cargo clippy --all-targets

# Build release binary
cargo build --release
```

---

## 📄 License

Distributed under the **MIT License**. See [`LICENSE`](LICENSE) for more information.
