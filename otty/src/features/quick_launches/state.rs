use std::collections::{HashMap, HashSet};
use std::sync::{Arc, atomic::AtomicBool};
use std::time::Instant;

use iced::Point;

use super::model::{NodePath, QuickLaunchFile};

/// Snapshot of a failed quick launch.
#[derive(Debug, Clone)]
pub(crate) struct QuickLaunchErrorState {
    pub(crate) title: String,
    pub(crate) message: String,
}

/// Target location for quick launch context menus.
#[derive(Debug, Clone)]
pub(crate) enum ContextMenuTarget {
    Command(NodePath),
    Folder(NodePath),
    Background,
}

/// UI state for a visible context menu.
#[derive(Debug, Clone)]
pub(crate) struct ContextMenuState {
    pub(crate) target: ContextMenuTarget,
    pub(crate) cursor: Point,
}

/// Inline edit modes supported in the tree.
#[derive(Debug, Clone)]
pub(crate) enum InlineEditKind {
    CreateFolder { parent_path: NodePath },
    Rename { path: NodePath },
}

/// Inline editing state for a single row.
#[derive(Debug, Clone)]
pub(crate) struct InlineEditState {
    pub(crate) kind: InlineEditKind,
    pub(crate) value: String,
    pub(crate) error: Option<String>,
    pub(crate) id: iced::widget::Id,
}

/// Workspace state for saved quick launches and their UI.
#[derive(Debug)]
pub(crate) struct QuickLaunchState {
    pub(crate) data: QuickLaunchFile,
    pub(crate) dirty: bool,
    persist_in_flight: bool,
    pub(crate) selected: Option<NodePath>,
    pub(crate) hovered: Option<NodePath>,
    pub(crate) launching: HashMap<NodePath, LaunchInfo>,
    pub(crate) canceled_launches: HashSet<u64>,
    pub(crate) next_launch_id: u64,
    pub(crate) blink_nonce: u64,
    pub(crate) context_menu: Option<ContextMenuState>,
    pub(crate) inline_edit: Option<InlineEditState>,
    pub(crate) pressed: Option<NodePath>,
    pub(crate) drag: Option<DragState>,
    pub(crate) drop_target: Option<DropTarget>,
    pub(crate) cursor: Point,
}

/// Runtime info for a pending quick launch.
#[derive(Debug, Clone)]
pub(crate) struct LaunchInfo {
    pub(crate) id: u64,
    pub(crate) started_at: Instant,
    pub(crate) cancel: Arc<AtomicBool>,
}

impl QuickLaunchState {
    /// Construct state from a pre-loaded optional data payload.
    pub(crate) fn from_data(data: Option<QuickLaunchFile>) -> Self {
        match data {
            Some(data) => Self {
                data,
                dirty: false,
                persist_in_flight: false,
                selected: None,
                hovered: None,
                launching: HashMap::new(),
                canceled_launches: HashSet::new(),
                next_launch_id: 1,
                blink_nonce: 0,
                context_menu: None,
                inline_edit: None,
                pressed: None,
                drag: None,
                drop_target: None,
                cursor: Point::ORIGIN,
            },
            None => Self::default(),
        }
    }

    /// Return whether there are unsaved local changes.
    pub(crate) fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Return whether persistence is currently in progress.
    pub(crate) fn is_persist_in_flight(&self) -> bool {
        self.persist_in_flight
    }

    pub(crate) fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub(crate) fn begin_persist(&mut self) {
        self.persist_in_flight = true;
    }

    pub(crate) fn complete_persist(&mut self) {
        self.persist_in_flight = false;
        self.dirty = false;
    }

    pub(crate) fn fail_persist(&mut self) {
        self.persist_in_flight = false;
    }
}

impl Default for QuickLaunchState {
    fn default() -> Self {
        Self {
            data: QuickLaunchFile::empty(),
            dirty: false,
            persist_in_flight: false,
            selected: None,
            hovered: None,
            launching: HashMap::new(),
            canceled_launches: HashSet::new(),
            next_launch_id: 1,
            blink_nonce: 0,
            context_menu: None,
            inline_edit: None,
            pressed: None,
            drag: None,
            drop_target: None,
            cursor: Point::ORIGIN,
        }
    }
}

/// Active drag state for a tree node.
#[derive(Debug, Clone)]
pub(crate) struct DragState {
    pub(crate) source: NodePath,
    pub(crate) origin: Point,
    pub(crate) active: bool,
}

/// Drop target for a drag operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DropTarget {
    Root,
    Folder(NodePath),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_no_data_when_loading_state_then_falls_back_to_default() {
        let state = QuickLaunchState::from_data(None);

        assert!(state.data.root.children.is_empty());
        assert!(!state.dirty);
        assert!(state.launching.is_empty());
    }

    #[test]
    fn given_loaded_payload_when_loading_state_then_uses_loaded_data() {
        let mut payload = QuickLaunchFile::empty();
        payload.root.title = String::from("Loaded");

        let state = QuickLaunchState::from_data(Some(payload));

        assert_eq!(state.data.root.title, "Loaded");
        assert!(state.selected.is_none());
        assert!(state.hovered.is_none());
    }
}
