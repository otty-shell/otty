use std::path::PathBuf;

use super::model::{FileNode, TreePath};

/// Internal commands dispatched to the explorer reducer.
#[derive(Debug, Clone)]
pub(crate) enum ExplorerCommand {
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
    /// A directory load operation failed.
    LoadFailed { message: String },
}
