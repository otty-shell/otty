use std::collections::{HashMap, HashSet};
use std::sync::{Arc, atomic::AtomicBool};
use std::time::Instant;

use iced::Point;

use super::errors::QuickLaunchError;
use super::model::{NodePath, QuickLaunchFile};
use super::storage::{load_quick_launches, save_quick_launches};

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
    pub(crate) fn load() -> Self {
        match load_quick_launches() {
            Ok(Some(data)) => Self {
                data,
                dirty: false,
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
            Ok(None) => Self::default(),
            Err(err) => {
                log::warn!("quick launches load failed: {err}");
                Self::default()
            },
        }
    }

    pub(crate) fn persist(&mut self) -> Result<(), QuickLaunchError> {
        save_quick_launches(&self.data)?;
        self.dirty = false;
        Ok(())
    }

    pub(crate) fn mark_dirty(&mut self) {
        self.dirty = true;
    }
}

impl Default for QuickLaunchState {
    fn default() -> Self {
        Self {
            data: QuickLaunchFile::empty(),
            dirty: false,
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
