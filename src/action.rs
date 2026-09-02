use crate::domain::{PaneId, SessionId, WindowId};

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    // Navigation
    NavigateDown,
    NavigateUp,
    NavigateLeft,
    NavigateRight,
    NextColumn,
    PrevColumn,
    OpenSelection,
    FocusSelection,

    // Inspect Mode
    ToggleInspect,
    InspectScrollUp(usize),
    InspectScrollDown(usize),
    InspectScrollTop,
    InspectScrollBottom,
    InspectStartSearch,
    InspectSearchInput(char),
    InspectSearchBackspace,
    InspectSearchSubmit,
    InspectSearchNext,
    InspectSearchPrev,
    InspectSearchCancel,
    CopyPaneOutput,

    // Search Mode
    ToggleSearch,
    SearchInput(char),
    SearchBackspace,
    SearchNext,
    SearchPrev,
    SearchSelect,
    SearchNextCategory,
    SearchPrevCategory,

    // Mutations & Modals
    PromptNewSession,
    PromptNewWindow,
    PromptNewPane,
    SplitPane {
        vertical: bool,
    },
    PromptRenameSession,
    PromptRenameWindow,
    PromptSendCommand,
    BreakPane,
    PromptKill,
    ConfirmKill,
    CancelModal,
    ModalInput(char),
    ModalBackspace,
    ModalSubmit,

    // Layout, Sync, & Pane actions
    NextLayout,
    ToggleSyncPanes,
    SwapPaneUp,
    SwapPaneDown,
    MoveWindowLeft,
    MoveWindowRight,
    RespawnPane,
    ResizePane(crate::tmux::client::ResizeDirection, usize),

    // Theme live switching
    NextTheme,
    PrevTheme,

    // Mouse Actions
    MouseClick {
        column: u16,
        row: u16,
        double_click: bool,
    },
    MouseDrag {
        column: u16,
        row: u16,
    },
    MouseUp,
    MouseScrollUp {
        column: u16,
        row: u16,
    },
    MouseScrollDown {
        column: u16,
        row: u16,
    },

    // Pane / Session actions
    ToggleZoom,
    ToggleFavorite,

    // Meta
    Help,
    Refresh,
    DataRefreshed,
    Tick,
    Quit,
    Handoff {
        session_id: SessionId,
        session_name: String,
        window_id: WindowId,
        pane_id: PaneId,
    },
    ShowToast {
        message: String,
        level: ToastLevel,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastLevel {
    Info,
    Success,
    Warning,
    Error,
}
