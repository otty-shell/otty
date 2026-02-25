use std::fmt;

use otty_ui_term::settings::Settings;

use crate::features::quick_launch::{NodePath, QuickLaunch};

/// Supported requests for opening a tab in the workspace.
#[derive(Clone)]
pub(crate) enum TabOpenRequest {
    Terminal,
    Settings,
    QuickLaunchWizardCreate {
        parent_path: NodePath,
    },
    QuickLaunchWizardEdit {
        path: NodePath,
        command: Box<QuickLaunch>,
    },
    QuickLaunchError {
        title: String,
        message: String,
    },
    CommandTerminal {
        title: String,
        settings: Box<Settings>,
    },
    QuickLaunchCommandTerminal {
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
            TabOpenRequest::QuickLaunchWizardCreate { parent_path } => f
                .debug_struct("QuickLaunchWizardCreate")
                .field("parent_path", parent_path)
                .finish(),
            TabOpenRequest::QuickLaunchWizardEdit { path, command } => f
                .debug_struct("QuickLaunchWizardEdit")
                .field("path", path)
                .field("command", command)
                .finish(),
            TabOpenRequest::QuickLaunchError { title, message } => f
                .debug_struct("QuickLaunchError")
                .field("title", title)
                .field("message", message)
                .finish(),
            TabOpenRequest::CommandTerminal { title, .. } => f
                .debug_struct("CommandTerminal")
                .field("title", title)
                .finish(),
            TabOpenRequest::QuickLaunchCommandTerminal {
                title,
                command,
                ..
            } => f
                .debug_struct("QuickLaunchCommandTerminal")
                .field("title", title)
                .field("command", command)
                .finish(),
        }
    }
}

/// Tab payloads stored in app state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TabContent {
    Terminal,
    Settings,
    QuickLaunchWizard,
    QuickLaunchError,
}

/// Metadata for a single tab entry.
pub(crate) struct TabItem {
    id: u64,
    title: String,
    content: TabContent,
}

impl TabItem {
    /// Create tab metadata with immutable identity and content kind.
    pub(crate) fn new(id: u64, title: String, content: TabContent) -> Self {
        Self { id, title, content }
    }

    /// Return tab identifier.
    pub(crate) fn id(&self) -> u64 {
        self.id
    }

    /// Return tab title shown in the tab bar.
    pub(crate) fn title(&self) -> &str {
        &self.title
    }

    /// Return content discriminator used by feature owners.
    pub(crate) fn content(&self) -> TabContent {
        self.content
    }

    /// Update tab title through tab reducer domain APIs.
    pub(crate) fn set_title(&mut self, title: String) {
        self.title = title;
    }
}
