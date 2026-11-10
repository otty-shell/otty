//! A saved launch and its place in the launch tree, as the launches widget
//! and the tabs that edit a launch both see it.

use serde::{Deserialize, Serialize};

use crate::exit_behavior::ExitBehavior;

/// Path to a node in the quick launch tree.
pub type NodePath = Vec<String>;

/// A saved quick launch command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickLaunch {
    title: String,
    spec: CommandSpec,
    /// Files saved before this setting existed load as `Close`.
    #[serde(default)]
    exit_behavior: ExitBehavior,
}

impl QuickLaunch {
    /// Command title.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Command specification.
    pub fn spec(&self) -> &CommandSpec {
        &self.spec
    }

    /// What the command tab does once the process exits.
    pub fn exit_behavior(&self) -> ExitBehavior {
        self.exit_behavior
    }

    /// Whether the command connects to a remote host over SSH.
    pub fn is_ssh(&self) -> bool {
        matches!(self.spec, CommandSpec::Ssh { .. })
    }

    /// Host the command connects to, or `None` for a local command.
    pub fn ssh_host(&self) -> Option<&str> {
        match &self.spec {
            CommandSpec::Ssh { ssh } => Some(ssh.host()),
            _ => None,
        }
    }

    /// `user@host` of an SSH command (just `host` without a user), or `None`
    /// for a local command.
    pub fn ssh_account(&self) -> Option<String> {
        match &self.spec {
            CommandSpec::Ssh { ssh } => Some(match ssh.user() {
                Some(user) => format!("{user}@{}", ssh.host()),
                None => ssh.host().to_string(),
            }),
            _ => None,
        }
    }

    /// A launch named `title` that runs `spec`.
    pub fn new(
        title: String,
        spec: CommandSpec,
        exit_behavior: ExitBehavior,
    ) -> Self {
        Self {
            title,
            spec,
            exit_behavior,
        }
    }

    /// Give the launch a new title.
    pub fn rename(&mut self, title: String) {
        self.title = title;
    }
}

/// Command specification variant.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CommandSpec {
    /// A program run on this machine.
    Custom { custom: CustomCommand },
    /// A connection to a remote host over SSH.
    Ssh { ssh: SshCommand },
}

/// Local command specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomCommand {
    program: String,
    args: Vec<String>,
    env: Vec<EnvVar>,
    working_directory: Option<String>,
}

impl CustomCommand {
    /// Program to run, as a name on `PATH` or a path.
    pub fn program(&self) -> &str {
        &self.program
    }

    /// Arguments passed to the program.
    pub fn args(&self) -> &[String] {
        &self.args
    }

    /// Environment variables set for the program.
    pub fn env(&self) -> &[EnvVar] {
        &self.env
    }

    /// Folder the program starts in, or `None` for the default one.
    pub fn working_directory(&self) -> Option<&str> {
        self.working_directory.as_deref()
    }

    /// A local command that runs `program` with `args` and `env`, in
    /// `working_directory` when one is given.
    pub fn new(
        program: String,
        args: Vec<String>,
        env: Vec<EnvVar>,
        working_directory: Option<String>,
    ) -> Self {
        Self {
            program,
            args,
            env,
            working_directory,
        }
    }
}

/// Environment variable key-value pair.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvVar {
    key: String,
    value: String,
}

impl EnvVar {
    /// Variable name.
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Variable value.
    pub fn value(&self) -> &str {
        &self.value
    }

    /// The variable `key` set to `value`.
    pub fn new(key: String, value: String) -> Self {
        Self { key, value }
    }
}

/// SSH command specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshCommand {
    host: String,
    port: u16,
    user: Option<String>,
    identity_file: Option<String>,
    extra_args: Vec<String>,
}

impl SshCommand {
    /// Host to connect to.
    pub fn host(&self) -> &str {
        &self.host
    }

    /// SSH port of the host.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// User to log in as, or `None` for the SSH default.
    pub fn user(&self) -> Option<&str> {
        self.user.as_deref()
    }

    /// Private key file to log in with, or `None` for the SSH default.
    pub fn identity_file(&self) -> Option<&str> {
        self.identity_file.as_deref()
    }

    /// Arguments passed to `ssh` after the connection options.
    pub fn extra_args(&self) -> &[String] {
        &self.extra_args
    }

    /// A connection to `host` on `port`, as `user` and with
    /// `identity_file` when they are given.
    pub fn new(
        host: String,
        port: u16,
        user: Option<String>,
        identity_file: Option<String>,
        extra_args: Vec<String>,
    ) -> Self {
        Self {
            host,
            port,
            user,
            identity_file,
            extra_args,
        }
    }
}

