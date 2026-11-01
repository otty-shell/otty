use iced::widget::text_editor::Action;
use otty_docker::{Container, Discovery, EngineAddress, EngineInfo};

use super::inspector::{InspectTarget, InspectorTab, LogTarget};
use super::types::{LoadedList, LoadedLists, Place, ResourceKind};

/// Intent events handled by the Docker widget.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum DockerIntent {
    /// Docker mode became active: search for engines, then load their
    /// lists.
    Enter,
    /// Another mode became active: start no more engine work.
    Leave,
    /// The engine search finished.
    DiscoveryFinished(Discovery),
    /// One load of an engine's four lists finished.
    ListsLoaded {
        engine: EngineAddress,
        lists: LoadedLists,
    },
    /// A tree row was pressed: show this list of this engine.
    Open(Place),
    /// The clear button of the "compose: <name>" chip was pressed: show all
    /// containers again.
    ClearComposeProject,
    /// The row at this position of the shown (filtered) table was pressed.
    SelectRow(usize),
    /// The name filter above the table changed.
    FilterChanged(String),
    /// Load every list of every engine again (the Refresh button).
    Refresh,
    /// Search for engines again (the Check again button).
    CheckAgain,
    /// Copy this command to the clipboard (the Copy button).
    CopyCommand(String),
    /// Open this page in the browser (the Install and How to fix buttons).
    OpenLink(String),
    // Inspector tabs.
    /// An inspector tab was pressed.
    SelectInspectorTab(InspectorTab),
    /// New lines of the followed logs arrived.
    LogLines {
        target: LogTarget,
        lines: Vec<String>,
    },
    /// The followed logs could not be read; the engine's message.
    LogsFailed { target: LogTarget, message: String },
    /// Selection or scroll in the log view.
    LogAction(Action),
    /// The inspect JSON of a row arrived, or the engine's message.
    InspectLoaded {
        target: InspectTarget,
        result: Result<String, String>,
    },
    /// Selection or scroll in the inspect view.
    InspectAction(Action),
    // Live updates from engine events.
    /// An engine event changed this list of this engine (the events watch
    /// sends it at most once per 500 ms per list).
    ListChanged {
        engine: EngineAddress,
        resource: ResourceKind,
    },
    /// The events stream of this engine broke and is open again; changes in
    /// between were missed.
    EngineReconnected(EngineAddress),
    /// This engine stopped answering.
    EngineUnreachable(EngineAddress),
    /// One list of an engine was loaded again.
    ListLoaded {
        engine: EngineAddress,
        list: LoadedList,
    },
}

/// Effect events produced by the Docker reducer. The app runs them off the
/// reducer, because they talk to the engines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DockerEffect {
    /// Search for the local engines (`otty_docker::discover`).
    Discover,
    /// Load the four lists of this engine.
    LoadLists(EngineInfo),
    /// Put this text on the clipboard.
    CopyToClipboard(String),
    /// Open this page in the default browser.
    OpenUrl(String),
    /// Load the inspect JSON of this row from this engine.
    LoadInspect {
        engine: EngineInfo,
        target: InspectTarget,
    },
    /// Load one list of this engine again. "Used by" of images, volumes and
    /// networks is read from `containers`; a container load does not use
    /// it.
    LoadList {
        info: EngineInfo,
        resource: ResourceKind,
        containers: Vec<Container>,
    },
}

/// Docker event stream routed through the app update loop.
#[derive(Debug, Clone)]
pub(crate) enum DockerEvent {
    /// Intent event reduced by the Docker widget.
    Intent(DockerIntent),
    /// External effect orchestrated by app-level routing.
    Effect(DockerEffect),
}
