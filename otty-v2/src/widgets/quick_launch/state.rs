use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use iced::Point;

use super::model::{ContextMenuTarget, LaunchInfo, NodePath, QuickLaunchFile};

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
    data: QuickLaunchFile,
    error_tabs: HashMap<u64, QuickLaunchErrorState>,
    dirty: bool,
    persist_in_flight: bool,
    selected: Option<NodePath>,
    hovered: Option<NodePath>,
    launching: HashMap<NodePath, LaunchInfo>,
    canceled_launches: HashSet<u64>,
    next_launch_id: u64,
    blink_nonce: u64,
    context_menu: Option<ContextMenuState>,
    inline_edit: Option<InlineEditState>,
    pressed: Option<NodePath>,
    drag: Option<DragState>,
    drop_target: Option<DropTarget>,
    cursor: Point,
}

impl QuickLaunchState {
    /// Construct state from a pre-loaded optional data payload.
    pub(crate) fn from_data(data: Option<QuickLaunchFile>) -> Self {
        match data {
            Some(data) => Self {
                data,
                error_tabs: HashMap::new(),
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

    /// Return mutable quick launch data payload for reducer-owned updates.
    pub(crate) fn data_mut(&mut self) -> &mut QuickLaunchFile {
        &mut self.data
    }

    /// Return stored error payload for an error tab.
    pub(crate) fn error_tab(
        &self,
        tab_id: u64,
    ) -> Option<&QuickLaunchErrorState> {
        self.error_tabs.get(&tab_id)
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

    /// Return selected path by value.
    pub(crate) fn selected_path_cloned(&self) -> Option<NodePath> {
        self.selected.clone()
    }

    /// Update selected quick launch path.
    pub(crate) fn set_selected_path(&mut self, path: Option<NodePath>) {
        self.selected = path;
    }

    /// Clear selected quick launch path.
    pub(crate) fn clear_selected_path(&mut self) {
        self.selected = None;
    }

    /// Return hovered quick launch path.
    pub(crate) fn hovered_path(&self) -> Option<&NodePath> {
        self.hovered.as_ref()
    }

    /// Update hovered quick launch path.
    pub(crate) fn set_hovered_path(&mut self, path: Option<NodePath>) {
        self.hovered = path;
    }

    /// Return cursor position used by quick launches interactions.
    pub(crate) fn cursor(&self) -> Point {
        self.cursor
    }

    /// Update interaction cursor position.
    pub(crate) fn set_cursor(&mut self, cursor: Point) {
        self.cursor = cursor;
    }

    /// Return in-flight launch map.
    pub(crate) fn launching(&self) -> &HashMap<NodePath, LaunchInfo> {
        &self.launching
    }

    /// Return mutable in-flight launch map for reducer-owned updates.
    pub(super) fn launching_mut(
        &mut self,
    ) -> &mut HashMap<NodePath, LaunchInfo> {
        &mut self.launching
    }

    /// Return launch info for a path.
    pub(crate) fn launch_info(&self, path: &[String]) -> Option<&LaunchInfo> {
        self.launching.get(path)
    }

    /// Return whether a path is currently launching.
    pub(crate) fn is_launching(&self, path: &[String]) -> bool {
        self.launching.contains_key(path)
    }

    /// Start launch tracking and allocate the next launch identifier.
    pub(super) fn begin_launch(
        &mut self,
        path: NodePath,
        cancel: Arc<AtomicBool>,
    ) -> u64 {
        let launch_id = self.next_launch_id;
        self.next_launch_id = self.next_launch_id.wrapping_add(1);
        self.launching.insert(
            path,
            LaunchInfo {
                id: launch_id,
                launch_ticks: 0,
                is_indicator_highlighted: true,
                cancel,
            },
        );
        launch_id
    }

    /// Remove launch tracking by path.
    pub(super) fn remove_launch(
        &mut self,
        path: &[String],
    ) -> Option<LaunchInfo> {
        self.launching.remove(path)
    }

    /// Mark launch as canceled by path, if present.
    pub(super) fn cancel_launch(&mut self, path: &[String]) {
        if let Some(info) = self.launching.get(path) {
            info.cancel.store(true, Ordering::Relaxed);
            self.canceled_launches.insert(info.id);
        }
    }

    /// Consume canceled mark for launch id.
    pub(super) fn take_canceled_launch(&mut self, launch_id: u64) -> bool {
        self.canceled_launches.remove(&launch_id)
    }

    /// Return current launch indicator nonce.
    pub(crate) fn blink_nonce(&self) -> u64 {
        self.blink_nonce
    }

    /// Increment launch indicator blink nonce.
    pub(super) fn advance_blink_nonce(&mut self) {
        self.blink_nonce = self.blink_nonce.wrapping_add(1);
    }

    /// Return current context menu state.
    pub(crate) fn context_menu(&self) -> Option<&ContextMenuState> {
        self.context_menu.as_ref()
    }

    /// Return cloned context menu state.
    pub(crate) fn context_menu_cloned(&self) -> Option<ContextMenuState> {
        self.context_menu.clone()
    }

    /// Set current context menu state.
    pub(super) fn set_context_menu(&mut self, menu: Option<ContextMenuState>) {
        self.context_menu = menu;
    }

    /// Clear current context menu state.
    pub(super) fn clear_context_menu(&mut self) {
        self.context_menu = None;
    }

    /// Return active inline edit state.
    pub(crate) fn inline_edit(&self) -> Option<&InlineEditState> {
        self.inline_edit.as_ref()
    }

    /// Return mutable inline edit state.
    pub(super) fn inline_edit_mut(&mut self) -> Option<&mut InlineEditState> {
        self.inline_edit.as_mut()
    }

    /// Set inline edit state.
    pub(super) fn set_inline_edit(&mut self, edit: Option<InlineEditState>) {
        self.inline_edit = edit;
    }

    /// Clear inline edit state.
    pub(super) fn clear_inline_edit(&mut self) {
        self.inline_edit = None;
    }

    /// Take and clear inline edit state.
    pub(super) fn take_inline_edit(&mut self) -> Option<InlineEditState> {
        self.inline_edit.take()
    }

    /// Return currently pressed path.
    pub(crate) fn pressed_path(&self) -> Option<&NodePath> {
        self.pressed.as_ref()
    }

    /// Set currently pressed path.
    pub(super) fn set_pressed_path(&mut self, path: Option<NodePath>) {
        self.pressed = path;
    }

    /// Clear currently pressed path.
    pub(super) fn clear_pressed_path(&mut self) {
        self.pressed = None;
    }

    /// Return drop target for drag and drop interactions.
    pub(crate) fn drop_target(&self) -> Option<&DropTarget> {
        self.drop_target.as_ref()
    }

    /// Set drop target for drag and drop interactions.
    pub(super) fn set_drop_target(&mut self, target: Option<DropTarget>) {
        self.drop_target = target;
    }

    /// Clear drop target for drag and drop interactions.
    pub(super) fn clear_drop_target(&mut self) {
        self.drop_target = None;
    }

    /// Take and clear drop target.
    pub(super) fn take_drop_target(&mut self) -> Option<DropTarget> {
        self.drop_target.take()
    }

    /// Return drag state.
    pub(crate) fn drag(&self) -> Option<&DragState> {
        self.drag.as_ref()
    }

    /// Return mutable drag state.
    pub(super) fn drag_mut(&mut self) -> Option<&mut DragState> {
        self.drag.as_mut()
    }

    /// Set drag state.
    pub(super) fn set_drag(&mut self, drag: Option<DragState>) {
        self.drag = drag;
    }

    /// Clear drag state.
    pub(super) fn clear_drag(&mut self) {
        self.drag = None;
    }

    /// Take and clear drag state.
    pub(super) fn take_drag(&mut self) -> Option<DragState> {
        self.drag.take()
    }

    pub(super) fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub(super) fn begin_persist(&mut self) {
        self.persist_in_flight = true;
    }

    pub(super) fn complete_persist(&mut self) {
        self.persist_in_flight = false;
        self.dirty = false;
    }

    pub(super) fn fail_persist(&mut self) {
        self.persist_in_flight = false;
    }

    /// Store or replace an error tab payload.
    pub(super) fn set_error_tab(
        &mut self,
        tab_id: u64,
        error: QuickLaunchErrorState,
    ) {
        self.error_tabs.insert(tab_id, error);
    }

    /// Remove stored error payload for a closed tab.
    pub(crate) fn remove_error_tab(&mut self, tab_id: u64) {
        let _ = self.error_tabs.remove(&tab_id);
    }

    #[cfg(test)]
    pub(crate) fn set_blink_nonce_for_tests(&mut self, value: u64) {
        self.blink_nonce = value;
    }
}

impl Default for QuickLaunchState {
    fn default() -> Self {
        Self {
            data: QuickLaunchFile::empty(),
            error_tabs: HashMap::new(),
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
