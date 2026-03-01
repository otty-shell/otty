mod errors;
pub(crate) mod event;
pub(crate) mod model;
pub(crate) mod reducer;
pub(crate) mod services;
pub(crate) mod state;
pub(crate) mod view;

pub(crate) use event::{ExplorerEffect, ExplorerEvent, ExplorerUiEvent};
use iced::Task;
pub(crate) use reducer::ExplorerCtx;
use state::ExplorerState;

/// Explorer widget: manages the file system tree browser,
/// lazy folder loading, selection/hover state, and root sync
/// from the active terminal CWD.
pub(crate) struct ExplorerWidget {
    state: ExplorerState,
}

impl ExplorerWidget {
    /// Create a new widget with default (empty) state.
    pub(crate) fn new() -> Self {
        Self {
            state: ExplorerState::default(),
        }
    }

    /// Reduce a UI event into state updates and effects.
    pub(crate) fn reduce(
        &mut self,
        event: ExplorerUiEvent,
        ctx: &ExplorerCtx,
    ) -> Task<ExplorerEvent> {
        reducer::reduce(&mut self.state, event, ctx)
    }

    /// Return a tree view model for the sidebar panel.
    pub(crate) fn tree_vm(&self) -> model::ExplorerTreeViewModel<'_> {
        model::ExplorerTreeViewModel {
            root_label: self.state.root_label(),
            tree: self.state.tree(),
            selected_path: self.state.selected_path(),
            hovered_path: self.state.hovered_path(),
        }
    }

    /// Return the current root label for the explorer header.
    pub(crate) fn root_label(&self) -> Option<&str> {
        self.state.root_label()
    }

    /// Return root tree entries.
    pub(crate) fn tree(&self) -> &[model::FileNode] {
        self.state.tree()
    }

    /// Return the selected tree path.
    pub(crate) fn selected_path(&self) -> Option<&model::TreePath> {
        self.state.selected_path()
    }

    /// Return the hovered tree path.
    pub(crate) fn hovered_path(&self) -> Option<&model::TreePath> {
        self.state.hovered_path()
    }

    /// Return read-only access to state for tests.
    #[cfg(test)]
    pub(crate) fn state(&self) -> &ExplorerState {
        &self.state
    }
}
