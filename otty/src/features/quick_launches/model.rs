use std::fmt;

use serde::{Deserialize, Serialize};

use super::errors::QuickLaunchError;

/// Current quick launches schema version.
pub(crate) const QUICK_LAUNCHES_VERSION: u8 = 1;

/// Path of titles from the root to a node.
pub(crate) type NodePath = Vec<String>;

/// Default SSH port for quick launch settings.
pub(crate) const SSH_DEFAULT_PORT: u16 = 22;

/// Supported quick launch types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum QuickLaunchType {
    Custom,
    Ssh,
}

impl fmt::Display for QuickLaunchType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            QuickLaunchType::Custom => "Custom",
            QuickLaunchType::Ssh => "SSH",
        };
        write!(f, "{label}")
    }
}

/// Root payload persisted to disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct QuickLaunchFile {
    pub(crate) version: u8,
    pub(crate) root: QuickLaunchFolder,
}

impl QuickLaunchFile {
    pub(crate) fn empty() -> Self {
        Self {
            version: QUICK_LAUNCHES_VERSION,
            root: QuickLaunchFolder {
                title: String::from("Quick Launces"),
                expanded: true,
                children: Vec::new(),
            },
        }
    }

    pub(crate) fn folder(&self, path: &[String]) -> Option<&QuickLaunchFolder> {
        if path.is_empty() {
            return Some(&self.root);
        }

        let mut current = &self.root;
        for segment in path {
            let node = current.child(segment)?;
            let QuickLaunchNode::Folder(folder) = node else {
                return None;
            };
            current = folder;
        }

        Some(current)
    }

    pub(crate) fn folder_mut(
        &mut self,
        path: &[String],
    ) -> Option<&mut QuickLaunchFolder> {
        if path.is_empty() {
            return Some(&mut self.root);
        }

        let mut current = &mut self.root;
        for segment in path {
            let node = current.child_mut(segment)?;
            let QuickLaunchNode::Folder(folder) = node else {
                return None;
            };
            current = folder;
        }

        Some(current)
    }

    pub(crate) fn node(&self, path: &[String]) -> Option<&QuickLaunchNode> {
        let (title, parent_path) = path.split_last()?;
        let parent = self.folder(parent_path)?;
        parent.child(title)
    }

    pub(crate) fn node_mut(
        &mut self,
        path: &[String],
    ) -> Option<&mut QuickLaunchNode> {
        let (title, parent_path) = path.split_last()?;
        let parent = self.folder_mut(parent_path)?;
        parent.child_mut(title)
    }

    pub(crate) fn parent_folder_mut(
        &mut self,
        path: &[String],
    ) -> Option<&mut QuickLaunchFolder> {
        let (_title, parent_path) = path.split_last()?;
        self.folder_mut(parent_path)
    }
}

/// Folder node in the quick launches tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct QuickLaunchFolder {
    pub(crate) title: String,
    #[serde(default)]
    pub(crate) expanded: bool,
    #[serde(default)]
    pub(crate) children: Vec<QuickLaunchNode>,
}

impl QuickLaunchFolder {
    pub(crate) fn child(&self, title: &str) -> Option<&QuickLaunchNode> {
        self.children.iter().find(|node| node.title() == title)
    }

    pub(crate) fn child_mut(
        &mut self,
        title: &str,
    ) -> Option<&mut QuickLaunchNode> {
        self.children.iter_mut().find(|node| node.title() == title)
    }

    pub(crate) fn contains_title(&self, title: &str) -> bool {
        self.child(title).is_some()
    }

    /// Validate and normalize a title for this folder scope.
    pub(crate) fn normalize_title(
        &self,
        raw: &str,
        current: Option<&str>,
    ) -> Result<String, QuickLaunchError> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(QuickLaunchError::TitleEmpty);
        }

        let conflicts = match current {
            Some(existing) => {
                trimmed != existing && self.contains_title(trimmed)
            },
            None => self.contains_title(trimmed),
        };
        if conflicts {
            return Err(QuickLaunchError::TitleDuplicate);
        }

        Ok(trimmed.to_string())
    }

    pub(crate) fn remove_child(
        &mut self,
        title: &str,
    ) -> Option<QuickLaunchNode> {
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
pub(crate) enum QuickLaunchNode {
    Folder(QuickLaunchFolder),
    Command(QuickLaunch),
}

impl QuickLaunchNode {
    pub(crate) fn title(&self) -> &str {
        match self {
            QuickLaunchNode::Folder(folder) => &folder.title,
            QuickLaunchNode::Command(command) => &command.title,
        }
    }

    pub(crate) fn title_mut(&mut self) -> &mut String {
        match self {
            QuickLaunchNode::Folder(folder) => &mut folder.title,
            QuickLaunchNode::Command(command) => &mut command.title,
        }
    }

    pub(crate) fn is_folder(&self) -> bool {
        matches!(self, QuickLaunchNode::Folder(_))
    }
}

/// Leaf node describing a runnable command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct QuickLaunch {
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
    SSH_DEFAULT_PORT
}

/// Build a detailed error message for a failed quick launch execution.
pub(crate) fn quick_launch_error_message(
    command: &QuickLaunch,
    err: &dyn fmt::Display,
) -> String {
    match &command.spec {
        CommandSpec::Custom { custom } => {
            let program = custom.program.as_str();
            let args = if custom.args.is_empty() {
                String::from("<none>")
            } else {
                custom.args.join(" ")
            };
            let cwd = custom
                .working_directory
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or("<default>");
            let env = if custom.env.is_empty() {
                String::from("<none>")
            } else {
                custom
                    .env
                    .iter()
                    .map(|entry| format!("{}={}", entry.key, entry.value))
                    .collect::<Vec<_>>()
                    .join(", ")
            };

            format!(
                "Type: Custom\nProgram: {program}\nArgs: {args}\nWorking dir: {cwd}\nEnv: {env}\nError: {err}"
            )
        },
        CommandSpec::Ssh { ssh } => {
            let host = ssh.host.as_str();
            let port = ssh.port;
            let user = ssh
                .user
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or("<default>");
            let identity = ssh
                .identity_file
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or("<default>");
            let extra_args = if ssh.extra_args.is_empty() {
                String::from("<none>")
            } else {
                ssh.extra_args.join(" ")
            };

            format!(
                "Type: SSH\nHost: {host}\nPort: {port}\nUser: {user}\nIdentity file: {identity}\nExtra args: {extra_args}\nError: {err}"
            )
        },
    }
}
