use iced::Point;

use super::model::{NodePath, QuickCommandsFile};
use super::storage::{
    QuickCommandsError, load_quick_commands, save_quick_commands,
};

/// Target location for quick command context menus.
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
}

/// Workspace state for saved quick commands and their UI.
#[derive(Debug)]
pub(crate) struct QuickCommandsState {
    pub(crate) data: QuickCommandsFile,
    pub(crate) dirty: bool,
    pub(crate) last_error: Option<String>,
    pub(crate) selected: Option<NodePath>,
    pub(crate) hovered: Option<NodePath>,
    pub(crate) context_menu: Option<ContextMenuState>,
    pub(crate) inline_edit: Option<InlineEditState>,
    pub(crate) pressed: Option<NodePath>,
    pub(crate) drag: Option<DragState>,
    pub(crate) drop_target: Option<DropTarget>,
    pub(crate) cursor: Point,
}

impl QuickCommandsState {
    pub(crate) fn load() -> Self {
        match load_quick_commands() {
            Ok(Some(data)) => Self {
                data,
                dirty: false,
                last_error: None,
                selected: None,
                hovered: None,
                context_menu: None,
                inline_edit: None,
                pressed: None,
                drag: None,
                drop_target: None,
                cursor: Point::ORIGIN,
            },
            Ok(None) => Self::default(),
            Err(err) => {
                log::warn!("quick commands load failed: {err}");
                Self {
                    last_error: Some(format!("{err}")),
                    ..Self::default()
                }
            },
        }
    }

    pub(crate) fn persist(&mut self) -> Result<(), QuickCommandsError> {
        save_quick_commands(&self.data)?;
        self.dirty = false;
        self.last_error = None;
        Ok(())
    }

    pub(crate) fn mark_dirty(&mut self) {
        self.dirty = true;
    }
}

impl Default for QuickCommandsState {
    fn default() -> Self {
        Self {
            data: QuickCommandsFile::empty(),
            dirty: false,
            last_error: None,
            selected: None,
            hovered: None,
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
