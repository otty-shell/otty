use otty_docker::{Discovery, EngineAddress, EngineInfo};

use super::types::{LoadedLists, Place};

/// Intent events handled by the Docker widget.
#[derive(Debug, Clone, PartialEq, Eq)]
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
    /// The row at this position of the shown (filtered) table was pressed.
    SelectRow(usize),
    /// The name filter above the table changed.
    FilterChanged(String),
    /// Load every list of every engine again (the Refresh button).
    Refresh,
}

/// Effect events produced by the Docker reducer. The app runs them off the
/// reducer, because they talk to the engines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DockerEffect {
    /// Search for the local engines (`otty_docker::discover`).
    Discover,
    /// Load the four lists of this engine.
    LoadLists(EngineInfo),
}

/// Docker event stream routed through the app update loop.
#[derive(Debug, Clone)]
pub(crate) enum DockerEvent {
    /// Intent event reduced by the Docker widget.
    Intent(DockerIntent),
    /// External effect orchestrated by app-level routing.
    Effect(DockerEffect),
}
