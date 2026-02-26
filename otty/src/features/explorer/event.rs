use std::path::PathBuf;

use otty_ui_tree::TreePath;

use super::model::{ExplorerLoadTarget, FileNode};

/// Events emitted by explorer UI and async services.
#[derive(Debug, Clone)]
pub(crate) enum ExplorerEvent {
    NodePressed {
        path: TreePath,
    },
    NodeHovered {
        path: Option<TreePath>,
    },
    SyncFromActiveTerminal,
    RootLoaded {
        root: PathBuf,
        nodes: Vec<FileNode>,
    },
    FolderLoaded {
        path: TreePath,
        nodes: Vec<FileNode>,
    },
    LoadFailed {
        target: ExplorerLoadTarget,
        message: String,
    },
}
