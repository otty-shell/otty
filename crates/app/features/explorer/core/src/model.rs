use iced::{Point, Size};
use otty_ui_tree::TreePath;

use crate::interaction::{DropTarget, InlineEdit, MenuTarget, PendingDelete};
use crate::state::ExplorerState;
use crate::types::FileNode;

/// Read-only view model for the explorer tree.
#[derive(Debug, Clone)]
pub(crate) struct SidebarTreeViewModel<'a> {
    pub(crate) root_label: Option<&'a str>,
    pub(crate) tree: &'a [FileNode],
    pub(crate) selected_path: Option<&'a TreePath>,
    pub(crate) hovered_path: Option<&'a TreePath>,
    pub(crate) inline_edit: Option<&'a InlineEdit>,
    pub(crate) drop_target: Option<&'a DropTarget>,
}

impl<'a> SidebarTreeViewModel<'a> {
    /// Build the view model from the explorer state.
    pub(crate) fn from_state(state: &'a ExplorerState) -> Self {
        Self {
            root_label: state.root_label(),
            tree: state.tree(),
            selected_path: state.selected_path(),
            hovered_path: state.hovered_path(),
            inline_edit: state.inline_edit(),
            drop_target: state.drop_target(),
        }
    }
}

/// Read-only view model for the open context menu.
#[derive(Debug, Clone)]
pub(crate) struct ContextMenuViewModel<'a> {
    pub(crate) target: &'a MenuTarget,
    pub(crate) cursor: Point,
    pub(crate) area_size: Size,
}

impl<'a> ContextMenuViewModel<'a> {
    /// Build the view model of the open menu, if one is open.
    pub(crate) fn from_state(state: &'a ExplorerState) -> Option<Self> {
        let menu = state.context_menu()?;

        Some(Self {
            target: &menu.target,
            cursor: menu.cursor,
            area_size: menu.area_size,
        })
    }
}

/// Read-only view model for the dialog that asks before a deletion.
#[derive(Debug, Clone)]
pub(crate) struct DeleteDialogViewModel<'a> {
    pub(crate) name: &'a str,
    pub(crate) is_folder: bool,
}

impl<'a> DeleteDialogViewModel<'a> {
    /// Build the view model of the waiting deletion, if one waits.
    pub(crate) fn from_state(state: &'a ExplorerState) -> Option<Self> {
        let delete: &PendingDelete = state.pending_delete()?;

        Some(Self {
            name: &delete.name,
            is_folder: delete.is_folder,
        })
    }
}
