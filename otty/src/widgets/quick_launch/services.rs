use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use otty_libterm::pty::{self, SSHAuth, Session};
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

    if let Err(error) = validate_command(&command) {
        return QuickLaunchSetupOutcome::Failed {
            path,
            launch_id,
            command: Box::new(command),
            error: Arc::new(error),
        };
    }

    if cancel.load(Ordering::Relaxed) {
        return QuickLaunchSetupOutcome::Canceled { path, launch_id };
    }

    if let Err(error) = probe_launch_runtime(&command, &cancel) {
        return QuickLaunchSetupOutcome::Failed {
            path,
            launch_id,
            command: Box::new(command),
            error: Arc::new(error),
        };
    }

    let session = command_session(&command, &cancel);
    let settings = terminal_settings_for_session(&terminal_settings, session);
    let title = command.title().to_string();

    if cancel.load(Ordering::Relaxed) {
        return QuickLaunchSetupOutcome::Canceled { path, launch_id };
    }

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
    match command.spec() {
        CommandSpec::Custom { custom } => {
            if custom.program().trim().is_empty() {
                return Err(QuickLaunchError::Validation {
                    message: String::from("Program path is empty."),
                });
            }

            validate_custom_runtime(custom)?;
        },
        CommandSpec::Ssh { ssh } => {
            if ssh.host().trim().is_empty() {
                return Err(QuickLaunchError::Validation {
                    message: String::from("SSH host is empty."),
                });
            }
            if ssh.port() == 0 {
                return Err(QuickLaunchError::Validation {
                    message: String::from("SSH port must be greater than 0."),
                });
            }

            validate_ssh_runtime(ssh)?;
        },
    }
    Ok(())
}

fn probe_launch_runtime(
    command: &QuickLaunch,
    cancel: &Arc<AtomicBool>,
) -> Result<(), QuickLaunchError> {
    match command.spec() {
        CommandSpec::Custom { .. } => Ok(()),
        CommandSpec::Ssh { ssh } => probe_ssh_session(ssh, cancel),
    }
}

fn probe_ssh_session(
    ssh: &super::model::SshCommand,
    cancel: &Arc<AtomicBool>,
) -> Result<(), QuickLaunchError> {
    let options = ssh_session(ssh, cancel);
    let mut builder = pty::ssh()
        .with_host(options.host())
        .with_user(options.user())
        .with_auth(options.auth());

    if let Some(timeout) = options.timeout() {
        builder = builder.with_timeout(timeout);
    }

    if let Some(cancel_token) = options.cancel_token() {
        builder = builder.with_cancel_token(cancel_token.clone());
    }

    let mut session =
        builder
            .spawn()
            .map_err(|err| QuickLaunchError::Validation {
                message: format!("SSH connection failed: {err}"),
            })?;
    let _ = session.close();
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
        .filter(|value| !value.trim().is_empty())
        .map(ToString::to_string)
        .or_else(|| std::env::var("USER").ok())
        .or_else(|| std::env::var("USERNAME").ok())
        .unwrap_or_default();

    let auth = ssh
        .identity_file()
        .filter(|path| !path.trim().is_empty())
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

fn validate_custom_runtime(
    custom: &super::model::CustomCommand,
) -> Result<(), QuickLaunchError> {
    let program = custom.program().trim();
    let _ = find_program_path(program)?;

    if let Some(dir) = custom.working_directory() {
        let expanded = expand_tilde(dir);
        let path = Path::new(&expanded);

        if !path.exists() {
            return Err(QuickLaunchError::Validation {
                message: format!("Working directory not found: {expanded}"),
            });
        }

        if !path.is_dir() {
            return Err(QuickLaunchError::Validation {
                message: format!(
                    "Working directory is not a directory: {expanded}"
                ),
            });
        }
    }

    Ok(())
}

fn validate_ssh_runtime(
    ssh: &super::model::SshCommand,
) -> Result<(), QuickLaunchError> {
    if let Some(identity) = ssh.identity_file() {
        let identity = identity.trim();
        if !identity.is_empty() {
            let expanded = expand_tilde(identity);
            let path = Path::new(&expanded);

            if !path.exists() {
                return Err(QuickLaunchError::Validation {
                    message: format!("Identity file not found: {expanded}"),
                });
            }

            if !path.is_file() {
                return Err(QuickLaunchError::Validation {
                    message: format!("Identity file is not a file: {expanded}"),
                });
            }
        }
    }

    Ok(())
}

fn find_program_path(program: &str) -> Result<PathBuf, QuickLaunchError> {
    let program = program.trim();
    let has_separator = program.contains('/') || program.contains('\\');
    let is_explicit = has_separator || program.starts_with('~');

    if is_explicit {
        let expanded = expand_tilde(program);
        let path = PathBuf::from(&expanded);
        return validate_program_path(&path, &expanded);
    }

    let paths: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|value| std::env::split_paths(&value).collect())
        .unwrap_or_default();

    for dir in paths {
        let candidate = dir.join(program);
        if is_executable_path(&candidate) {
            return Ok(candidate);
        }
    }

    Err(QuickLaunchError::Validation {
        message: format!("Program not found in PATH: {program}"),
    })
}

fn validate_program_path(
    path: &Path,
    label: &str,
) -> Result<PathBuf, QuickLaunchError> {
    if !path.exists() {
        return Err(QuickLaunchError::Validation {
            message: format!("Program not found: {label}"),
        });
    }

    if path.is_dir() {
        return Err(QuickLaunchError::Validation {
            message: format!("Program is a directory: {label}"),
        });
    }

    if !is_executable_path(path) {
        return Err(QuickLaunchError::Validation {
            message: format!("Program is not executable: {label}"),
        });
    }

    Ok(path.to_path_buf())
}

fn is_executable_path(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(path)
            .map(|meta| meta.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }

    #[cfg(not(unix))]
    {
        true
    }
}

fn expand_tilde(path: &str) -> String {
    if path == "~" {
        return std::env::var("HOME").unwrap_or_else(|_| String::from("~"));
    }

    if let Some(rest) = path.strip_prefix("~/")
        && let Ok(home) = std::env::var("HOME")
    {
        return format!("{home}/{rest}");
    }

    path.to_string()
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
    fn given_missing_program_when_validating_then_error_returned() {
        let cmd = QuickLaunch {
            title: String::from("Missing"),
            spec: CommandSpec::Custom {
                custom: CustomCommand {
                    program: String::from("otty-v3-test-bin-does-not-exist"),
                    args: Vec::new(),
                    env: Vec::new(),
                    working_directory: None,
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
