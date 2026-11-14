//! What the user is doing with the Files tree right now, besides looking at
//! it: an open context menu, a name being typed, a deletion waiting for its
//! answer, or a row being dragged.

use std::path::PathBuf;

use iced::widget::Id;
use iced::{Point, Size};

use crate::types::TreePath;

/// The row the context menu was opened on, which decides its items.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MenuTarget {
    /// The empty area of the tree: the followed folder itself.
    Background,
    /// A folder row.
    Folder(TreePath),
    /// A file row.
    File(TreePath),
}

/// An open context menu: what it was opened on and where.
#[derive(Debug, Clone)]
pub(crate) struct ContextMenu {
    pub(crate) target: MenuTarget,
    /// The pointer in the window when the menu opened.
    pub(crate) cursor: Point,
    /// Size of the main area; the menu stays inside it.
    pub(crate) area_size: Size,
}

/// What the typed name is for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum InlineEditKind {
    /// A new file in the folder at `parent`; empty for the followed folder.
    NewFile { parent: TreePath },
    /// A new folder in the folder at `parent`; empty for the followed
    /// folder.
    NewFolder { parent: TreePath },
    /// A new name of the entry at `path`.
    Rename { path: TreePath },
}

/// A name typed in the tree, with the error of the last try to use it.
#[derive(Debug, Clone)]
pub(crate) struct InlineEdit {
    pub(crate) kind: InlineEditKind,
    pub(crate) value: String,
    pub(crate) error: Option<String>,
    /// Id of the name field, which gets the keyboard when it shows.
    pub(crate) id: Id,
}

impl InlineEdit {
    /// Start typing a name for `kind`, with `value` in the field.
    pub(crate) fn new(kind: InlineEditKind, value: String) -> Self {
        Self {
            kind,
            value,
            error: None,
            id: Id::unique(),
        }
    }
}

/// A deletion that waits for the user's answer in the dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PendingDelete {
    pub(crate) path: PathBuf,
    pub(crate) name: String,
    pub(crate) is_folder: bool,
}

/// A row held down by the pointer. It becomes a drag once the pointer
/// moves far enough from where it was pressed.
#[derive(Debug, Clone)]
pub(crate) struct Drag {
    pub(crate) source: TreePath,
    pub(crate) origin: Point,
    pub(crate) is_active: bool,
}

/// Where a dragged row would land if it were dropped now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DropTarget {
    /// The followed folder itself.
    Root,
    /// The folder at this place in the tree.
    Folder(TreePath),
}

impl DropTarget {
    /// Return the place of the folder in the tree; empty for the followed
    /// folder.
    pub(crate) fn folder_path(&self) -> &[String] {
        match self {
            DropTarget::Root => &[],
            DropTarget::Folder(path) => path,
        }
    }
}
