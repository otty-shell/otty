use iced::widget::text_editor::Action;
use otty_containers::{ContainerRun, EngineAddress, EngineInfo, LogPosition};

use super::super::links::LinkTarget;
use super::super::types::ContainersError;
use super::types::{InspectTarget, LogTarget};

/// Intent events of the inspector: what the parent sends after its own
/// intents, the followed logs, the inspect answer and the presses in the tab
/// bodies.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum InspectorIntent {
    /// The parent finished an intent: follow the logs of the selected
    /// container and load the inspect JSON of the selected row, as far as the
    /// open tab and the screen allow.
    FollowSelection,
    /// The events stream of this engine broke and is open again, so a start
    /// of the container whose logs are open may have been missed.
    EngineReconnected(EngineAddress),
    /// This container of this engine was started or restarted.
    ContainerStarted { engine: EngineAddress, id: String },
    /// New lines of the followed logs arrived; `position` is where they end,
    /// when they have a time.
    LogLines {
        target: LogTarget,
        lines: Vec<String>,
        position: Option<LogPosition>,
    },
    /// The stream of the followed logs read which run it follows.
    LogRunRead {
        target: LogTarget,
        run: ContainerRun,
    },
    /// What became of the container of the followed logs, asked after its
    /// engine reconnected, or the engine's message.
    LogRunChecked {
        target: LogTarget,
        result: Result<ContainerRun, ContainersError>,
    },
    /// The followed logs could not be read; the engine's message.
    LogsFailed {
        target: LogTarget,
        error: ContainersError,
    },
    /// Selection or scroll in the log view.
    LogAction(Action),
    /// The Follow switch of the logs was turned on or off.
    LogFollowToggled(bool),
    /// The inspect JSON of a row arrived, or the engine's message.
    InspectLoaded {
        target: InspectTarget,
        result: Result<String, ContainersError>,
    },
    /// Selection or scroll in the inspect view.
    InspectAction(Action),
    /// The title of this inspect section was pressed: open or close it.
    InspectSectionToggled(String),
    /// The Show empty switch of the inspect tree was turned on or off.
    InspectShowEmptyToggled(bool),
    /// The Raw JSON switch of the Inspect tab was turned on or off.
    InspectRawJsonToggled(bool),
    /// Copy the inspect JSON to the clipboard (the Copy JSON button).
    CopyInspectJson,
    /// A link in the Overview or in the Inspect tree was pressed: open the
    /// page of a published port, show the named item selected in its list,
    /// or add the `compose` tag of a Compose project.
    LinkPressed(LinkTarget),
}

/// Effect events of the inspector, run by the containers route.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum InspectorEffect {
    /// Load the inspect JSON of this row from this engine.
    LoadInspect {
        engine: EngineInfo,
        target: InspectTarget,
    },
    /// Ask what became of the container of these logs, after its engine
    /// reconnected.
    CheckLogRun(LogTarget),
    /// Put this text on the clipboard.
    CopyToClipboard(String),
}
