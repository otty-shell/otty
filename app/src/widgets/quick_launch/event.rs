use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use iced::Point;

use super::types::{
    ContextMenuAction, QuickLaunchFile, QuickLaunchSetupOutcome,
    QuickLaunchType,
};
use crate::domain::exit_behavior::ExitBehavior;
use crate::domain::quick_launch::{NodePath, QuickLaunch};

/// Intent events handled by the quick launch presentation layer.
#[derive(Debug, Clone)]
pub(crate) enum QuickLaunchIntent {
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
    /// Run a launch (or toggle a folder) as a tree click does, without the
    /// press and release pair. Used by the New session menu.
    LaunchNode {
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
    HeaderCreateFolder,
    HeaderCreateCommand,

    // Tab lifecycle
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
    WizardArgInput {
        tab_id: u64,
        value: String,
    },
    WizardArgSubmit {
        tab_id: u64,
    },
    WizardArgBackspace {
        tab_id: u64,
    },
    WizardRemoveArg {
        tab_id: u64,
        index: usize,
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
    WizardExtraArgInput {
        tab_id: u64,
        value: String,
    },
    WizardExtraArgSubmit {
        tab_id: u64,
    },
    WizardExtraArgBackspace {
        tab_id: u64,
    },
    WizardRemoveExtraArg {
        tab_id: u64,
        index: usize,
    },
    WizardSelectCommandType {
        tab_id: u64,
        command_type: QuickLaunchType,
    },
    WizardSelectExitBehavior {
        tab_id: u64,
        exit_behavior: ExitBehavior,
    },

    // SSH secret dialog events
    SecretPromptChanged(String),
    SecretPromptSubmit,
    SecretPromptCancel,

    // Async completions
    SetupCompleted(QuickLaunchSetupOutcome),
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
    /// Tell the user that a launch or a save of the launches failed;
    /// carries the text in their language.
    Failed(String),
    /// Request closing a tab.
    CloseTabRequested { tab_id: u64 },
    /// Write the launches to disk; the result comes back as an intent.
    PersistRequested { data: QuickLaunchFile },
    /// Prepare a launch in the background; the outcome comes back as an
    /// intent. `cancel` stops the preparation early.
    LaunchSetupRequested {
        command: Box<QuickLaunch>,
        path: NodePath,
        launch_id: u64,
        settings: otty_ui_term::settings::Settings,
        secret: Option<String>,
        cancel: Arc<AtomicBool>,
    },
    /// The last open overlay of the launches closed: the context menu, the
    /// SSH secret prompt or the inline edit. Back in Terminal mode the
    /// active pane gets its keys back.
    OverlayClosed,
}

/// Quick launch event stream routed through the app update loop.
#[derive(Debug, Clone)]
pub(crate) enum QuickLaunchEvent {
    /// Intent event that should be reduced by the quick launch widget.
    Intent(QuickLaunchIntent),
    /// External effect that should be orchestrated by app-level routing.
    Effect(QuickLaunchEffect),
}
