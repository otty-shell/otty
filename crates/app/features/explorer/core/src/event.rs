use std::path::PathBuf;

use iced::Point;
use iced::widget::Id;

use crate::disk_operation::DiskOperation;
use crate::types::{FileNode, TreePath};

/// What the explorer is asked to do: a press, a hover or a drag in the
/// Files tree, its context menu, name field and delete dialog, the folder to
/// follow, or the answer of a folder read or a disk operation the binary
/// ran.
#[derive(Debug, Clone)]
pub enum ExplorerIntent {
    /// A tree row was pressed: it is selected and may be dragged.
    NodePressed { path: TreePath },
    /// A tree row was released: a click opens the file or the folder, a
    /// drag drops the row.
    NodeReleased { path: TreePath },
    /// A tree row was right-clicked: its context menu opens.
    NodeRightClicked { path: TreePath },
    /// The cursor entered a tree node.
    NodeHovered { path: Option<TreePath> },
    /// The empty area of the tree was pressed.
    BackgroundPressed,
    /// The empty area of the tree was released: a drag drops the row into
    /// the followed folder.
    BackgroundReleased,
    /// The empty area of the tree was right-clicked: the menu of the
    /// followed folder opens.
    BackgroundRightClicked,
    /// The pointer moved over the tree.
    CursorMoved { position: Point },

    /// Close the context menu.
    ContextMenuDismiss,
    /// An item of the context menu was pressed.
    ContextMenuAction(ExplorerMenuAction),

    /// The name typed in the tree changed.
    InlineEditChanged(String),
    /// Enter in the name field: create the entry or rename it.
    InlineEditSubmit,
    /// Stop naming: Esc, or any other input of the window.
    CancelInlineEdit,

    /// Delete in the dialog that asks before a deletion.
    DeleteConfirmed,
    /// Cancel in the dialog that asks before a deletion.
    DeleteCancelled,

    /// Sync explorer root from the active terminal CWD.
    SyncRoot { cwd: PathBuf },
    /// Root directory contents loaded successfully.
    RootLoaded { root: PathBuf, nodes: Vec<FileNode> },
    /// Folder contents loaded successfully.
    FolderLoaded {
        path: TreePath,
        nodes: Vec<FileNode>,
    },
    /// A watched directory emitted a filesystem tree change.
    DirectoryChanged { directory: PathBuf },
    /// Watched directory contents reloaded successfully.
    DirectoryReloaded {
        directory: PathBuf,
        nodes: Vec<FileNode>,
    },
    /// A directory load operation failed.
    LoadFailed { message: String },
    /// A folder the user pressed open could not be read.
    FolderLoadFailed { path: TreePath, message: String },
    /// A disk operation was done.
    DiskOperationDone { operation: DiskOperation },
    /// A disk operation could not be done; why.
    DiskOperationFailed {
        operation: DiskOperation,
        message: String,
    },
}

/// What the explorer reports back to the binary: a folder to read, a file
/// to open, a disk operation to run, a terminal to open, the keyboard
/// focus, or a failure to tell the user about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExplorerOutcome {
    /// Read the folder the explorer follows now; the answer comes back as
    /// [`ExplorerIntent::RootLoaded`], a failure as
    /// [`ExplorerIntent::LoadFailed`].
    LoadRoot { root: PathBuf },
    /// Read a folder the user opened; the answer comes back as
    /// [`ExplorerIntent::FolderLoaded`], a failure as
    /// [`ExplorerIntent::FolderLoadFailed`].
    LoadFolder { path: TreePath, directory: PathBuf },
    /// Read again a watched folder that changed on disk; the answer comes
    /// back as [`ExplorerIntent::DirectoryReloaded`], a failure as
    /// [`ExplorerIntent::LoadFailed`].
    ReloadFolder { directory: PathBuf },
    /// The user opened a file: open it in the saved editor, in a command
    /// terminal tab.
    OpenFile { file_path: PathBuf },
    /// Run a disk operation; the answer comes back as
    /// [`ExplorerIntent::DiskOperationDone`] or [`ExplorerIntent::DiskOperationFailed`].
    RunDiskOperation(DiskOperation),
    /// Open a new terminal tab whose shell starts in `folder`.
    OpenTerminal { folder: PathBuf },
    /// Give the keyboard to the name field in the tree.
    FocusInput(Id),
    /// The context menu opened: it takes the keyboard from the terminal, so
    /// typed keys do not reach the shell and Esc reaches the frame.
    TakeKeyboard,
    /// The last open overlay of Files closed: the context menu, the delete
    /// dialog or the name field. The active pane gets its keys back.
    OverlayClosed,
    /// Tell the user about a failure; the text is ready to show.
    Failed(String),
}

/// The items of the Files context menu. Which of them show depends on the
/// row: a file, a folder or the empty area of the tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExplorerMenuAction {
    /// Open the file in the editor, or open the folder in the tree.
    Open,
    /// Open a new terminal tab in the folder.
    OpenInTerminal,
    /// Name a new file in the folder.
    NewFile,
    /// Name a new folder in the folder.
    NewFolder,
    /// Type a new name for the file or the folder.
    Rename,
    /// Delete the file or the folder, after the user confirms it.
    Delete,
}
