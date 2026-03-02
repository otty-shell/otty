use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use otty_libterm::pty::SSHAuth;
use otty_ui_term::settings::{
    LocalSessionOptions, SSHSessionOptions, SessionKind,
};

use super::errors::QuickLaunchError;
use super::model::{
    CommandSpec, NodePath, PreparedQuickLaunch, QuickLaunch,
    QuickLaunchSetupOutcome,
};
use crate::widgets::terminal_workspace::services::terminal_settings_for_session;

const QUICK_LAUNCH_SSH_TIMEOUT: Duration = Duration::from_secs(15);

/// Prepare a quick launch command asynchronously.
///
/// Validates the command, resolves paths, and returns a prepared launch
/// or an error/canceled outcome.
pub(crate) async fn prepare_quick_launch_setup(
    command: QuickLaunch,
    path: NodePath,
    launch_id: u64,
    terminal_settings: otty_ui_term::settings::Settings,
    cancel: Arc<AtomicBool>,
) -> QuickLaunchSetupOutcome {
    if cancel.load(Ordering::Relaxed) {
        return QuickLaunchSetupOutcome::Canceled { path, launch_id };
    }

    match validate_command(&command) {
        Ok(()) => {},
        Err(err) => {
            return QuickLaunchSetupOutcome::Failed {
                path,
                launch_id,
                command: Box::new(command),
                error: Arc::new(err),
            };
        },
    }

    if cancel.load(Ordering::Relaxed) {
        return QuickLaunchSetupOutcome::Canceled { path, launch_id };
    }

    let session = command_session(&command, &cancel);
    let settings = terminal_settings_for_session(&terminal_settings, session);
    let title = command.title().to_string();
    QuickLaunchSetupOutcome::Prepared(PreparedQuickLaunch {
        path,
        launch_id,
        title,
        settings,
        command: Box::new(command),
    })
}

/// Validate a quick launch command before execution.
fn validate_command(command: &QuickLaunch) -> Result<(), QuickLaunchError> {
    match &command.spec {
        CommandSpec::Custom { custom } => {
            if custom.program.trim().is_empty() {
                return Err(QuickLaunchError::Validation {
                    message: String::from("Program path is empty."),
                });
            }
        },
        CommandSpec::Ssh { ssh } => {
            if ssh.host.trim().is_empty() {
                return Err(QuickLaunchError::Validation {
                    message: String::from("SSH host is empty."),
                });
            }
            if ssh.port == 0 {
                return Err(QuickLaunchError::Validation {
                    message: String::from("SSH port must be greater than 0."),
                });
            }
        },
    }
    Ok(())
}

fn command_session(
    command: &QuickLaunch,
    cancel: &Arc<AtomicBool>,
) -> SessionKind {
    match command.spec() {
        CommandSpec::Custom { custom } => {
            SessionKind::from_local_options(custom_session(custom))
        },
        CommandSpec::Ssh { ssh } => {
            SessionKind::from_ssh_options(ssh_session(ssh, cancel))
        },
    }
}

fn custom_session(custom: &super::model::CustomCommand) -> LocalSessionOptions {
    let mut options = LocalSessionOptions::default()
        .with_program(custom.program())
        .with_args(custom.args().to_vec());

    for env in custom.env() {
        options = options.with_env(env.key(), env.value());
    }

    if let Some(dir) = custom.working_directory() {
        options = options.with_working_directory(PathBuf::from(dir));
    }

    options
}

fn ssh_session(
    ssh: &super::model::SshCommand,
    cancel: &Arc<AtomicBool>,
) -> SSHSessionOptions {
    let host = format!("{}:{}", ssh.host(), ssh.port());
    let user = ssh
        .user()
        .map(ToString::to_string)
        .or_else(|| std::env::var("USER").ok())
        .or_else(|| std::env::var("USERNAME").ok())
        .unwrap_or_default();

    let auth = ssh
        .identity_file()
        .map(|path| SSHAuth::KeyFile {
            private_key_path: path.to_string(),
            passphrase: None,
        })
        .unwrap_or_else(|| SSHAuth::Password(String::new()));

    SSHSessionOptions::default()
        .with_host(&host)
        .with_user(&user)
        .with_auth(auth)
        .with_timeout(QUICK_LAUNCH_SSH_TIMEOUT)
        .with_cancel_token(cancel.clone())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;

    use super::*;
    use crate::widgets::quick_launch::model::{CustomCommand, SshCommand};

    #[test]
    fn given_empty_program_when_validating_then_error_returned() {
        let cmd = QuickLaunch {
            title: String::from("Bad"),
            spec: CommandSpec::Custom {
                custom: CustomCommand {
                    program: String::new(),
                    args: Vec::new(),
                    env: Vec::new(),
                    working_directory: None,
                },
            },
        };
        assert!(validate_command(&cmd).is_err());
    }

    #[test]
    fn given_valid_custom_command_when_validating_then_ok() {
        let cmd = QuickLaunch {
            title: String::from("Good"),
            spec: CommandSpec::Custom {
                custom: CustomCommand {
                    program: String::from("bash"),
                    args: Vec::new(),
                    env: Vec::new(),
                    working_directory: None,
                },
            },
        };
        assert!(validate_command(&cmd).is_ok());
    }

    #[test]
    fn given_empty_ssh_host_when_validating_then_error_returned() {
        let cmd = QuickLaunch {
            title: String::from("SSH"),
            spec: CommandSpec::Ssh {
                ssh: SshCommand {
                    host: String::new(),
                    port: 22,
                    user: None,
                    identity_file: None,
                    extra_args: Vec::new(),
                },
            },
        };
        assert!(validate_command(&cmd).is_err());
    }

    #[test]
    fn given_custom_command_when_prepared_then_settings_use_saved_program() {
        let command = QuickLaunch {
            title: String::from("Run htop"),
            spec: CommandSpec::Custom {
                custom: CustomCommand {
                    program: String::from("htop"),
                    args: vec![
                        String::from("--sort-key"),
                        String::from("PERCENT_CPU"),
                    ],
                    env: Vec::new(),
                    working_directory: None,
                },
            },
        };

        let cancel = Arc::new(AtomicBool::new(false));
        let session = command_session(&command, &cancel);
        let SessionKind::Local(options) = session else {
            panic!("expected local session");
        };
        assert_eq!(options.program(), "htop");
        assert_eq!(
            options.args(),
            &[String::from("--sort-key"), String::from("PERCENT_CPU")]
        );
    }
}
