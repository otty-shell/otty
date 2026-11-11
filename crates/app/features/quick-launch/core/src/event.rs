use std::fmt;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use iced::Point;
use iced::widget::Id;
use otty_app_domain::exit_behavior::ExitBehavior;
use otty_app_domain::quick_launch::{NodePath, QuickLaunch, WizardTabInit};
use otty_app_domain::tab::TabMachine;
use otty_ui_term::settings::Settings;

use crate::types::{
    ContextMenuAction, QuickLaunchFile, QuickLaunchSetupOutcome,
    QuickLaunchType,
};

/// What quick launch is asked to do: by the Launches section, the launch
/// editor, the SSH secret prompt, the New session menu, the palette, a tick,
/// an I/O answer, or another feature through the binary.
#[derive(Debug, Clone)]
pub enum QuickLaunchIntent {
    /// The pointer entered a row of the tree.
    NodeHovered {
        /// Place of the row in the tree.
        path: NodePath,
    },
    /// A row of the tree was pressed: it is selected and may be dragged.
    NodePressed {
        /// Place of the row in the tree.
        path: NodePath,
    },
    /// A row of the tree was released: a click runs the launch or opens
    /// the folder, a drag drops the row.
    NodeReleased {
        /// Place of the row in the tree.
        path: NodePath,
    },
    /// Run a launch (or toggle a folder) as a tree click does, without the
    /// press and release pair. Used by the New session menu and the palette.
    LaunchNode {
        /// Place of the launch in the tree.
        path: NodePath,
    },
    /// A row of the tree was right-clicked: its context menu opens.
    NodeRightClicked {
        /// Place of the row in the tree.
        path: NodePath,
    },
    /// The empty area of the tree was pressed.
    BackgroundPressed,
    /// The empty area of the tree was released: a drag drops at the top.
    BackgroundReleased,
    /// The empty area of the tree was right-clicked: its menu opens.
    BackgroundRightClicked,
    /// The pointer moved over the tree.
    CursorMoved {
        /// Where the pointer is, in the tree.
        position: Point,
    },
    /// A panel edge is being dragged: hovers, presses and drags stop.
    ResetInteractionState,

    /// Close the context menu.
    ContextMenuDismiss,
    /// An item of the context menu was pressed.
    ContextMenuAction(ContextMenuAction),

    /// The name typed in the tree changed.
    InlineEditChanged(String),
    /// Enter in the name field: create the folder or rename.
    InlineEditSubmit,
    /// Stop naming: Esc, or any other input of the window.
    CancelInlineEdit,

    /// New folder: name a new folder in the selected folder.
    HeaderCreateFolder,
    /// New launch: open the launch editor for a launch in the selected
    /// folder.
    HeaderCreateCommand,

    /// A tab became the active one; the main area shows it.
    TabActivated {
        /// Id of the tab.
        tab_id: u64,
    },
    /// A tab closed: its launch editor, if it had one, goes away.
    TabClosed {
        /// Id of the closed tab.
        tab_id: u64,
        /// Id of the tab that is active now; `None` when no tab is left.
        new_active_id: Option<u64>,
    },

