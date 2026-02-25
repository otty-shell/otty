use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use iced::Point;

use super::model::{NodePath, QuickLaunchFile};

/// Snapshot of a failed quick launch.
#[derive(Debug, Clone)]
pub(crate) struct QuickLaunchErrorState {
    title: String,
    message: String,
}

impl QuickLaunchErrorState {
    /// Create a new error state payload.
    pub(crate) fn new(title: String, message: String) -> Self {
        Self { title, message }
    }

    /// Return error title.
    pub(crate) fn title(&self) -> &str {
        &self.title
    }

    /// Return error message.
    pub(crate) fn message(&self) -> &str {
        &self.message
    }
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
    pub(super) target: ContextMenuTarget,
    pub(super) cursor: Point,
}

impl ContextMenuState {
    /// Return context menu target.
    pub(crate) fn target(&self) -> &ContextMenuTarget {
        &self.target
    }

    /// Return cursor position for context menu anchoring.
    pub(crate) fn cursor(&self) -> Point {
        self.cursor
    }
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
    pub(super) kind: InlineEditKind,
    pub(super) value: String,
    pub(super) error: Option<String>,
    pub(super) id: iced::widget::Id,
}

impl InlineEditState {
    /// Return inline edit kind.
    pub(crate) fn kind(&self) -> &InlineEditKind {
        &self.kind
    }

    /// Return current inline edit input value.
    pub(crate) fn value(&self) -> &str {
        &self.value
    }

    /// Return optional inline edit error.
    pub(crate) fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Return input widget identifier.
    pub(crate) fn id(&self) -> &iced::widget::Id {
        &self.id
    }
}

/// Workspace state for saved quick launches and their UI.
#[derive(Debug)]
pub(crate) struct QuickLaunchState {
    pub(super) data: QuickLaunchFile,
    pub(super) dirty: bool,
    persist_in_flight: bool,
    pub(super) selected: Option<NodePath>,
    pub(super) hovered: Option<NodePath>,
    pub(super) launching: HashMap<NodePath, LaunchInfo>,
    pub(super) canceled_launches: HashSet<u64>,
    pub(super) next_launch_id: u64,
    pub(super) blink_nonce: u64,
    pub(super) context_menu: Option<ContextMenuState>,
    pub(super) inline_edit: Option<InlineEditState>,
    pub(super) pressed: Option<NodePath>,
    pub(super) drag: Option<DragState>,
    pub(super) drop_target: Option<DropTarget>,
    pub(super) cursor: Point,
}

/// Runtime info for a pending quick launch.
#[derive(Debug, Clone)]
pub(crate) struct LaunchInfo {
    pub(super) id: u64,
    pub(super) launch_ticks: u64,
    pub(super) is_indicator_highlighted: bool,
    pub(super) cancel: Arc<AtomicBool>,
}

impl LaunchInfo {
    /// Return whether launch indicator is highlighted.
    pub(crate) fn is_indicator_highlighted(&self) -> bool {
        self.is_indicator_highlighted
    }
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

    /// Return immutable quick launch data payload.
    pub(crate) fn data(&self) -> &QuickLaunchFile {
        &self.data
    }

    /// Return whether there are unsaved local changes.
    pub(crate) fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Return whether persistence is currently in progress.
    pub(crate) fn is_persist_in_flight(&self) -> bool {
        self.persist_in_flight
    }

    /// Return whether any launch is currently in progress.
    pub(crate) fn has_active_launches(&self) -> bool {
        !self.launching.is_empty()
    }

    /// Return selected quick launch path.
    pub(crate) fn selected_path(&self) -> Option<&NodePath> {
        self.selected.as_ref()
    }

    /// Return hovered quick launch path.
    pub(crate) fn hovered_path(&self) -> Option<&NodePath> {
        self.hovered.as_ref()
    }

    /// Return in-flight launch map.
    pub(crate) fn launching(&self) -> &HashMap<NodePath, LaunchInfo> {
        &self.launching
    }

    /// Return current context menu state.
    pub(crate) fn context_menu(&self) -> Option<&ContextMenuState> {
        self.context_menu.as_ref()
    }

    /// Return active inline edit state.
    pub(crate) fn inline_edit(&self) -> Option<&InlineEditState> {
        self.inline_edit.as_ref()
    }

    /// Return drop target for drag and drop interactions.
    pub(crate) fn drop_target(&self) -> Option<&DropTarget> {
        self.drop_target.as_ref()
    }

    /// Return drag state.
    pub(crate) fn drag(&self) -> Option<&DragState> {
        self.drag.as_ref()
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
    pub(super) source: NodePath,
    pub(super) origin: Point,
    pub(super) active: bool,
}

impl DragState {
    /// Return whether drag operation is active.
    pub(crate) fn is_active(&self) -> bool {
        self.active
    }
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
