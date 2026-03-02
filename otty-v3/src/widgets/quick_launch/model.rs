use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use serde::{Deserialize, Serialize};

/// Path to a node in the quick launch tree.
pub(crate) type NodePath = Vec<String>;

/// Default SSH port.
pub(crate) const SSH_DEFAULT_PORT: u16 = 22;

/// Quick launch command type selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum QuickLaunchType {
    Custom,
    Ssh,
}

/// A saved quick launch command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct QuickLaunch {
    pub(super) title: String,
    pub(super) spec: CommandSpec,
}

impl QuickLaunch {
    /// Command title.
    pub(crate) fn title(&self) -> &str {
        &self.title
    }

    /// Command specification.
    pub(crate) fn spec(&self) -> &CommandSpec {
        &self.spec
    }
}

/// Command specification variant.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub(crate) enum CommandSpec {
    Custom { custom: CustomCommand },
    Ssh { ssh: SshCommand },
}

/// Local command specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CustomCommand {
    pub(super) program: String,
    pub(super) args: Vec<String>,
    pub(super) env: Vec<EnvVar>,
    pub(super) working_directory: Option<String>,
}

impl CustomCommand {
    pub(crate) fn program(&self) -> &str {
        &self.program
    }

    pub(crate) fn args(&self) -> &[String] {
        &self.args
    }

    pub(crate) fn env(&self) -> &[EnvVar] {
        &self.env
    }

    pub(crate) fn working_directory(&self) -> Option<&str> {
        self.working_directory.as_deref()
    }
}

/// Environment variable key-value pair.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct EnvVar {
    pub(super) key: String,
    pub(super) value: String,
}

impl EnvVar {
    pub(crate) fn key(&self) -> &str {
        &self.key
    }

    pub(crate) fn value(&self) -> &str {
        &self.value
    }
}

/// SSH command specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SshCommand {
    pub(super) host: String,
    pub(super) port: u16,
    pub(super) user: Option<String>,
    pub(super) identity_file: Option<String>,
    pub(super) extra_args: Vec<String>,
}

impl SshCommand {
    pub(crate) fn host(&self) -> &str {
        &self.host
    }

    pub(crate) fn port(&self) -> u16 {
        self.port
    }

    pub(crate) fn user(&self) -> Option<&str> {
        self.user.as_deref()
    }

    pub(crate) fn identity_file(&self) -> Option<&str> {
        self.identity_file.as_deref()
    }

    pub(crate) fn extra_args(&self) -> &[String] {
        &self.extra_args
    }
}

/// Top-level persistence structure for quick launches.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct QuickLaunchFile {
    pub(super) version: u32,
    pub(super) root: QuickLaunchFolder,
}

impl Default for QuickLaunchFile {
    fn default() -> Self {
        Self {
            version: 1,
            root: QuickLaunchFolder {
                title: String::from("Root"),
                expanded: true,
                children: Vec::new(),
            },
        }
    }
}

impl QuickLaunchFile {
    /// Return the root folder.
    pub(crate) fn root(&self) -> &QuickLaunchFolder {
        &self.root
    }

    /// Return a node by path.
    pub(crate) fn node(&self, path: &[String]) -> Option<&QuickLaunchNode> {
        let mut current = &self.root.children;
        let mut result: Option<&QuickLaunchNode> = None;

        for segment in path {
            result = current.iter().find(|n| n.title() == segment);
            match result {
                Some(QuickLaunchNode::Folder(f)) => current = &f.children,
                Some(QuickLaunchNode::Command(_)) => {},
                None => return None,
            }
        }

        result
    }

    /// Return a mutable node by path.
    pub(crate) fn node_mut(
        &mut self,
        path: &[String],
    ) -> Option<&mut QuickLaunchNode> {
        let mut current = &mut self.root.children;

        for (i, segment) in path.iter().enumerate() {
            let idx = current.iter().position(|n| n.title() == segment)?;
            if i == path.len() - 1 {
                return current.get_mut(idx);
            }
            match &mut current[idx] {
                QuickLaunchNode::Folder(f) => current = &mut f.children,
                QuickLaunchNode::Command(_) => return None,
            }
        }

        None
    }

