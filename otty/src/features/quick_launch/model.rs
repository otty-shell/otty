use std::fmt;
use std::sync::Arc;

use otty_ui_term::settings::Settings;
use serde::{Deserialize, Serialize};

use super::errors::QuickLaunchError;

/// Current quick launches schema version.
pub(crate) const QUICK_LAUNCHES_VERSION: u8 = 1;

/// Path of titles from the root to a node.
pub(crate) type NodePath = Vec<String>;

/// Prepared runtime payload for launching a quick launch command.
#[derive(Clone)]
pub(crate) struct PreparedQuickLaunch {
    pub(crate) path: NodePath,
    pub(crate) launch_id: u64,
    pub(crate) title: String,
    pub(crate) settings: Box<Settings>,
    pub(crate) command: Box<QuickLaunch>,
}

impl fmt::Debug for PreparedQuickLaunch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PreparedQuickLaunch")
            .field("path", &self.path)
            .field("launch_id", &self.launch_id)
            .field("title", &self.title)
            .finish()
    }
}

/// Outcome produced after quick launch preflight/setup.
#[derive(Debug, Clone)]
pub(crate) enum QuickLaunchSetupOutcome {
    Prepared(PreparedQuickLaunch),
    Failed {
        path: NodePath,
        launch_id: u64,
        command: Box<QuickLaunch>,
        error: Arc<QuickLaunchError>,
    },
    Canceled {
        path: NodePath,
        launch_id: u64,
    },
}

/// Save target emitted by tab quick launch editor.
#[derive(Debug, Clone)]
pub(crate) enum QuickLaunchWizardSaveTarget {
    Create { parent_path: NodePath },
    Edit { path: NodePath },
}

/// Actions that can be triggered from the context menu.
#[derive(Debug, Clone, Copy)]
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

/// Save request emitted by tab quick launch editor.
#[derive(Debug, Clone)]
pub(crate) struct QuickLaunchWizardSaveRequest {
    pub(crate) tab_id: u64,
    pub(crate) target: QuickLaunchWizardSaveTarget,
    pub(crate) command: QuickLaunch,
}

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

/// Validate a custom command before runtime preflight.
pub(crate) fn validate_custom_command(
    custom: &CustomCommand,
) -> Result<(), QuickLaunchError> {
    let program = custom.program.trim();
    if program.is_empty() {
        return Err(QuickLaunchError::Validation {
            message: String::from("Program is empty."),
        });
    }

    if let Some(dir) = custom.working_directory.as_deref()
        && dir.trim().is_empty()
    {
        return Err(QuickLaunchError::Validation {
            message: String::from("Working directory is empty."),
        });
    }

    Ok(())
}

/// Validate an SSH command before runtime preflight.
pub(crate) fn validate_ssh_command(
    ssh: &SshCommand,
) -> Result<(), QuickLaunchError> {
    if ssh.host.trim().is_empty() {
        return Err(QuickLaunchError::Validation {
            message: String::from("Host is empty."),
        });
    }

    if ssh.port == 0 {
        return Err(QuickLaunchError::Validation {
            message: String::from("Port must be greater than 0."),
        });
    }

    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_empty_custom_program_when_validating_then_returns_error() {
        let custom = CustomCommand {
            program: String::from(" "),
            args: Vec::new(),
            env: Vec::new(),
            working_directory: None,
        };

        let result = validate_custom_command(&custom);

        assert!(matches!(
            result,
            Err(QuickLaunchError::Validation { message })
            if message == "Program is empty."
        ));
    }

    #[test]
    fn given_empty_ssh_host_when_validating_then_returns_error() {
        let ssh = SshCommand {
            host: String::from(""),
            port: SSH_DEFAULT_PORT,
            user: None,
            identity_file: None,
            extra_args: Vec::new(),
        };

        let result = validate_ssh_command(&ssh);

        assert!(matches!(
            result,
            Err(QuickLaunchError::Validation { message })
            if message == "Host is empty."
        ));
    }

    #[test]
    fn given_valid_commands_when_validating_then_returns_ok() {
        let custom = CustomCommand {
            program: String::from("bash"),
            args: Vec::new(),
            env: Vec::new(),
            working_directory: Some(String::from("/tmp")),
        };
        let ssh = SshCommand {
            host: String::from("example.com"),
            port: SSH_DEFAULT_PORT,
            user: None,
            identity_file: None,
            extra_args: Vec::new(),
        };

        assert!(validate_custom_command(&custom).is_ok());
        assert!(validate_ssh_command(&ssh).is_ok());
    }
}
