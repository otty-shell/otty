use serde::{Deserialize, Serialize};

/// Current quick commands schema version.
pub(crate) const QUICK_COMMANDS_VERSION: u8 = 1;

/// Path of titles from the root to a node.
pub(crate) type NodePath = Vec<String>;

/// Root payload persisted to disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct QuickCommandsFile {
    pub(crate) version: u8,
    pub(crate) root: QuickCommandFolder,
}

impl QuickCommandsFile {
    pub(crate) fn empty() -> Self {
        Self {
            version: QUICK_COMMANDS_VERSION,
            root: QuickCommandFolder {
                title: String::from("Quick Commands"),
                expanded: true,
                children: Vec::new(),
            },
        }
    }

    pub(crate) fn folder(
        &self,
        path: &[String],
    ) -> Option<&QuickCommandFolder> {
        if path.is_empty() {
            return Some(&self.root);
        }

        let mut current = &self.root;
        for segment in path {
            let node = current.child(segment)?;
            let QuickCommandNode::Folder(folder) = node else {
                return None;
            };
            current = folder;
        }

        Some(current)
    }

    pub(crate) fn folder_mut(
        &mut self,
        path: &[String],
    ) -> Option<&mut QuickCommandFolder> {
        if path.is_empty() {
            return Some(&mut self.root);
        }

        let mut current = &mut self.root;
        for segment in path {
            let node = current.child_mut(segment)?;
            let QuickCommandNode::Folder(folder) = node else {
                return None;
            };
            current = folder;
        }

        Some(current)
    }

    pub(crate) fn node(&self, path: &[String]) -> Option<&QuickCommandNode> {
        let (title, parent_path) = path.split_last()?;
        let parent = self.folder(parent_path)?;
        parent.child(title)
    }

    pub(crate) fn node_mut(
        &mut self,
        path: &[String],
    ) -> Option<&mut QuickCommandNode> {
        let (title, parent_path) = path.split_last()?;
        let parent = self.folder_mut(parent_path)?;
        parent.child_mut(title)
    }

    pub(crate) fn parent_folder_mut(
        &mut self,
        path: &[String],
    ) -> Option<&mut QuickCommandFolder> {
        let (_title, parent_path) = path.split_last()?;
        self.folder_mut(parent_path)
    }
}

/// Folder node in the quick commands tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct QuickCommandFolder {
    pub(crate) title: String,
    #[serde(default)]
    pub(crate) expanded: bool,
    #[serde(default)]
    pub(crate) children: Vec<QuickCommandNode>,
}

impl QuickCommandFolder {
    pub(crate) fn child(&self, title: &str) -> Option<&QuickCommandNode> {
        self.children.iter().find(|node| node.title() == title)
    }

    pub(crate) fn child_mut(
        &mut self,
        title: &str,
    ) -> Option<&mut QuickCommandNode> {
        self.children.iter_mut().find(|node| node.title() == title)
    }

    pub(crate) fn contains_title(&self, title: &str) -> bool {
        self.child(title).is_some()
    }

    pub(crate) fn remove_child(
        &mut self,
        title: &str,
    ) -> Option<QuickCommandNode> {
        let index = self
            .children
            .iter()
            .position(|node| node.title() == title)?;
        Some(self.children.remove(index))
    }
}

/// Tree node representing either a folder or a command.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum QuickCommandNode {
    Folder(QuickCommandFolder),
    Command(QuickCommand),
}

impl QuickCommandNode {
    pub(crate) fn title(&self) -> &str {
        match self {
            QuickCommandNode::Folder(folder) => &folder.title,
            QuickCommandNode::Command(command) => &command.title,
        }
    }

    pub(crate) fn title_mut(&mut self) -> &mut String {
        match self {
            QuickCommandNode::Folder(folder) => &mut folder.title,
            QuickCommandNode::Command(command) => &mut command.title,
        }
    }

    pub(crate) fn is_folder(&self) -> bool {
        matches!(self, QuickCommandNode::Folder(_))
    }
}

/// Leaf node describing a runnable command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct QuickCommand {
    pub(crate) title: String,
    #[serde(flatten)]
    pub(crate) spec: CommandSpec,
}

/// Variants for supported command types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "command_type", rename_all = "snake_case")]
pub(crate) enum CommandSpec {
    Custom { custom: CustomCommand },
    Ssh { ssh: SshCommand },
}

/// Local program execution specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CustomCommand {
    pub(crate) program: String,
    #[serde(default)]
    pub(crate) args: Vec<String>,
    #[serde(default)]
    pub(crate) env: Vec<EnvVar>,
    #[serde(default)]
    pub(crate) working_directory: Option<String>,
}

/// Environment variable entry for custom commands.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct EnvVar {
    pub(crate) key: String,
    pub(crate) value: String,
}

/// SSH connection specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SshCommand {
    pub(crate) host: String,
    #[serde(default = "default_ssh_port")]
    pub(crate) port: u16,
    #[serde(default)]
    pub(crate) user: Option<String>,
    #[serde(default)]
    pub(crate) identity_file: Option<String>,
    #[serde(default)]
    pub(crate) extra_args: Vec<String>,
}

fn default_ssh_port() -> u16 {
    22
}