    /// Return a folder by path.
    pub(crate) fn folder(&self, path: &[String]) -> Option<&QuickLaunchFolder> {
        if path.is_empty() {
            return Some(&self.root);
        }
        match self.node(path)? {
            QuickLaunchNode::Folder(f) => Some(f),
            QuickLaunchNode::Command(_) => None,
        }
    }

    /// Return a mutable folder by path.
    pub(crate) fn folder_mut(
        &mut self,
        path: &[String],
    ) -> Option<&mut QuickLaunchFolder> {
        if path.is_empty() {
            return Some(&mut self.root);
        }
        match self.node_mut(path)? {
            QuickLaunchNode::Folder(f) => Some(f),
            QuickLaunchNode::Command(_) => None,
        }
    }

    /// Return parent folder for a node path.
    pub(crate) fn parent_folder_mut(
        &mut self,
        path: &[String],
    ) -> Option<&mut QuickLaunchFolder> {
        if path.is_empty() {
            return None;
        }
        let parent = &path[..path.len() - 1];
        self.folder_mut(parent)
    }
}

/// A folder in the quick launch tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct QuickLaunchFolder {
    pub(super) title: String,
    pub(super) expanded: bool,
    pub(super) children: Vec<QuickLaunchNode>,
}

impl QuickLaunchFolder {
    pub(crate) fn title(&self) -> &str {
        &self.title
    }

    pub(crate) fn is_expanded(&self) -> bool {
        self.expanded
    }

    pub(crate) fn children(&self) -> &[QuickLaunchNode] {
        &self.children
    }

    /// Return whether a child with the given title exists.
    pub(crate) fn contains_title(&self, title: &str) -> bool {
        self.children.iter().any(|n| n.title() == title)
    }

    /// Validate and normalize a title within this folder.
    ///
    /// Returns an error if the title is empty or duplicates an existing
    /// sibling (ignoring `exclude` which is the current title during rename).
    pub(crate) fn normalize_title(
        &self,
        raw: &str,
        exclude: Option<&str>,
    ) -> Result<String, super::errors::QuickLaunchError> {
        let trimmed = raw.trim().to_string();
        if trimmed.is_empty() {
            return Err(super::errors::QuickLaunchError::TitleEmpty);
        }
        let is_duplicate = self.children.iter().any(|n| {
            let title = n.title();
            title == trimmed && exclude.is_none_or(|excluded| title != excluded)
        });
        if is_duplicate {
            return Err(super::errors::QuickLaunchError::TitleDuplicate);
        }
        Ok(trimmed)
    }

    /// Remove a child by title, returning the removed node.
    pub(crate) fn remove_child(
        &mut self,
        title: &str,
    ) -> Option<QuickLaunchNode> {
        let idx = self.children.iter().position(|n| n.title() == title)?;
        Some(self.children.remove(idx))
    }
}

/// A node in the quick launch tree (folder or command).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub(crate) enum QuickLaunchNode {
    Folder(QuickLaunchFolder),
    Command(QuickLaunch),
}

impl QuickLaunchNode {
    /// Return the title of this node.
    pub(crate) fn title(&self) -> &str {
        match self {
            Self::Folder(f) => &f.title,
            Self::Command(c) => &c.title,
        }
    }

    /// Return a mutable reference to the title.
    pub(crate) fn title_mut(&mut self) -> &mut String {
        match self {
            Self::Folder(f) => &mut f.title,
            Self::Command(c) => &mut c.title,
        }
    }
}

/// In-flight launch tracking info.
#[derive(Debug, Clone)]
pub(crate) struct LaunchInfo {
    pub(crate) id: u64,
    pub(crate) launch_ticks: u64,
    pub(crate) is_indicator_highlighted: bool,
    pub(crate) cancel: Arc<AtomicBool>,
}

/// Result of async launch preparation.
#[derive(Debug, Clone)]
pub(crate) struct PreparedQuickLaunch {
    pub(crate) path: NodePath,
    pub(crate) launch_id: u64,
    pub(crate) title: String,
    pub(crate) settings: otty_ui_term::settings::Settings,
    pub(crate) command: Box<QuickLaunch>,
}

