use std::path::PathBuf;

use otty_containers::{
    EngineAddress, EngineInfo, ImageDefaults, PullProgress, RunRequest,
};

use super::types::RunEdit;
use crate::types::{ContainersError, PullHandle};

/// Intent events of the Run container form and of the image panel beside
/// it.
#[derive(Debug, Clone, PartialEq)]
pub enum RunIntent {
    /// Open the form on `engine` with `image` typed (empty from "Run
    /// container"); a typed image is looked up at once.
    Open { engine: EngineInfo, image: String },
    /// Close the form, from Cancel or when another place opens; it stops a
    /// running pull.
    Close,
    /// A field of the form changed.
    Edited(RunEdit),
    /// Another engine was chosen in the form.
    EngineChosen(EngineAddress),
    /// Run was pressed.
    Submitted,
    /// The container was created and started (its id), or the engine's
    /// message.
    Finished {
        engine: EngineAddress,
        result: Result<String, ContainersError>,
    },
    /// What the image `reference` sets on its own (`None`: not on this
    /// computer), or the engine's message.
    ImageDefaultsLoaded {
        engine: EngineAddress,
        reference: String,
        result: Result<Option<ImageDefaults>, ContainersError>,
    },
    /// The disk answered whether this folder of the form exists.
    FolderChecked { path: PathBuf, exists: bool },
    /// Ask the disk again about the folders of the form, so a folder
    /// created elsewhere clears its error by itself.
    RecheckFolders,
    /// Typing a volume source stopped for a moment on this folder: ask the
    /// disk about it if the row still has it and nobody asked yet.
    FolderTypingPaused(PathBuf),
    /// A question about this folder went out a while ago: its row says the
    /// folder is not answering if the answer has not come yet.
    FolderAnswerLate(PathBuf),
    /// The pull of the form's image started; the form keeps its abort
    /// handle while it pulls.
    PullStarted(PullHandle),
    /// The pull of the form's image got further.
    PullProgressed(PullProgress),
    /// The pull of the form's image ended; the error is the engine's
    /// message.
    PullFinished(Result<(), ContainersError>),
}

/// What the Run container form reports after an intent: the engine and disk
/// work the binary runs for it and what the parent acts on in the same
/// update.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RunOutcome {
    /// Pull `image` at `tag` on `engine`, reporting progress.
    StartPull {
        engine: EngineInfo,
        image: String,
        tag: String,
    },
    /// Stop the running pull through its abort handle.
    CancelPull(PullHandle),
    /// Ask `engine` what the image `reference` sets on its own.
    LoadImageDefaults {
        engine: EngineInfo,
        reference: String,
    },
    /// Create and start a container on `engine`.
    RunContainer {
        engine: EngineInfo,
        request: RunRequest,
    },
    /// Ask the disk whether this folder of the form exists.
    CheckFolder(PathBuf),
    /// Wait for a pause in typing, then send `FolderTypingPaused`.
    WaitForTypingPause(PathBuf),
    /// Give the disk time to answer, then send `FolderAnswerLate`.
    WaitForLateAnswer(PathBuf),
    /// The container `id` runs on `engine` and the form closed: show it
    /// selected in the Containers table and reload the engine's lists.
    Started { engine: EngineAddress, id: String },
    /// The pull or the run failed and the form stays as typed: tell the
    /// user; carries the text in their language.
    Failed(String),
}
