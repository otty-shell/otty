mod errors;
pub(crate) mod event;
pub(crate) mod model;
pub(crate) mod reducer;
pub(crate) mod services;
pub(crate) mod state;
pub(crate) mod types;
pub(crate) mod view;
mod watcher;

pub(crate) use event::{ExplorerEffect, ExplorerEvent, ExplorerIntent};
use iced::{Subscription, Task};
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

    /// Reduce an intent event into state updates and effects.
    pub(crate) fn reduce(
        &mut self,
        event: ExplorerIntent,
        ctx: &ExplorerCtx,
    ) -> Task<ExplorerEvent> {
        reducer::reduce(&mut self.state, event, ctx)
    }

    /// Return active filesystem watcher subscription for loaded directories.
    pub(crate) fn subscription(&self) -> Subscription<ExplorerEvent> {
        watcher::subscription(self.state.watched_directories())
    }

    /// Return a tree view model for the sidebar panel.
    pub(crate) fn vm(&self) -> model::ExplorerTreeViewModel<'_> {
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
    pub(crate) fn tree(&self) -> &[types::FileNode] {
        self.state.tree()
    }

    /// Return the selected tree path.
    pub(crate) fn selected_path(&self) -> Option<&types::TreePath> {
        self.state.selected_path()
    }

    /// Return the hovered tree path.
    pub(crate) fn hovered_path(&self) -> Option<&types::TreePath> {
        self.state.hovered_path()
    }

    /// Return read-only access to state for tests.
    #[cfg(test)]
    pub(crate) fn state(&self) -> &ExplorerState {
        &self.state
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{ExplorerCtx, ExplorerIntent, ExplorerWidget};

    #[test]
    fn given_empty_explorer_when_subscription_requested_then_no_units_are_registered()
     {
        let widget = ExplorerWidget::new();

        assert_eq!(widget.subscription().units(), 0);
    }

    #[test]
    fn given_rooted_explorer_when_subscription_requested_then_watcher_unit_is_registered()
     {
        let mut widget = ExplorerWidget::new();
        let ctx = ExplorerCtx {
            active_shell_cwd: None,
        };

        let _task = widget.reduce(
            ExplorerIntent::SyncRoot {
                cwd: PathBuf::from("/tmp/project"),
            },
            &ctx,
        );

        assert_eq!(widget.subscription().units(), 1);
    }
}
