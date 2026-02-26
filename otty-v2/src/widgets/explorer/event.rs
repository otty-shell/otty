use std::path::PathBuf;

use otty_ui_term::settings::Settings;
use otty_ui_tree::TreePath;

use super::model::{ExplorerLoadTarget, FileNode};

/// UI-level events handled by explorer router.
#[derive(Debug, Clone)]
pub(crate) enum ExplorerUiEvent {
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

/// Side-effect events emitted by explorer reducer.
#[derive(Debug, Clone)]
pub(crate) enum ExplorerEffectEvent {
    LoadRootRequested {
        root: PathBuf,
    },
    LoadFolderRequested {
        path: TreePath,
        directory: PathBuf,
    },
    OpenCommandTerminalTab {
        title: String,
        settings: Box<Settings>,
    },
}
