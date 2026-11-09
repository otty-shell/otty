use otty_containers::{EngineInfo, PullProgress};

use super::super::super::types::{ContainersError, PullHandle};
use super::types::FormField;

/// Intent events of the create dialogs: Pull image, Create volume and
/// Create network.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CreateIntent {
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

/// Effect events of the create dialogs, run by `effects/containers.rs`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CreateEffect {
    /// Pull `image` at `tag` on `engine`, reporting progress.
    StartPull {
        engine: EngineInfo,
        image: String,
        tag: String,
    },
    /// Stop the running pull through its abort handle.
    CancelPull(PullHandle),
}
