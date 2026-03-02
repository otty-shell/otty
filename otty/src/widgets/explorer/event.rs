use std::path::PathBuf;

use super::model::{FileNode, TreePath};

/// Intent events handled by the explorer presentation layer.
#[derive(Debug, Clone)]
pub(crate) enum ExplorerIntent {
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

/// Effect events produced by the explorer reducer.
#[derive(Debug, Clone)]
pub(crate) enum ExplorerEffect {
    /// Request asynchronous loading of the root directory.
    LoadRootRequested { root: PathBuf },
    /// Request asynchronous loading of a folder's children.
    LoadFolderRequested { path: TreePath, directory: PathBuf },
    /// Request opening a file in a command terminal tab.
    OpenFileTerminalTab { file_path: PathBuf },
}

/// Explorer event stream routed through the app update loop.
#[derive(Debug, Clone)]
pub(crate) enum ExplorerEvent {
    /// Intent event reduced by the explorer widget.
    Intent(ExplorerIntent),
    /// External effect orchestrated by app-level routing.
    Effect(ExplorerEffect),
}
