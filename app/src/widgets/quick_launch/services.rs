use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use otty_libterm::pty::{self, SSHAuth, Session};
use otty_ui_term::settings::{
    LocalSessionOptions, SSHSessionOptions, SessionKind,
};

use super::constants::SSH_DEFAULT_PORT;
use super::errors::{QuickLaunchError, QuickLaunchWizardError};
use super::state::WizardEditorState;
use super::types::{
    CommandSpec, CustomCommand, EnvVar, NodePath, PreparedQuickLaunch,
    QuickLaunch, QuickLaunchSetupOutcome, QuickLaunchType, SshCommand,
};
use crate::services::terminal_settings_for_session;

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

    QuickLaunchSetupOutcome::Prepared(Box::new(PreparedQuickLaunch {
        path,
        launch_id,
        title,
        settings,
        command: Box::new(command),
    }))
}

/// Validate a quick launch command before execution.
fn validate_command(command: &QuickLaunch) -> Result<(), QuickLaunchError> {
    match command.spec() {
        CommandSpec::Custom { custom } => {
            if custom.program().trim().is_empty() {
                return Err(QuickLaunchError::ProgramRequired);
            }

            validate_custom_runtime(custom)?;
        },
        CommandSpec::Ssh { ssh } => {
            if ssh.host().trim().is_empty() {
                return Err(QuickLaunchError::HostRequired);
            }
            if ssh.port() == 0 {
                return Err(QuickLaunchError::SshPortNotPositive);
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
    ssh: &super::types::SshCommand,
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

    let mut session = builder.spawn().map_err(|err| {
        QuickLaunchError::SshConnectionFailed(format!("{err}"))
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

fn custom_session(custom: &super::types::CustomCommand) -> LocalSessionOptions {
    let mut options = LocalSessionOptions::default()
        .with_program(&expand_home_dir(custom.program()))
        .with_args(custom.args().to_vec());

    for env in custom.env() {
        options = options.with_env(env.key(), env.value());
    }

    if let Some(dir) = custom.working_directory() {
        options =
            options.with_working_directory(PathBuf::from(expand_home_dir(dir)));
    }

    options
}

fn ssh_session(
    ssh: &super::types::SshCommand,
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
            private_key_path: expand_home_dir(path),
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
    custom: &super::types::CustomCommand,
) -> Result<(), QuickLaunchError> {
    let program = custom.program().trim();
    let _ = find_program_path(program)?;

    if let Some(dir) = custom.working_directory() {
        let expanded = expand_home_dir(dir);
        let path = Path::new(&expanded);

        if !path.exists() {
            return Err(QuickLaunchError::WorkingDirectoryNotFound(expanded));
        }

        if !path.is_dir() {
            return Err(QuickLaunchError::WorkingDirectoryNotDirectory(
                expanded,
            ));
        }
    }

    Ok(())
}

fn validate_ssh_runtime(
    ssh: &super::types::SshCommand,
) -> Result<(), QuickLaunchError> {
    if let Some(identity) = ssh.identity_file() {
        let identity = identity.trim();
        if !identity.is_empty() {
            let expanded = expand_home_dir(identity);
            let path = Path::new(&expanded);

            if !path.exists() {
                return Err(QuickLaunchError::IdentityFileNotFound(expanded));
            }

            if !path.is_file() {
                return Err(QuickLaunchError::IdentityFileNotFile(expanded));
            }
        }
    }

    Ok(())
}

fn find_program_path(program: &str) -> Result<PathBuf, QuickLaunchError> {
    let program = expand_home_dir(program.trim());
    let is_explicit = program.contains('/') || program.contains('\\');

    if is_explicit {
        let path = PathBuf::from(&program);
        return validate_program_path(&path, &program);
    }

    let paths: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|value| std::env::split_paths(&value).collect())
        .unwrap_or_default();

    for dir in paths {
        let candidate = dir.join(&program);
        if is_executable_path(&candidate) {
            return Ok(candidate);
        }
    }

    Err(QuickLaunchError::ProgramNotFoundInPath(program))
}

fn validate_program_path(
    path: &Path,
    label: &str,
) -> Result<PathBuf, QuickLaunchError> {
    if !path.exists() {
        return Err(QuickLaunchError::ProgramNotFound(label.to_string()));
    }

    if path.is_dir() {
        return Err(QuickLaunchError::ProgramIsDirectory(label.to_string()));
    }

    if !is_executable_path(path) {
        return Err(QuickLaunchError::ProgramNotExecutable(label.to_string()));
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

/// Replace a leading `~`, `$HOME` or `${HOME}` with the user's home
/// directory, the way a shell would before running the command.
fn expand_home_dir(path: &str) -> String {
    let home = std::env::var("HOME").ok();
    expand_home_prefix(path, home.as_deref())
}

fn expand_home_prefix(path: &str, home: Option<&str>) -> String {
    let Some(home) = home else {
        return path.to_string();
    };

    for prefix in ["~", "$HOME", "${HOME}"] {
        let Some(rest) = path.strip_prefix(prefix) else {
            continue;
        };

        if rest.is_empty() || rest.starts_with('/') {
            return format!("{home}{rest}");
        }
    }

    path.to_string()
}

/// Build a domain quick launch command from editor draft state.
pub(crate) fn build_command(
    editor: &WizardEditorState,
) -> Result<QuickLaunch, QuickLaunchWizardError> {
    let title = editor.title().trim();
    if title.is_empty() {
        return Err(QuickLaunchWizardError::TitleRequired);
    }

    let spec = match editor.command_type() {
        QuickLaunchType::Custom => {
            let Some(custom) = editor.custom() else {
                return Err(QuickLaunchWizardError::MissingCustomDraft);
            };
            let program = custom.program().trim();
            if program.is_empty() {
                return Err(QuickLaunchWizardError::ProgramRequired);
            }

            let env = custom
                .env()
                .iter()
                .filter_map(|(key, value)| {
                    let key = key.trim();
                    if key.is_empty() {
                        return None;
                    }
                    Some(EnvVar {
                        key: key.to_string(),
                        value: value.clone(),
                    })
                })
                .collect::<Vec<_>>();

            let working_directory = custom.working_directory().trim();

            CommandSpec::Custom {
                custom: CustomCommand {
                    program: program.to_string(),
                    args: custom.args().to_vec(),
                    env,
                    working_directory: if working_directory.is_empty() {
                        None
                    } else {
                        Some(working_directory.to_string())
                    },
                },
            }
        },
        QuickLaunchType::Ssh => {
            let Some(ssh) = editor.ssh() else {
                return Err(QuickLaunchWizardError::MissingSshDraft);
            };
            let host = ssh.host().trim();
            if host.is_empty() {
                return Err(QuickLaunchWizardError::HostRequired);
            }

            let port = ssh.port().trim();
            let port = if port.is_empty() {
                SSH_DEFAULT_PORT
            } else {
                port.parse::<u16>()
                    .map_err(|_| QuickLaunchWizardError::InvalidPort)?
            };

            CommandSpec::Ssh {
                ssh: SshCommand {
                    host: host.to_string(),
                    port,
                    user: optional_string(ssh.user()),
                    identity_file: optional_string(ssh.identity_file()),
                    extra_args: ssh.extra_args().to_vec(),
                },
            }
        },
    };

    Ok(QuickLaunch {
        title: title.to_string(),
        spec,
    })
}

fn optional_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;

    use super::*;
    use crate::widgets::quick_launch::types::{CustomCommand, SshCommand};

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
        assert!(matches!(
            validate_command(&cmd),
            Err(QuickLaunchError::ProgramRequired)
        ));
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
        assert!(matches!(
            validate_command(&cmd),
            Err(QuickLaunchError::HostRequired)
        ));
    }

    #[test]
    fn given_zero_ssh_port_when_validating_then_positive_port_error_returned() {
        let cmd = QuickLaunch {
            title: String::from("SSH"),
            spec: CommandSpec::Ssh {
                ssh: SshCommand {
                    host: String::from("example.com"),
                    port: 0,
                    user: None,
                    identity_file: None,
                    extra_args: Vec::new(),
                },
            },
        };

        assert!(matches!(
            validate_command(&cmd),
            Err(QuickLaunchError::SshPortNotPositive)
        ));
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

        assert!(matches!(
            validate_command(&cmd),
            Err(QuickLaunchError::ProgramNotFoundInPath(_))
        ));
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

    #[test]
    fn given_home_prefix_when_expanding_then_home_directory_is_substituted() {
        let home = Some("/home/user");

        assert_eq!(expand_home_prefix("~", home), "/home/user");
        assert_eq!(expand_home_prefix("~/src", home), "/home/user/src");
        assert_eq!(expand_home_prefix("$HOME", home), "/home/user");
        assert_eq!(expand_home_prefix("$HOME/src", home), "/home/user/src");
        assert_eq!(expand_home_prefix("${HOME}", home), "/home/user");
        assert_eq!(expand_home_prefix("${HOME}/src", home), "/home/user/src");
    }

    #[test]
    fn given_path_without_home_prefix_when_expanding_then_path_is_unchanged() {
        let home = Some("/home/user");

        assert_eq!(expand_home_prefix("/opt/~/bin", home), "/opt/~/bin");
        assert_eq!(expand_home_prefix("~other/src", home), "~other/src");
        assert_eq!(expand_home_prefix("$HOMEDIR/src", home), "$HOMEDIR/src");
        assert_eq!(expand_home_prefix("src/$HOME", home), "src/$HOME");
    }

    #[test]
    fn given_unknown_home_when_expanding_then_path_is_unchanged() {
        assert_eq!(expand_home_prefix("~/src", None), "~/src");
        assert_eq!(expand_home_prefix("$HOME/src", None), "$HOME/src");
    }

    #[test]
    fn given_home_variable_working_directory_when_validating_then_ok() {
        let cmd = QuickLaunch {
            title: String::from("Home"),
            spec: CommandSpec::Custom {
                custom: CustomCommand {
                    program: String::from("bash"),
                    args: Vec::new(),
                    env: Vec::new(),
                    working_directory: Some(String::from("$HOME")),
                },
            },
        };

        assert!(validate_command(&cmd).is_ok());
    }

    #[test]
    fn given_home_relative_paths_when_building_local_session_then_paths_are_expanded()
     {
        let home = std::env::var("HOME").expect("HOME should be set in tests");
        let custom = CustomCommand {
            program: String::from("~/bin/tool"),
            args: Vec::new(),
            env: Vec::new(),
            working_directory: Some(String::from("$HOME/project")),
        };

        let options = custom_session(&custom);

        assert_eq!(options.program(), &format!("{home}/bin/tool"));
        assert_eq!(
            options.working_directory(),
            &Some(PathBuf::from(format!("{home}/project")))
        );
    }

    #[test]
    fn given_home_relative_identity_file_when_building_ssh_session_then_key_path_is_expanded()
     {
        let home = std::env::var("HOME").expect("HOME should be set in tests");
        let ssh = SshCommand {
            host: String::from("example.com"),
            port: 22,
            user: Some(String::from("user")),
            identity_file: Some(String::from("${HOME}/.ssh/id_ed25519")),
            extra_args: Vec::new(),
        };
        let cancel = Arc::new(AtomicBool::new(false));

        let options = ssh_session(&ssh, &cancel);

        let SSHAuth::KeyFile {
            private_key_path, ..
        } = options.auth()
        else {
            panic!("expected key file auth");
        };
        assert_eq!(private_key_path, format!("{home}/.ssh/id_ed25519"));
    }

    #[test]
    fn given_empty_title_when_building_command_then_returns_title_required() {
        let editor = WizardEditorState::new(vec![]);
        let result = build_command(&editor);
        assert!(matches!(result, Err(QuickLaunchWizardError::TitleRequired)));
    }

    #[test]
    fn given_custom_editor_when_building_command_then_returns_custom_launch() {
        let mut editor = WizardEditorState::new(vec![]);
        editor.set_title(String::from("Build"));
        editor.set_program(String::from("cargo"));
        editor.add_arg();
        editor.update_arg(0, String::from("check"));
        editor.add_env();
        editor.update_env_key(0, String::from("RUST_LOG"));
        editor.update_env_value(0, String::from("debug"));
        editor.set_working_directory(String::from("/tmp/project"));

        let quick_launch =
            build_command(&editor).expect("build should succeed");
        assert_eq!(quick_launch.title(), "Build");
        let CommandSpec::Custom { custom } = &quick_launch.spec else {
            panic!("expected custom command");
        };
        assert_eq!(custom.program(), "cargo");
        assert_eq!(custom.args(), &[String::from("check")]);
        assert_eq!(custom.env().len(), 1);
        assert_eq!(custom.env()[0].key(), "RUST_LOG");
        assert_eq!(custom.env()[0].value(), "debug");
        assert_eq!(custom.working_directory(), Some("/tmp/project"));
    }

    #[test]
    fn given_invalid_ssh_port_when_building_command_then_returns_error() {
        let mut editor = WizardEditorState::new(vec![]);
        editor.set_title(String::from("SSH"));
        editor.set_command_type(QuickLaunchType::Ssh);
        editor.set_host(String::from("example.com"));
        editor.set_port(String::from("not-a-port"));

        let result = build_command(&editor);
        assert!(matches!(result, Err(QuickLaunchWizardError::InvalidPort)));
    }
}