    /// The launch editor opened in tab `tab_id` for a new launch.
    WizardInitializeCreate {
        /// Id of the tab.
        tab_id: u64,
        /// Folder the new launch goes in.
        parent_path: NodePath,
    },
    /// The launch editor opened in tab `tab_id` for a saved launch.
    WizardInitializeEdit {
        /// Id of the tab.
        tab_id: u64,
        /// Place of the launch in the tree.
        path: NodePath,
        /// The launch as it is saved.
        command: Box<QuickLaunch>,
    },
    /// Cancel in the launch editor: its tab closes.
    WizardCancel {
        /// Id of the tab.
        tab_id: u64,
    },
    /// Save in the launch editor.
    WizardSave {
        /// Id of the tab.
        tab_id: u64,
    },
    /// The title field changed.
    WizardUpdateTitle {
        /// Id of the tab.
        tab_id: u64,
        /// The new text.
        value: String,
    },
    /// The program field changed.
    WizardUpdateProgram {
        /// Id of the tab.
        tab_id: u64,
        /// The new text.
        value: String,
    },
    /// The host field changed.
    WizardUpdateHost {
        /// Id of the tab.
        tab_id: u64,
        /// The new text.
        value: String,
    },
    /// The user field changed.
    WizardUpdateUser {
        /// Id of the tab.
        tab_id: u64,
        /// The new text.
        value: String,
    },
    /// The port field changed.
    WizardUpdatePort {
        /// Id of the tab.
        tab_id: u64,
        /// The new text.
        value: String,
    },
    /// The identity file field changed.
    WizardUpdateIdentityFile {
        /// Id of the tab.
        tab_id: u64,
        /// The new text.
        value: String,
    },
    /// The working folder field changed.
    WizardUpdateWorkingDirectory {
        /// Id of the tab.
        tab_id: u64,
        /// The new text.
        value: String,
    },
    /// The text typed for the next argument changed.
    WizardArgInput {
        /// Id of the tab.
        tab_id: u64,
        /// The new text.
        value: String,
    },
    /// Enter in the argument field: the typed text becomes an argument.
    WizardArgSubmit {
        /// Id of the tab.
        tab_id: u64,
    },
    /// Backspace in an empty argument field: the last argument goes.
    WizardArgBackspace {
        /// Id of the tab.
        tab_id: u64,
    },
    /// The cross of an argument.
    WizardRemoveArg {
        /// Id of the tab.
        tab_id: u64,
        /// Position of the argument.
        index: usize,
    },
    /// Add an empty environment variable.
    WizardAddEnv {
        /// Id of the tab.
        tab_id: u64,
    },
    /// The cross of an environment variable.
    WizardRemoveEnv {
        /// Id of the tab.
        tab_id: u64,
        /// Position of the variable.
        index: usize,
    },
    /// The name of an environment variable changed.
    WizardUpdateEnvKey {
        /// Id of the tab.
        tab_id: u64,
        /// Position of the variable.
        index: usize,
        /// The new text.
        value: String,
    },
    /// The value of an environment variable changed.
    WizardUpdateEnvValue {
        /// Id of the tab.
        tab_id: u64,
        /// Position of the variable.
        index: usize,
        /// The new text.
        value: String,
    },
    /// The text typed for the next extra SSH argument changed.
    WizardExtraArgInput {
        /// Id of the tab.
        tab_id: u64,
        /// The new text.
        value: String,
    },
    /// Enter in the extra argument field.
    WizardExtraArgSubmit {
        /// Id of the tab.
        tab_id: u64,
    },
    /// Backspace in an empty extra argument field.
    WizardExtraArgBackspace {
        /// Id of the tab.
        tab_id: u64,
    },
    /// The cross of an extra argument.
    WizardRemoveExtraArg {
        /// Id of the tab.
        tab_id: u64,
        /// Position of the argument.
        index: usize,
    },
    /// A launch type was chosen for a new launch.
    WizardSelectCommandType {
        /// Id of the tab.
        tab_id: u64,
        /// The chosen type.
        command_type: QuickLaunchType,
    },
    /// What the tab does when the launch ends was chosen.
    WizardSelectExitBehavior {
        /// Id of the tab.
        tab_id: u64,
        /// The chosen behavior.
        exit_behavior: ExitBehavior,
    },

    /// The secret typed in the SSH prompt changed.
    SecretPromptChanged(String),
    /// Connect in the SSH prompt: the launch runs again with the secret.
    SecretPromptSubmit,
    /// Cancel in the SSH prompt.
    SecretPromptCancel,

