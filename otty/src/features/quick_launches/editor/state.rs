use crate::features::quick_launches::model::{
    CommandSpec, EnvVar, NodePath, QuickLaunch, QuickLaunchType,
    SSH_DEFAULT_PORT,
};

/// Mode for a quick launch editor tab.
#[derive(Debug, Clone)]
pub(crate) enum QuickLaunchEditorMode {
    Create { parent_path: NodePath },
    Edit { path: NodePath },
}

/// Launch options for a local command.
#[derive(Debug, Clone, Default)]
pub(crate) struct CommandLaunchOptions {
    program: String,
    args: Vec<String>,
    env: Vec<(String, String)>,
    working_directory: String,
}

impl CommandLaunchOptions {
    /// Program executable path.
    pub(crate) fn program(&self) -> &str {
        &self.program
    }

    /// Program argument list.
    pub(crate) fn args(&self) -> &[String] {
        &self.args
    }

    /// Environment key/value entries.
    pub(crate) fn env(&self) -> &[(String, String)] {
        &self.env
    }

    /// Working directory value.
    pub(crate) fn working_directory(&self) -> &str {
        &self.working_directory
    }

    pub(crate) fn set_program(&mut self, value: String) {
        self.program = value;
    }

    pub(crate) fn set_working_directory(&mut self, value: String) {
        self.working_directory = value;
    }

    pub(crate) fn add_arg(&mut self) {
        self.args.push(String::new());
    }

    pub(crate) fn remove_arg(&mut self, index: usize) {
        if index < self.args.len() {
            self.args.remove(index);
        }
    }

    pub(crate) fn update_arg(&mut self, index: usize, value: String) {
        if let Some(arg) = self.args.get_mut(index) {
            *arg = value;
        }
    }

    pub(crate) fn add_env(&mut self) {
        self.env.push((String::new(), String::new()));
    }

    pub(crate) fn remove_env(&mut self, index: usize) {
        if index < self.env.len() {
            self.env.remove(index);
        }
    }

    pub(crate) fn update_env_key(&mut self, index: usize, value: String) {
        if let Some(pair) = self.env.get_mut(index) {
            pair.0 = value;
        }
    }

    pub(crate) fn update_env_value(&mut self, index: usize, value: String) {
        if let Some(pair) = self.env.get_mut(index) {
            pair.1 = value;
        }
    }
}

/// Launch options for an SSH command.
#[derive(Debug, Clone)]
pub(crate) struct SshLaunchOptions {
    host: String,
    port: String,
    user: String,
    identity_file: String,
    extra_args: Vec<String>,
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

impl SshLaunchOptions {
    /// SSH host.
    pub(crate) fn host(&self) -> &str {
        &self.host
    }

    /// SSH port.
    pub(crate) fn port(&self) -> &str {
        &self.port
    }

    /// SSH user.
    pub(crate) fn user(&self) -> &str {
        &self.user
    }

    /// SSH identity file path.
    pub(crate) fn identity_file(&self) -> &str {
        &self.identity_file
    }

    /// Extra SSH arguments.
    pub(crate) fn extra_args(&self) -> &[String] {
        &self.extra_args
    }

    pub(crate) fn set_host(&mut self, value: String) {
        self.host = value;
    }

    pub(crate) fn set_port(&mut self, value: String) {
        self.port = value;
    }

    pub(crate) fn set_user(&mut self, value: String) {
        self.user = value;
    }

    pub(crate) fn set_identity_file(&mut self, value: String) {
        self.identity_file = value;
    }

    pub(crate) fn add_extra_arg(&mut self) {
        self.extra_args.push(String::new());
    }

    pub(crate) fn remove_extra_arg(&mut self, index: usize) {
        if index < self.extra_args.len() {
            self.extra_args.remove(index);
        }
    }

    pub(crate) fn update_extra_arg(&mut self, index: usize, value: String) {
        if let Some(arg) = self.extra_args.get_mut(index) {
            *arg = value;
        }
    }
}

/// Active editor options for the selected command type.
#[derive(Debug, Clone)]
enum QuickLaunchEditorOptions {
    Custom(CommandLaunchOptions),
    Ssh(SshLaunchOptions),
}

impl QuickLaunchEditorOptions {
    fn command_type(&self) -> QuickLaunchType {
        match self {
            Self::Custom(_) => QuickLaunchType::Custom,
            Self::Ssh(_) => QuickLaunchType::Ssh,
        }
    }
}

/// Runtime state for a quick launch editor tab.
#[derive(Debug, Clone)]
pub(crate) struct QuickLaunchEditorState {
    mode: QuickLaunchEditorMode,
    title: String,
    options: QuickLaunchEditorOptions,
    error: Option<String>,
}

impl QuickLaunchEditorState {
    /// Build state for creating a command in the target folder.
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

    /// Build state from an existing command.
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

    /// Current editor mode.
    pub(crate) fn mode(&self) -> &QuickLaunchEditorMode {
        &self.mode
    }

    /// Current command title.
    pub(crate) fn title(&self) -> &str {
        &self.title
    }

    /// Visible editor error message.
    pub(crate) fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Active command type.
    pub(crate) fn command_type(&self) -> QuickLaunchType {
        self.options.command_type()
    }

    /// Return true when editor is in create mode.
    pub(crate) fn is_create_mode(&self) -> bool {
        matches!(self.mode, QuickLaunchEditorMode::Create { .. })
    }

    pub(crate) fn set_title(&mut self, value: String) {
        self.title = value;
    }

    pub(crate) fn set_error(&mut self, value: String) {
        self.error = Some(value);
    }

