use otty_containers::{EngineAddress, EngineInfo, PullProgress};

use super::types::FormField;
use crate::types::{ActionRequest, ContainersError, PullHandle};

/// Intent events of the create dialogs: Pull image, Create volume and
/// Create network.
#[derive(Debug, Clone, PartialEq)]
pub enum CreateIntent {
    /// Open the pull dialog on `engine`, filled with `image` and `tag` (the
    /// default tag when blank).
    OpenPull {
        engine: EngineInfo,
        image: String,
        tag: String,
    },
    /// Open the create volume dialog on this engine.
    OpenVolume(EngineInfo),
    /// Open the create network dialog on this engine.
    OpenNetwork(EngineInfo),
    /// A field of the open dialog changed.
    Changed(FormField, String),
    /// The action button of the open dialog was pressed.
    Submitted,
    /// Cancel of the open dialog was pressed; it stops a running pull.
    Cancelled,
    /// The pull started; the dialog keeps its abort handle while it pulls.
    PullStarted(PullHandle),
    /// The running pull got further.
    PullProgressed(PullProgress),
    /// The pull ended; the error is the engine's message.
    PullFinished(Result<(), ContainersError>),
}

/// What the create dialogs report after an intent: the engine work the
/// binary runs for them and what the parent acts on in the same update.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CreateOutcome {
    /// Pull `image` at `tag` on `engine`, reporting progress.
    StartPull {
        engine: EngineInfo,
        image: String,
        tag: String,
    },
    /// Stop the running pull through its abort handle.
    CancelPull(PullHandle),
    /// A volume or network dialog was sent: run this engine call the way
    /// every action runs, so its list stays busy until the call ends.
    ActionRequested(ActionRequest),
    /// The pull finished: reload the lists of this engine.
    Pulled { engine: EngineAddress },
    /// The pull failed: tell the user; carries the text in their language.
    Failed(String),
}
