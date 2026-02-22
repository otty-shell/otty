use crate::features::quick_launches::model::{
    CommandSpec, EnvVar, NodePath, QuickLaunch, QuickLaunchType,
    SSH_DEFAULT_PORT,
};

/// Mode for the editor tab.
#[derive(Debug, Clone)]
pub(crate) enum QuickLaunchEditorMode {
    Create { parent_path: NodePath },
    Edit { path: NodePath },
}

/// Launch options for a local command.
#[derive(Debug, Clone, Default)]
pub(crate) struct CommandLaunchOptions {
    pub(crate) program: String,
    pub(crate) args: Vec<String>,
    pub(crate) env: Vec<(String, String)>,
    pub(crate) working_directory: String,
}

/// Launch options for an SSH command.
#[derive(Debug, Clone)]
pub(crate) struct SshLaunchOptions {
    pub(crate) host: String,
    pub(crate) port: String,
    pub(crate) user: String,
    pub(crate) identity_file: String,
    pub(crate) extra_args: Vec<String>,
}

impl Default for SshLaunchOptions {
    fn default() -> Self {
        Self {
            host: String::new(),
            port: SSH_DEFAULT_PORT.to_string(),
            user: String::new(),
            identity_file: String::new(),
            extra_args: Vec::new(),
        }
    }
}

/// Active editor options for the selected command type.
#[derive(Debug, Clone)]
pub(crate) enum QuickLaunchEditorOptions {
    Custom(CommandLaunchOptions),
    Ssh(SshLaunchOptions),
}

impl QuickLaunchEditorOptions {
    fn command_type(&self) -> QuickLaunchType {
        match self {
            QuickLaunchEditorOptions::Custom(_) => QuickLaunchType::Custom,
            QuickLaunchEditorOptions::Ssh(_) => QuickLaunchType::Ssh,
        }
    }
}

/// Editor state stored in a tab.
#[derive(Debug, Clone)]
pub(crate) struct QuickLaunchEditorState {
    pub(crate) mode: QuickLaunchEditorMode,
    pub(crate) title: String,
    options: QuickLaunchEditorOptions,
    pub(crate) error: Option<String>,
}

impl QuickLaunchEditorState {
    /// Build a state for creating a command in the target folder.
    pub(crate) fn new_create(parent_path: NodePath) -> Self {
        Self {
            mode: QuickLaunchEditorMode::Create { parent_path },
            title: String::new(),
            options: QuickLaunchEditorOptions::Custom(
                CommandLaunchOptions::default(),
            ),
            error: None,
        }
    }

    /// Build an editor state from an existing command.
    pub(crate) fn from_command(path: NodePath, command: &QuickLaunch) -> Self {
        match &command.spec {
            CommandSpec::Custom { custom } => Self {
                mode: QuickLaunchEditorMode::Edit { path },
                title: command.title.clone(),
                options: QuickLaunchEditorOptions::Custom(
                    CommandLaunchOptions {
                        program: custom.program.clone(),
                        args: custom.args.clone(),
                        env: custom
                            .env
                            .iter()
                            .map(|EnvVar { key, value }| {
                                (key.clone(), value.clone())
                            })
                            .collect(),
                        working_directory: custom
                            .working_directory
                            .clone()
                            .unwrap_or_default(),
                    },
                ),
                error: None,
            },
            CommandSpec::Ssh { ssh } => Self {
                mode: QuickLaunchEditorMode::Edit { path },
                title: command.title.clone(),
                options: QuickLaunchEditorOptions::Ssh(SshLaunchOptions {
                    host: ssh.host.clone(),
                    port: ssh.port.to_string(),
                    user: ssh.user.clone().unwrap_or_default(),
                    identity_file: ssh
                        .identity_file
                        .clone()
                        .unwrap_or_default(),
                    extra_args: ssh.extra_args.clone(),
                }),
                error: None,
            },
        }
    }

    /// Return the active command type.
    pub(crate) fn command_type(&self) -> QuickLaunchType {
        self.options.command_type()
    }

    /// Change active command type for create mode.
    pub(crate) fn set_command_type(&mut self, command_type: QuickLaunchType) {
        if self.command_type() == command_type {
            return;
        }

        self.options = match command_type {
            QuickLaunchType::Custom => {
                QuickLaunchEditorOptions::Custom(CommandLaunchOptions::default())
            },
            QuickLaunchType::Ssh => {
                QuickLaunchEditorOptions::Ssh(SshLaunchOptions::default())
            },
        };
    }

    /// Access current custom command options.
    pub(crate) fn custom(&self) -> Option<&CommandLaunchOptions> {
        match &self.options {
            QuickLaunchEditorOptions::Custom(custom) => Some(custom),
            _ => None,
        }
    }

    /// Access current custom command options mutably.
    pub(crate) fn custom_mut(&mut self) -> Option<&mut CommandLaunchOptions> {
        match &mut self.options {
            QuickLaunchEditorOptions::Custom(custom) => Some(custom),
            _ => None,
        }
    }

    /// Access current SSH command options.
    pub(crate) fn ssh(&self) -> Option<&SshLaunchOptions> {
        match &self.options {
            QuickLaunchEditorOptions::Ssh(ssh) => Some(ssh),
            _ => None,
        }
    }

    /// Access current SSH command options mutably.
    pub(crate) fn ssh_mut(&mut self) -> Option<&mut SshLaunchOptions> {
        match &mut self.options {
            QuickLaunchEditorOptions::Ssh(ssh) => Some(ssh),
            _ => None,
        }
    }
}
