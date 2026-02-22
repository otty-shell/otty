use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

use otty_libterm::pty::SSHAuth;
use otty_ui_term::settings::{
    LocalSessionOptions, SSHSessionOptions, SessionKind, Settings,
};

use crate::features::terminal::terminal_settings_for_session;

use super::errors::QuickLaunchError;
use super::event::{PreparedQuickLaunch, QuickLaunchSetupOutcome};
use super::model::{
    CommandSpec, CustomCommand, EnvVar, NodePath, QuickLaunch, SshCommand,
    validate_custom_command, validate_ssh_command,
};

const QUICK_LAUNCH_SSH_TIMEOUT: Duration = Duration::from_secs(15);

/// Build runtime data required for starting a quick launch terminal session.
pub(crate) async fn prepare_quick_launch_setup(
    command: QuickLaunch,
    path: NodePath,
    launch_id: u64,
    tab_id: u64,
    terminal_id: u64,
    terminal_settings: Settings,
    cancel: Arc<AtomicBool>,
) -> QuickLaunchSetupOutcome {
    if cancel.load(Ordering::Relaxed) {
        return QuickLaunchSetupOutcome::Canceled { path, launch_id };
    }

    if let Err(error) = validate_launch_for_setup(&command) {
        return QuickLaunchSetupOutcome::Failed {
            path,
            launch_id,
            command: Box::new(command),
            error: Arc::new(error),
        };
    }

    // Keep setup in an async pipeline so remote preflight can be added here.
    std::future::ready(()).await;

    let session = command_session(&command, &cancel);
    let settings = terminal_settings_for_session(&terminal_settings, session);
    let title = command.title.clone();

    if cancel.load(Ordering::Relaxed) {
        return QuickLaunchSetupOutcome::Canceled { path, launch_id };
    }

    QuickLaunchSetupOutcome::Prepared(PreparedQuickLaunch {
        path,
        launch_id,
        tab_id,
        terminal_id,
        title,
        settings: Box::new(settings),
        command: Box::new(command),
    })
}

fn validate_launch_for_setup(
    command: &QuickLaunch,
) -> Result<(), QuickLaunchError> {
    match &command.spec {
        CommandSpec::Custom { custom } => {
            validate_custom_command(custom)?;
            validate_custom_runtime(custom)?;
        },
        CommandSpec::Ssh { ssh } => {
            validate_ssh_command(ssh)?;
            validate_ssh_runtime(ssh)?;
        },
    }

    Ok(())
}

fn validate_custom_runtime(
    custom: &CustomCommand,
) -> Result<(), QuickLaunchError> {
    let program = custom.program.trim();
    let _program_path = find_program_path(program)?;

    if let Some(dir) = custom.working_directory.as_deref() {
        let expanded = expand_tilde(dir);
        let path = Path::new(&expanded);
        if !path.exists() {
            return Err(validation_error(format!(
                "Working directory not found: {expanded}"
            )));
        }
        if !path.is_dir() {
            return Err(validation_error(format!(
                "Working directory is not a directory: {expanded}"
            )));
        }
    }

    Ok(())
}

fn validate_ssh_runtime(ssh: &SshCommand) -> Result<(), QuickLaunchError> {
    if let Some(identity) = ssh.identity_file.as_deref() {
        let identity = identity.trim();
        if !identity.is_empty() {
            let expanded = expand_tilde(identity);
            let path = Path::new(&expanded);
            if !path.exists() {
                return Err(validation_error(format!(
                    "Identity file not found: {expanded}"
                )));
            }
            if !path.is_file() {
                return Err(validation_error(format!(
                    "Identity file is not a file: {expanded}"
                )));
            }
        }
    }

    Ok(())
}

fn command_session(
    command: &QuickLaunch,
    cancel: &Arc<AtomicBool>,
) -> SessionKind {
    match &command.spec {
        CommandSpec::Custom { custom } => {
            SessionKind::from_local_options(custom_session(custom))
        },
        CommandSpec::Ssh { ssh } => {
            SessionKind::from_ssh_options(ssh_session(ssh, cancel))
        },
    }
}

fn custom_session(custom: &CustomCommand) -> LocalSessionOptions {
    let mut options = LocalSessionOptions::default()
        .with_program(&custom.program)
        .with_args(custom.args.clone());

    if !custom.env.is_empty() {
        let mut envs = HashMap::new();
        for EnvVar { key, value } in &custom.env {
            envs.insert(key.clone(), value.clone());
        }
        options = options.with_envs(envs);
    }

    if let Some(dir) = &custom.working_directory {
        options = options.with_working_directory(dir.into());
    }

    options
}

fn ssh_session(
    ssh: &SshCommand,
    cancel: &Arc<AtomicBool>,
) -> SSHSessionOptions {
    let host = format!("{}:{}", ssh.host, ssh.port);
    let user = ssh
        .user
        .clone()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| std::env::var("USER").ok())
        .or_else(|| std::env::var("USERNAME").ok())
        .unwrap_or_default();

    let auth = ssh
        .identity_file
        .clone()
        .filter(|value| !value.trim().is_empty())
        .map(|path| SSHAuth::KeyFile {
            private_key_path: path,
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

    Err(validation_error(format!(
        "Program not found in PATH: {program}"
    )))
}

fn validate_program_path(
    path: &Path,
    label: &str,
) -> Result<PathBuf, QuickLaunchError> {
    if !path.exists() {
        return Err(validation_error(format!("Program not found: {label}")));
    }
    if path.is_dir() {
        return Err(validation_error(format!(
            "Program is a directory: {label}"
        )));
    }
    if !is_executable_path(path) {
        return Err(validation_error(format!(
            "Program is not executable: {label}"
        )));
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

fn validation_error(message: String) -> QuickLaunchError {
    QuickLaunchError::Validation { message }
}
