use iced::Point;

use super::model::{
    ContextMenuAction, NodePath, QuickLaunchSetupOutcome,
    QuickLaunchWizardSaveRequest,
};

/// Tick interval for launch indicators and persistence flush.
pub(crate) const QUICK_LAUNCHES_TICK_MS: u64 = 200;

/// Events emitted by the quick launches sidebar tree.
#[derive(Debug, Clone)]
pub(crate) enum QuickLaunchEvent {
    OpenErrorTab {
        tab_id: u64,
        title: String,
        message: String,
    },
    TabClosed {
        tab_id: u64,
    },
    CursorMoved {
        position: Point,
    },
    NodeHovered {
        path: Option<NodePath>,
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
    BackgroundRightClicked,
    BackgroundPressed,
    BackgroundReleased,
    HeaderCreateFolder,
    HeaderCreateCommand,
    DeleteSelected,
    ContextMenuAction(ContextMenuAction),
    ContextMenuDismiss,
    CancelInlineEdit,
    ResetInteractionState,
    InlineEditChanged(String),
    InlineEditSubmit,
    WizardSaveRequested(QuickLaunchWizardSaveRequest),
    SetupCompleted(QuickLaunchSetupOutcome),
    PersistCompleted,
    PersistFailed(String),
    Tick,
}
