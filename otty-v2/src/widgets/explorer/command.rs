use std::path::PathBuf;

use otty_ui_tree::TreePath;

use super::model::{ExplorerLoadTarget, FileNode};

/// Commands accepted by explorer reducer.
#[derive(Debug, Clone)]
pub(crate) enum ExplorerCommand {
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
