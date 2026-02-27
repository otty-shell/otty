use iced::Point;

use super::model::{
    ContextMenuAction, NodePath, QuickLaunch, QuickLaunchSetupOutcome,
    QuickLaunchType, QuickLaunchWizardSaveRequest,
};

/// Tick interval for launch indicator animations.
pub(crate) const QUICK_LAUNCHES_TICK_MS: u64 = 200;

/// UI events emitted by the quick launch presentation layer.
#[derive(Debug, Clone)]
pub(crate) enum QuickLaunchEvent {
    // Tree interaction events
    NodeHovered {
        path: NodePath,
    },
    NodePressed {
        path: NodePath,
    },
    NodeReleased {
        path: NodePath,
    },
    NodeRightClicked {
        path: NodePath,
    },
    BackgroundPressed,
    BackgroundReleased,
    BackgroundRightClicked,
    CursorMoved {
        position: Point,
    },
    ResetInteractionState,

    // Context menu events
    ContextMenuDismiss,
    ContextMenuAction(ContextMenuAction),

    // Inline edit events
    InlineEditChanged(String),
    InlineEditSubmit,
    CancelInlineEdit,

    // Header actions
    HeaderAddButtonPressed,
    HeaderCreateFolder,
    HeaderCreateCommand,
    DeleteSelected,

    // Tab lifecycle
    OpenErrorTab {
        tab_id: u64,
        title: String,
        message: String,
    },
    TabClosed {
        tab_id: u64,
    },

    // Wizard events
    WizardInitializeCreate {
        tab_id: u64,
        parent_path: NodePath,
    },
    WizardInitializeEdit {
        tab_id: u64,
        path: NodePath,
        command: Box<QuickLaunch>,
    },
    WizardCancel {
        tab_id: u64,
    },
    WizardSave {
        tab_id: u64,
    },
    WizardSetError {
        tab_id: u64,
        message: String,
    },
    WizardUpdateTitle {
        tab_id: u64,
        value: String,
    },
    WizardUpdateProgram {
        tab_id: u64,
        value: String,
    },
    WizardUpdateHost {
        tab_id: u64,
        value: String,
    },
    WizardUpdateUser {
        tab_id: u64,
        value: String,
    },
    WizardUpdatePort {
        tab_id: u64,
        value: String,
    },
    WizardUpdateIdentityFile {
        tab_id: u64,
        value: String,
    },
    WizardUpdateWorkingDirectory {
        tab_id: u64,
        value: String,
    },
    WizardAddArg {
        tab_id: u64,
    },
    WizardRemoveArg {
        tab_id: u64,
        index: usize,
    },
    WizardUpdateArg {
        tab_id: u64,
        index: usize,
        value: String,
    },
    WizardAddEnv {
        tab_id: u64,
    },
    WizardRemoveEnv {
        tab_id: u64,
        index: usize,
    },
    WizardUpdateEnvKey {
        tab_id: u64,
        index: usize,
        value: String,
    },
    WizardUpdateEnvValue {
        tab_id: u64,
        index: usize,
        value: String,
    },
    WizardAddExtraArg {
        tab_id: u64,
    },
    WizardRemoveExtraArg {
        tab_id: u64,
        index: usize,
    },
    WizardUpdateExtraArg {
        tab_id: u64,
        index: usize,
        value: String,
    },
    WizardSelectCommandType {
        tab_id: u64,
        command_type: QuickLaunchType,
    },

    // Async completions
    SetupCompleted(QuickLaunchSetupOutcome),
    WizardSaveRequested(QuickLaunchWizardSaveRequest),
    PersistCompleted,
    PersistFailed(String),
    Tick,
}

/// Effect events produced by the quick launch reducer.
#[derive(Debug, Clone)]
pub(crate) enum QuickLaunchEffect {
    /// Request opening a wizard tab in create mode.
    OpenWizardCreateTab { parent_path: NodePath },
    /// Request opening a wizard tab in edit mode.
    OpenWizardEditTab {
        path: NodePath,
        command: Box<QuickLaunch>,
    },
    /// Request opening a terminal tab with a prepared command.
    OpenCommandTerminalTab {
        title: String,
        settings: otty_ui_term::settings::Settings,
        command: Box<QuickLaunch>,
    },
    /// Request opening an error tab.
    OpenErrorTab { title: String, message: String },
    /// Request closing a tab.
    CloseTabRequested { tab_id: u64 },
    /// Send an error back to the wizard editor.
    WizardSetError { tab_id: u64, message: String },
    /// Launch setup async completion.
    SetupCompleted(QuickLaunchSetupOutcome),
    /// Persist operation completed successfully.
    PersistCompleted,
    /// Persist operation failed.
    PersistFailed(String),
}
