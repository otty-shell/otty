use std::fmt;

use crate::features::quick_launches::QuickLaunchErrorState;
use crate::features::quick_launches::{
    NodePath, QuickLaunch, QuickLaunchEditorState,
};
use crate::features::terminal::TerminalState;
use otty_ui_term::settings::Settings;

/// Supported requests for opening a tab in the workspace.
#[derive(Clone)]
pub(crate) enum TabOpenRequest {
    Terminal,
    Settings,
    QuickLaunchEditorCreate {
        parent_path: NodePath,
    },
    QuickLaunchEditorEdit {
        path: NodePath,
        command: Box<QuickLaunch>,
    },
    QuickLaunchError {
        title: String,
        message: String,
    },
    CommandTerminal {
        tab_id: u64,
        terminal_id: u64,
        title: String,
        settings: Box<Settings>,
    },
    QuickLaunchCommandTerminal {
        tab_id: u64,
        terminal_id: u64,
        title: String,
        settings: Box<Settings>,
        command: Box<QuickLaunch>,
    },
}

impl fmt::Debug for TabOpenRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TabOpenRequest::Terminal => f.write_str("Terminal"),
            TabOpenRequest::Settings => f.write_str("Settings"),
            TabOpenRequest::QuickLaunchEditorCreate { parent_path } => f
                .debug_struct("QuickLaunchEditorCreate")
                .field("parent_path", parent_path)
                .finish(),
            TabOpenRequest::QuickLaunchEditorEdit { path, command } => f
                .debug_struct("QuickLaunchEditorEdit")
                .field("path", path)
                .field("command", command)
                .finish(),
            TabOpenRequest::QuickLaunchError { title, message } => f
                .debug_struct("QuickLaunchError")
                .field("title", title)
                .field("message", message)
                .finish(),
            TabOpenRequest::CommandTerminal {
                tab_id,
                terminal_id,
                title,
                ..
            } => f
                .debug_struct("CommandTerminal")
                .field("tab_id", tab_id)
                .field("terminal_id", terminal_id)
                .field("title", title)
                .finish(),
            TabOpenRequest::QuickLaunchCommandTerminal {
                tab_id,
                terminal_id,
                title,
                command,
                ..
            } => f
                .debug_struct("QuickLaunchCommandTerminal")
                .field("tab_id", tab_id)
                .field("terminal_id", terminal_id)
                .field("title", title)
                .field("command", command)
                .finish(),
        }
    }
}

/// Tab payloads stored in app state.
pub(crate) enum TabContent {
    Terminal(Box<TerminalState>),
    Settings,
    QuickLaunchEditor(Box<QuickLaunchEditorState>),
    QuickLaunchError(QuickLaunchErrorState),
}

/// Metadata for a single tab entry.
pub(crate) struct TabItem {
    pub(crate) id: u64,
    pub(crate) title: String,
    pub(crate) content: TabContent,
}