    pub(crate) fn clear_error(&mut self) {
        self.error = None;
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

    /// Access current SSH command options.
    pub(crate) fn ssh(&self) -> Option<&SshLaunchOptions> {
        match &self.options {
            QuickLaunchEditorOptions::Ssh(ssh) => Some(ssh),
            _ => None,
        }
    }

    pub(crate) fn set_program(&mut self, value: String) {
        if let Some(custom) = self.custom_mut() {
            custom.set_program(value);
        }
    }

    pub(crate) fn set_working_directory(&mut self, value: String) {
        if let Some(custom) = self.custom_mut() {
            custom.set_working_directory(value);
        }
    }

    pub(crate) fn add_arg(&mut self) {
        if let Some(custom) = self.custom_mut() {
            custom.add_arg();
        }
    }

    pub(crate) fn remove_arg(&mut self, index: usize) {
        if let Some(custom) = self.custom_mut() {
            custom.remove_arg(index);
        }
    }

    pub(crate) fn update_arg(&mut self, index: usize, value: String) {
        if let Some(custom) = self.custom_mut() {
            custom.update_arg(index, value);
        }
    }

    pub(crate) fn add_env(&mut self) {
        if let Some(custom) = self.custom_mut() {
            custom.add_env();
        }
    }

    pub(crate) fn remove_env(&mut self, index: usize) {
        if let Some(custom) = self.custom_mut() {
            custom.remove_env(index);
        }
    }

    pub(crate) fn update_env_key(&mut self, index: usize, value: String) {
        if let Some(custom) = self.custom_mut() {
            custom.update_env_key(index, value);
        }
    }

    pub(crate) fn update_env_value(&mut self, index: usize, value: String) {
        if let Some(custom) = self.custom_mut() {
            custom.update_env_value(index, value);
        }
    }

    pub(crate) fn set_host(&mut self, value: String) {
        if let Some(ssh) = self.ssh_mut() {
            ssh.set_host(value);
        }
    }

    pub(crate) fn set_user(&mut self, value: String) {
        if let Some(ssh) = self.ssh_mut() {
            ssh.set_user(value);
        }
    }

    pub(crate) fn set_port(&mut self, value: String) {
        if let Some(ssh) = self.ssh_mut() {
            ssh.set_port(value);
        }
    }

    pub(crate) fn set_identity_file(&mut self, value: String) {
        if let Some(ssh) = self.ssh_mut() {
            ssh.set_identity_file(value);
        }
    }

    pub(crate) fn add_extra_arg(&mut self) {
        if let Some(ssh) = self.ssh_mut() {
            ssh.add_extra_arg();
        }
    }

    pub(crate) fn remove_extra_arg(&mut self, index: usize) {
        if let Some(ssh) = self.ssh_mut() {
            ssh.remove_extra_arg(index);
        }
    }

    pub(crate) fn update_extra_arg(&mut self, index: usize, value: String) {
        if let Some(ssh) = self.ssh_mut() {
            ssh.update_extra_arg(index, value);
        }
    }

    fn custom_mut(&mut self) -> Option<&mut CommandLaunchOptions> {
        match &mut self.options {
            QuickLaunchEditorOptions::Custom(custom) => Some(custom),
            _ => None,
        }
    }

    fn ssh_mut(&mut self) -> Option<&mut SshLaunchOptions> {
        match &mut self.options {
            QuickLaunchEditorOptions::Ssh(ssh) => Some(ssh),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_create_editor_when_switching_command_type_then_options_reset() {
        let mut editor = QuickLaunchEditorState::new_create(vec![]);
        editor.set_program(String::from("bash"));

        editor.set_command_type(QuickLaunchType::Ssh);

        assert_eq!(editor.command_type(), QuickLaunchType::Ssh);
        assert!(editor.custom().is_none());
        let ssh = editor.ssh().expect("ssh options should exist");
        assert_eq!(ssh.port(), SSH_DEFAULT_PORT.to_string());
    }

    #[test]
    fn given_custom_editor_when_mutating_fields_then_custom_values_are_updated()
    {
        let mut editor = QuickLaunchEditorState::new_create(vec![]);

        editor.set_title(String::from("Demo"));
        editor.set_program(String::from("/bin/echo"));
        editor.add_arg();
        editor.update_arg(0, String::from("hello"));
        editor.add_env();
        editor.update_env_key(0, String::from("KEY"));
        editor.update_env_value(0, String::from("value"));
        editor.set_working_directory(String::from("/tmp"));

        assert_eq!(editor.title(), "Demo");
        let custom = editor.custom().expect("custom options should exist");
        assert_eq!(custom.program(), "/bin/echo");
        assert_eq!(custom.args(), &[String::from("hello")]);
        assert_eq!(
            custom.env(),
            &[(String::from("KEY"), String::from("value"))],
        );
        assert_eq!(custom.working_directory(), "/tmp");
    }

    #[test]
    fn given_ssh_editor_when_mutating_fields_then_ssh_values_are_updated() {
        let mut editor = QuickLaunchEditorState::new_create(vec![]);
        editor.set_command_type(QuickLaunchType::Ssh);

        editor.set_host(String::from("example.com"));
        editor.set_port(String::from("2222"));
        editor.set_user(String::from("ubuntu"));
        editor.set_identity_file(String::from("~/.ssh/id_ed25519"));
        editor.add_extra_arg();
        editor.update_extra_arg(0, String::from("-A"));

        let ssh = editor.ssh().expect("ssh options should exist");
        assert_eq!(ssh.host(), "example.com");
        assert_eq!(ssh.port(), "2222");
        assert_eq!(ssh.user(), "ubuntu");
        assert_eq!(ssh.identity_file(), "~/.ssh/id_ed25519");
        assert_eq!(ssh.extra_args(), &[String::from("-A")]);
    }
}
