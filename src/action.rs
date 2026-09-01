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
    CopyPaneOutput,

    // Search Mode
    ToggleSearch,
    SearchInput(char),
    SearchBackspace,
    SearchNext,
    SearchPrev,
    SearchSelect,

    // Mutations & Modals
    PromptNewSession,
    PromptNewWindow,
    PromptNewPane,
    SplitPane { vertical: bool },
    PromptRenameSession,
    PromptRenameWindow,
    PromptKill,
    ConfirmKill,
    CancelModal,
    ModalInput(char),
    ModalBackspace,
    ModalSubmit,

    // Mouse Actions
    MouseClick { column: u16, row: u16, double_click: bool },
    MouseScrollUp { column: u16, row: u16 },
    MouseScrollDown { column: u16, row: u16 },

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
