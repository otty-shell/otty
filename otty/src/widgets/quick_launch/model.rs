use std::collections::HashMap;

use super::state::InlineEditState;
use super::types::{DropTarget, LaunchInfo, NodePath, QuickLaunchFile};

/// Read-only view model for the quick launch tree.
#[derive(Debug, Clone)]
pub(crate) struct QuickLaunchTreeViewModel<'a> {
    pub(super) data: &'a QuickLaunchFile,
    pub(super) selected_path: Option<&'a NodePath>,
    pub(super) hovered_path: Option<&'a NodePath>,
    pub(super) inline_edit: Option<&'a InlineEditState>,
    pub(super) launching: &'a HashMap<NodePath, LaunchInfo>,
    pub(super) drop_target: Option<&'a DropTarget>,
}