/// Outcome of a quick launch setup attempt.
#[derive(Debug, Clone)]
pub(crate) enum QuickLaunchSetupOutcome {
    Prepared(PreparedQuickLaunch),
    Failed {
        path: NodePath,
        launch_id: u64,
        command: Box<QuickLaunch>,
        error: Arc<super::errors::QuickLaunchError>,
    },
    Canceled {
        path: NodePath,
        launch_id: u64,
    },
}

/// Target for a wizard save operation.
#[derive(Debug, Clone)]
pub(crate) enum QuickLaunchWizardSaveTarget {
    Create { parent_path: NodePath },
    Edit { path: NodePath },
}

/// Request to save a wizard form result.
#[derive(Debug, Clone)]
pub(crate) struct QuickLaunchWizardSaveRequest {
    pub(crate) tab_id: u64,
    pub(crate) target: QuickLaunchWizardSaveTarget,
    pub(crate) command: QuickLaunch,
}

/// Target for a context menu.
#[derive(Debug, Clone)]
pub(crate) enum ContextMenuTarget {
    Background,
    Folder(NodePath),
    Command(NodePath),
}

/// Actions available in the context menu.
#[derive(Debug, Clone)]
pub(crate) enum ContextMenuAction {
    Edit,
    Rename,
    Duplicate,
    Remove,
    Delete,
    CreateFolder,
    CreateCommand,
    Kill,
}

/// Read-only view model for the quick launch tree.
#[derive(Debug, Clone)]
pub(crate) struct QuickLaunchTreeViewModel<'a> {
    pub(crate) data: &'a QuickLaunchFile,
    pub(crate) selected_path: Option<&'a NodePath>,
    pub(crate) hovered_path: Option<&'a NodePath>,
    pub(crate) inline_edit: Option<&'a super::state::InlineEditState>,
    pub(crate) launching: &'a std::collections::HashMap<NodePath, LaunchInfo>,
    pub(crate) drop_target: Option<&'a super::state::DropTarget>,
}

/// Read-only view model for the quick launch error tab.
#[derive(Debug, Clone)]
pub(crate) struct QuickLaunchErrorViewModel<'a> {
    pub(crate) title: &'a str,
    pub(crate) message: &'a str,
}

/// Read-only view model for the quick launch context menu.
#[derive(Debug, Clone)]
pub(crate) struct QuickLaunchContextMenuViewModel<'a> {
    pub(crate) menu: &'a super::state::ContextMenuState,
    pub(crate) launching: &'a std::collections::HashMap<NodePath, LaunchInfo>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_empty_title_when_normalizing_then_error_is_returned() {
        let folder = QuickLaunchFolder {
            title: String::from("Root"),
            expanded: true,
            children: Vec::new(),
        };
        assert!(folder.normalize_title("  ", None).is_err());
    }

    #[test]
    fn given_duplicate_title_when_normalizing_then_error_is_returned() {
        let folder = QuickLaunchFolder {
            title: String::from("Root"),
            expanded: true,
            children: vec![QuickLaunchNode::Command(QuickLaunch {
                title: String::from("Run"),
                spec: CommandSpec::Custom {
                    custom: CustomCommand {
                        program: String::from("bash"),
                        args: Vec::new(),
                        env: Vec::new(),
                        working_directory: None,
                    },
                },
            })],
        };
        assert!(folder.normalize_title("Run", None).is_err());
    }

    #[test]
    fn given_rename_same_title_when_normalizing_then_title_is_accepted() {
        let folder = QuickLaunchFolder {
            title: String::from("Root"),
            expanded: true,
            children: vec![QuickLaunchNode::Command(QuickLaunch {
                title: String::from("Run"),
                spec: CommandSpec::Custom {
                    custom: CustomCommand {
                        program: String::from("bash"),
                        args: Vec::new(),
                        env: Vec::new(),
                        working_directory: None,
                    },
                },
            })],
        };
        assert!(folder.normalize_title("Run", Some("Run")).is_ok());
    }
}
