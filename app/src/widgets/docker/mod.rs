pub(crate) mod event;
pub(crate) mod format;
pub(crate) mod model;
pub(crate) mod no_engine;
mod reducer;
pub(crate) mod resource_rows;
mod state;
pub(crate) mod types;
pub(crate) mod view;

pub(crate) use event::{DockerEffect, DockerEvent, DockerIntent};
use iced::Task;
use model::{InspectorViewModel, MainViewModel, TreeRowViewModel};
use state::DockerState;

/// Docker widget: the engines found on this computer, their containers,
/// images, volumes and networks, the open list, the selected rows and the
/// name filter (spec Decisions 5, 8 and 10). It asks for engine work only
/// while Docker mode is active.
pub(crate) struct DockerWidget {
    state: DockerState,
}

impl DockerWidget {
    /// Create the widget before any engine search.
    pub(crate) fn new() -> Self {
        Self {
            state: DockerState::default(),
        }
    }

    /// Reduce an intent into state changes and effect events.
    pub(crate) fn reduce(&mut self, intent: DockerIntent) -> Task<DockerEvent> {
        reducer::reduce(&mut self.state, intent)
    }

    /// Build the rows of the engine tree in the left panel.
    pub(crate) fn tree_vm(&self) -> Vec<TreeRowViewModel> {
        model::tree_rows(&self.state)
    }

    /// Build the main area; ages are counted up to `now` (Unix seconds).
    pub(crate) fn main_vm(&self, now: i64) -> MainViewModel<'_> {
        model::main_area(&self.state, now)
    }

    /// The short text under the engine tree while no engine is usable.
    pub(crate) fn tree_note(&self) -> Option<&'static str> {
        model::panel_note(&self.state)
    }

    /// Build the inspector header of the selected row.
    pub(crate) fn inspector_vm(&self) -> Option<InspectorViewModel> {
        model::inspector(&self.state)
    }
}

#[cfg(test)]
impl DockerWidget {
    /// Return whether Docker mode is the active mode.
    pub(crate) fn is_active(&self) -> bool {
        self.state.is_active()
    }
}