/// What the launch editor starts with when its tab opens: a new launch in a
/// folder of the tree, or a saved launch to edit. The quick launch feature
/// asks for the tab with it, and the tabs feature hands it back once the tab
/// is open, so the editor fills the right tab.
///
/// # Examples
///
/// ```
/// use otty_app_domain::quick_launch::WizardTabInit;
///
/// let init = WizardTabInit::Create {
///     parent_path: vec![String::from("Servers")],
/// };
///
/// assert!(matches!(init, WizardTabInit::Create { .. }));
/// ```
#[derive(Debug, Clone)]
pub enum WizardTabInit {
    /// Create a new launch in the folder at `parent_path`.
    Create {
        /// Folder of the tree the new launch goes in; empty for the top.
        parent_path: NodePath,
    },
    /// Edit the saved launch at `path`.
    Edit {
        /// Place of the launch in the tree.
        path: NodePath,
        /// The launch as it is saved.
        command: Box<QuickLaunch>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn local_command(title: &str) -> QuickLaunch {
        QuickLaunch::new(
            String::from(title),
            CommandSpec::Custom {
                custom: CustomCommand::new(
                    String::from("bash"),
                    Vec::new(),
                    Vec::new(),
                    None,
                ),
            },
            ExitBehavior::Close,
        )
    }

    fn ssh_command(title: &str, user: Option<&str>) -> QuickLaunch {
        QuickLaunch::new(
            String::from(title),
            CommandSpec::Ssh {
                ssh: SshCommand::new(
                    String::from("example.com"),
                    22,
                    user.map(String::from),
                    None,
                    Vec::new(),
                ),
            },
            ExitBehavior::Close,
        )
    }

    #[test]
    fn given_saved_command_without_exit_behavior_when_loaded_then_tab_closes_on_exit()
     {
        let json = serde_json::json!({
            "title": "List",
            "spec": {
                "type": "Custom",
                "custom": {
                    "program": "ls",
                    "args": ["-la"],
                    "env": [],
                    "working_directory": null
                }
            }
        });

        let command: QuickLaunch =
            serde_json::from_value(json).expect("should deserialize");

        assert_eq!(command.exit_behavior(), ExitBehavior::Close);
    }

    #[test]
    fn given_command_with_exit_behavior_when_round_tripped_then_choice_is_kept()
    {
        let command = QuickLaunch::new(
            String::from("List"),
            CommandSpec::Custom {
                custom: CustomCommand::new(
                    String::from("ls"),
                    vec![String::from("-la")],
                    Vec::new(),
                    None,
                ),
            },
            ExitBehavior::KeepOpen,
        );

        let json = serde_json::to_value(&command).expect("should serialize");
        let parsed: QuickLaunch =
            serde_json::from_value(json.clone()).expect("should deserialize");

        assert_eq!(json["exit_behavior"], "keep_open");
        assert_eq!(parsed.exit_behavior(), ExitBehavior::KeepOpen);
    }

    #[test]
    fn given_ssh_and_custom_commands_when_checked_then_only_ssh_is_ssh() {
        let ssh = ssh_command("prod", None);
        let local = local_command("shell");

        assert!(ssh.is_ssh());
        assert!(!local.is_ssh());
    }

    #[test]
    fn given_ssh_and_custom_commands_when_host_read_then_only_ssh_has_one() {
        let ssh = ssh_command("prod", None);
        let local = local_command("shell");

        assert_eq!(ssh.ssh_host(), Some("example.com"));
        assert_eq!(local.ssh_host(), None);
    }

    #[test]
    fn given_ssh_command_when_account_read_then_it_names_the_user_if_any() {
        let with_user = ssh_command("prod", Some("ubuntu"));
        let without_user = ssh_command("prod", None);
        let local = local_command("shell");

        assert_eq!(
            with_user.ssh_account().as_deref(),
            Some("ubuntu@example.com")
        );
        assert_eq!(without_user.ssh_account().as_deref(), Some("example.com"));
        assert_eq!(local.ssh_account(), None);
    }

    #[test]
    fn given_launch_when_renamed_then_only_its_title_changes() {
        let mut launch = ssh_command("prod", Some("ubuntu"));

        launch.rename(String::from("production"));

        assert_eq!(launch.title(), "production");
        assert_eq!(launch.ssh_host(), Some("example.com"));
    }
}
