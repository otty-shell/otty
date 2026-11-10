use otty_ui_tree::TreePath;

use crate::state::ExplorerState;
use crate::types::FileNode;

/// Read-only view model for the explorer tree.
#[derive(Debug, Clone)]
pub(crate) struct SidebarTreeViewModel<'a> {
    pub(crate) root_label: Option<&'a str>,
    pub(crate) tree: &'a [FileNode],
    pub(crate) selected_path: Option<&'a TreePath>,
    pub(crate) hovered_path: Option<&'a TreePath>,
}

impl<'a> SidebarTreeViewModel<'a> {
    /// Build the view model from the explorer state.
    pub(crate) fn from_state(state: &'a ExplorerState) -> Self {
        Self {
            root_label: state.root_label(),
            tree: state.tree(),
            selected_path: state.selected_path(),
            hovered_path: state.hovered_path(),
        }
    }
}
