use std::path::PathBuf;

use crate::types::{FileNode, TreePath};

/// What the explorer is asked to do: a press or hover in the Files tree, the
/// folder to follow, or the answer of a folder read the binary ran.
#[derive(Debug, Clone)]
pub enum ExplorerIntent {
    /// A tree node was clicked.
    NodePressed { path: TreePath },
    /// The cursor entered a tree node.
    NodeHovered { path: Option<TreePath> },
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
}

/// What the explorer reports back to the binary: a folder to read, a file
/// to open, or a failure to tell the user about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExplorerOutcome {
    /// Read the folder the explorer follows now; the answer comes back as
    /// [`ExplorerIntent::RootLoaded`], a failure as
    /// [`ExplorerIntent::LoadFailed`].
    LoadRoot { root: PathBuf },
    /// Read a folder the user pressed open; the answer comes back as
    /// [`ExplorerIntent::FolderLoaded`], a failure as
    /// [`ExplorerIntent::FolderLoadFailed`].
    LoadFolder { path: TreePath, directory: PathBuf },
    /// Read again a watched folder that changed on disk; the answer comes
    /// back as [`ExplorerIntent::DirectoryReloaded`], a failure as
    /// [`ExplorerIntent::LoadFailed`].
    ReloadFolder { directory: PathBuf },
    /// The user pressed a file: open it in the saved editor, in a command
    /// terminal tab.
    OpenFile { file_path: PathBuf },
    /// Tell the user about a failure; the text is ready to show.
    Failed(String),
}