    /// The check before a launch answered.
    SetupCompleted(QuickLaunchSetupOutcome),
    /// The launches were written.
    PersistCompleted,
    /// The launches could not be written; why.
    PersistFailed(String),
    /// A tick of the feature's own timer, which runs while a launch is
    /// being prepared or the tree waits to be saved.
    Tick,
}

/// What quick launch reports back to the frame: news for the tabs and the
/// terminal, I/O to start, the keyboard focus, and a failure to tell.
#[derive(Debug, Clone)]
pub enum QuickLaunchOutcome {
    /// Open the launch editor in a new tab with this title. The tabs hand
    /// `init` back once the tab is open, and the editor fills then.
    OpenEditorTab {
        /// Title of the tab.
        title: String,
        /// What the editor starts with.
        init: WizardTabInit,
    },
    /// Check a launch and build its terminal settings in the background;
    /// the answer comes back as [`QuickLaunchIntent::SetupCompleted`].
    PrepareLaunch(LaunchRequest),
    /// A launch is ready: open a command tab with it on `machine`.
    LaunchReady {
        /// Title of the tab, the launch title.
        title: String,
        /// Settings of the terminal that runs the launch.
        settings: Box<Settings>,
        /// What the tab does when the launch ends.
        exit_behavior: ExitBehavior,
        /// Machine the launch runs on: the SSH host or this computer.
        machine: TabMachine,
    },
    /// Close the tab of a launch editor that was saved or cancelled.
    CloseTab {
        /// Id of the tab.
        tab_id: u64,
    },
    /// Write the launches in the background; the answer comes back as
    /// [`QuickLaunchIntent::PersistCompleted`] or
    /// [`QuickLaunchIntent::PersistFailed`].
    SaveLaunches(QuickLaunchFile),
    /// Give the keyboard to this input: the SSH secret field or the name
    /// field in the tree.
    FocusInput(Id),
    /// The context menu opened: it takes the keyboard from the terminal, so
    /// typed keys do not reach the shell and Esc reaches the frame.
    TakeKeyboard,
    /// The last open overlay of the launches closed: the context menu, the
    /// SSH secret prompt or the name field. Back in Terminal mode the active
    /// pane gets its keys back.
    OverlayClosed,
    /// Tell the user about a failed launch or save; the text is ready to
    /// show.
    Failed(String),
}

/// A launch to check and prepare in the background.
///
/// It has no derived `Debug`: the typed SSH secret must never reach logs.
#[derive(Clone)]
pub struct LaunchRequest {
    command: QuickLaunch,
    path: NodePath,
    launch_id: u64,
    secret: Option<String>,
    cancel: Arc<AtomicBool>,
}

impl LaunchRequest {
    /// The launch to run.
    pub fn command(&self) -> &QuickLaunch {
        &self.command
    }

    /// Place of the launch in the tree; the answer names it.
    pub fn path(&self) -> &[String] {
        &self.path
    }

    /// Id of this run; the answer names it, so a stale answer is dropped.
    pub fn launch_id(&self) -> u64 {
        self.launch_id
    }

    /// The SSH key passphrase or password the user typed, if asked.
    pub fn secret(&self) -> Option<&str> {
        self.secret.as_deref()
    }

    /// Set when the launch is killed: the preparation stops early.
    pub fn cancel(&self) -> &Arc<AtomicBool> {
        &self.cancel
    }

    pub(crate) fn new(
        command: QuickLaunch,
        path: NodePath,
        launch_id: u64,
        secret: Option<String>,
        cancel: Arc<AtomicBool>,
    ) -> Self {
        Self {
            command,
            path,
            launch_id,
            secret,
            cancel,
        }
    }
}

/// Hide the typed secret so it never reaches logs.
impl fmt::Debug for LaunchRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LaunchRequest")
            .field("path", &self.path)
            .field("launch_id", &self.launch_id)
            .field("secret", &self.secret.as_ref().map(|_| "<redacted>"))
            .finish()
    }
}
