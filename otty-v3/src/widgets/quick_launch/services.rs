use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use super::errors::QuickLaunchError;
use super::model::{
    CommandSpec, NodePath, PreparedQuickLaunch, QuickLaunch,
    QuickLaunchSetupOutcome,
};

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

    let title = command.title().to_string();
    QuickLaunchSetupOutcome::Prepared(PreparedQuickLaunch {
        path,
        launch_id,
        title,
        settings: terminal_settings,
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
        },
    }
    Ok(())
}

#[cfg(test)]
mod tests {
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
}
