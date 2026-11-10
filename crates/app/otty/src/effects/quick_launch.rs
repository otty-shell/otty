//! The I/O of quick launch: the launches file,
//! `~/.config/otty/quick_launches.json`, read once at start and written
//! when the tree changes, and the check before a launch runs: the program,
//! the folders and the key file must exist, and an SSH launch must log in.
//! The check then builds the terminal settings the launch runs with. Each
//! answer comes back as a quick launch intent.

#![allow(
    clippy::disallowed_methods,
    reason = "effects/ is where the binary reads and writes files, checks programs and connects over SSH"
)]

use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use iced::Task;
use otty_app_domain::quick_launch::{
    CommandSpec, CustomCommand, NodePath, QuickLaunch, SshCommand,
};
use otty_app_i18n::{self as i18n, Key};
use otty_app_quick_launch::{
    LaunchRequest, QuickLaunchFile, QuickLaunchIntent, QuickLaunchSetupOutcome,
    SshSecretKind, SshSecretRequest,
};
use otty_app_shell::{shell_integration_dir, with_shell_integration};
use otty_libterm::pty::{self, SSHAuth, Session, SessionError};
use otty_ui_term::settings::{
    LocalSessionOptions, SSHSessionOptions, SessionKind, Settings,
};
use thiserror::Error;

use crate::message::AppEvent;
use crate::terminal_settings;

/// How long an SSH launch waits for the server before it gives up.
const QUICK_LAUNCH_SSH_TIMEOUT: Duration = Duration::from_secs(15);

/// Errors of reading or writing the launches file.
#[derive(Debug, Error)]
enum LaunchesFileError {
    #[error("{}{}", i18n::t(Key::ErrIoPrefix), .0)]
    Io(#[from] std::io::Error),
    #[error("{}{}", i18n::t(Key::ErrJsonPrefix), .0)]
    Json(#[from] serde_json::Error),
}

/// Why a launch can not run, found by the check before it runs.
#[derive(Debug, Error)]
enum LaunchError {
    /// A custom command has no program.
    #[error("{}", i18n::t(Key::ErrProgramRequired))]
    ProgramRequired,
    /// An SSH command has no host.
    #[error("{}", i18n::t(Key::ErrHostRequired))]
    HostRequired,
    /// An SSH command uses port zero.
    #[error("{}", i18n::t(Key::ErrSshPortPositive))]
    SshPortNotPositive,
    /// An SSH connection attempt failed.
    #[error("{}", i18n::ssh_connection_failed(.0))]
    SshConnectionFailed(String),
    /// An SSH connection needs a passphrase or password from the user.
    #[error("{}", secret_prompt_title(.0.kind()))]
    SshSecretRequired(SshSecretRequest),
    /// A configured working directory does not exist.
    #[error("{}", i18n::working_directory_not_found(.0))]
    WorkingDirectoryNotFound(String),
    /// A configured working path is not a directory.
    #[error("{}", i18n::working_directory_not_directory(.0))]
    WorkingDirectoryNotDirectory(String),
    /// A configured SSH identity file does not exist.
    #[error("{}", i18n::identity_file_not_found(.0))]
    IdentityFileNotFound(String),
    /// A configured SSH identity path is not a file.
    #[error("{}", i18n::identity_file_not_file(.0))]
    IdentityFileNotFile(String),
    /// A program name could not be resolved through `PATH`.
    #[error("{}", i18n::program_not_found_in_path(.0))]
    ProgramNotFoundInPath(String),
    /// An explicit program path does not exist.
    #[error("{}", i18n::program_not_found(.0))]
    ProgramNotFound(String),
    /// An explicit program path points to a directory.
    #[error("{}", i18n::program_is_directory(.0))]
    ProgramIsDirectory(String),
    /// An explicit program path is not executable.
    #[error("{}", i18n::program_not_executable(.0))]
    ProgramNotExecutable(String),
}

/// Read the launches at start. A missing file is the first start and gives
/// an empty tree; a file that can not be read gives an empty tree too, and
/// the text that tells the user (the next change writes over the file).
pub(crate) fn load_at_start() -> (QuickLaunchFile, Option<String>) {
    load_at_start_from(&launches_path())
}

/// Write the launches in the background; the answer comes back as
/// [`QuickLaunchIntent::PersistCompleted`] or
/// [`QuickLaunchIntent::PersistFailed`].
pub(crate) fn save(data: QuickLaunchFile) -> Task<AppEvent> {
    Task::perform(
        async move {
            save_to(&launches_path(), &data).map_err(|err| format!("{err}"))
        },
        save_finished,
    )
}

/// Check `request` in the background and build its terminal settings from
/// `base`; the answer comes back as [`QuickLaunchIntent::SetupCompleted`].
pub(crate) fn prepare(
    request: LaunchRequest,
    base: Settings,
) -> Task<AppEvent> {
    Task::perform(
        async move {
            prepare_launch(
                request.command().clone(),
                request.path().to_vec(),
                request.launch_id(),
                &base,
                request.secret(),
                request.cancel(),
            )
        },
        setup_finished,
    )
}

fn setup_finished(outcome: QuickLaunchSetupOutcome) -> AppEvent {
    AppEvent::QuickLaunch(QuickLaunchIntent::SetupCompleted(outcome))
}

fn save_finished(result: Result<(), String>) -> AppEvent {
    let intent = match result {
        Ok(()) => QuickLaunchIntent::PersistCompleted,
        Err(message) => QuickLaunchIntent::PersistFailed(message),
    };

    AppEvent::QuickLaunch(intent)
}

/// Read the launches at start from `path`, as [`load_at_start`] tells.
fn load_at_start_from(path: &Path) -> (QuickLaunchFile, Option<String>) {
    let error = match load_from(path) {
        Ok(data) => return (data, None),
        Err(err) => format!("{err}"),
    };

    log::warn!("Failed to load quick launches, using defaults: {error}");
    let problem = i18n::with_error(Key::TplLaunchesReadFailed, &error);

    (QuickLaunchFile::default(), Some(problem))
}

/// Read the launches from the file at `path`; a missing file gives an
/// empty tree.
fn load_from(path: &Path) -> Result<QuickLaunchFile, LaunchesFileError> {
    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(err) if err.kind() == ErrorKind::NotFound => {
            return Ok(QuickLaunchFile::default());
        },
        Err(err) => return Err(err.into()),
    };

    Ok(serde_json::from_str(&content)?)
}

/// Write the launches to `path`: next to it first, then moved there, so a
/// failed write never leaves half a file.
fn save_to(
    path: &Path,
    data: &QuickLaunchFile,
) -> Result<(), LaunchesFileError> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }

    let payload = serde_json::to_string_pretty(data)?;
    let tmp_path = path.with_extension("json.tmp");
    std::fs::write(&tmp_path, payload.as_bytes())?;
    std::fs::rename(tmp_path, path)?;

    Ok(())
}

/// Return the path of the launches file.
fn launches_path() -> PathBuf {
    // A save in a test must not touch the launches of the user running it.
    if cfg!(test) {
        return std::env::temp_dir()
            .join("otty-app-tests")
            .join("quick_launches.json");
    }

    if let Ok(home) = std::env::var("HOME") {
        return Path::new(&home)
            .join(".config")
            .join("otty")
            .join("quick_launches.json");
    }

    std::env::temp_dir()
        .join("otty")
        .join("quick_launches.json")
}

/// Check a launch and build the terminal settings it runs with.
///
/// It validates the command, resolves paths, logs in over SSH, and returns
/// the prepared launch or why not. `secret` is the SSH key passphrase or
/// password typed by the user; it lives only in memory for this launch.
fn prepare_launch(
    command: QuickLaunch,
    path: NodePath,
    launch_id: u64,
    base: &Settings,
    secret: Option<&str>,
    cancel: &Arc<AtomicBool>,
) -> QuickLaunchSetupOutcome {
    if cancel.load(Ordering::Relaxed) {
        return QuickLaunchSetupOutcome::Canceled { path, launch_id };
    }

    if let Err(error) = validate_command(&command) {
        return QuickLaunchSetupOutcome::Failed {
            path,
            launch_id,
            command: Box::new(command),
            error: format!("{error}"),
        };
    }

    if cancel.load(Ordering::Relaxed) {
        return QuickLaunchSetupOutcome::Canceled { path, launch_id };
    }

    if let Err(error) = probe_launch_runtime(&command, secret, cancel) {
        return probe_failure_outcome(path, launch_id, command, error);
    }

    let session = command_session(&command, secret, cancel);
    let settings = terminal_settings::for_session(base, session);

    if cancel.load(Ordering::Relaxed) {
        return QuickLaunchSetupOutcome::Canceled { path, launch_id };
    }

    QuickLaunchSetupOutcome::Prepared {
        path,
        launch_id,
        command: Box::new(command),
        settings: Box::new(settings),
    }
}

/// Validate a quick launch command before execution.
fn validate_command(command: &QuickLaunch) -> Result<(), LaunchError> {
    match command.spec() {
        CommandSpec::Custom { custom } => {
            if custom.program().trim().is_empty() {
                return Err(LaunchError::ProgramRequired);
            }

            validate_custom_runtime(custom)?;
        },
        CommandSpec::Ssh { ssh } => {
            if ssh.host().trim().is_empty() {
                return Err(LaunchError::HostRequired);
            }
            if ssh.port() == 0 {
                return Err(LaunchError::SshPortNotPositive);
            }

            validate_ssh_runtime(ssh)?;
        },
    }
    Ok(())
}

fn probe_launch_runtime(
    command: &QuickLaunch,
    secret: Option<&str>,
    cancel: &Arc<AtomicBool>,
) -> Result<(), LaunchError> {
    match command.spec() {
        CommandSpec::Custom { .. } => Ok(()),
        CommandSpec::Ssh { ssh } => probe_ssh_session(ssh, secret, cancel),
    }
}

/// Build the outcome for a failed probe: ask the user for a secret when the
/// SSH session needs one, otherwise report the failure.
fn probe_failure_outcome(
    path: NodePath,
    launch_id: u64,
    command: QuickLaunch,
    error: LaunchError,
) -> QuickLaunchSetupOutcome {
    match error {
        LaunchError::SshSecretRequired(request) => {
            QuickLaunchSetupOutcome::SecretRequired {
                path,
                launch_id,
                command: Box::new(command),
                request,
            }
        },
        _ => QuickLaunchSetupOutcome::Failed {
            path,
            launch_id,
            command: Box::new(command),
            error: format!("{error}"),
        },
    }
}

fn probe_ssh_session(
    ssh: &SshCommand,
    secret: Option<&str>,
    cancel: &Arc<AtomicBool>,
) -> Result<(), LaunchError> {
    let options = ssh_session(ssh, secret, cancel);
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

    let mut session = builder
        .spawn()
        .map_err(|err| ssh_connection_error(err, &options))?;
    let _ = session.close();
    Ok(())
}

/// Map an SSH session error to a launch error. Missing or wrong secrets
/// become a request to ask the user again.
fn ssh_connection_error(
    err: SessionError,
    options: &SSHSessionOptions,
) -> LaunchError {
    let key_target = || match options.auth() {
        SSHAuth::KeyFile {
            private_key_path, ..
        } => private_key_path,
        _ => String::new(),
    };
    let password_target = || format!("{}@{}", options.user(), options.host());

    let request = match err {
        SessionError::PassphraseRequired => SshSecretRequest::new(
            SshSecretKind::KeyPassphrase,
            key_target(),
            false,
        ),
        SessionError::WrongPassphrase => SshSecretRequest::new(
            SshSecretKind::KeyPassphrase,
            key_target(),
            true,
        ),
        SessionError::PasswordRequired => SshSecretRequest::new(
            SshSecretKind::Password,
            password_target(),
            false,
        ),
        SessionError::PasswordRejected => SshSecretRequest::new(
            SshSecretKind::Password,
            password_target(),
            true,
        ),
        _ => return LaunchError::SshConnectionFailed(format!("{err}")),
    };

    LaunchError::SshSecretRequired(request)
}

fn command_session(
    command: &QuickLaunch,
    secret: Option<&str>,
    cancel: &Arc<AtomicBool>,
) -> SessionKind {
    match command.spec() {
        CommandSpec::Custom { custom } => SessionKind::from_local_options(
            custom_session(custom, &shell_integration_dir()),
        ),
        CommandSpec::Ssh { ssh } => {
            SessionKind::from_ssh_options(ssh_session(ssh, secret, cancel))
        },
    }
}

/// Build the local session of a custom launch. A launch of a plain zsh or
/// bash, with no arguments, gets the OTTY shell integration written to
/// `integration_dir`, so the Files panel and the session row follow it like
/// a shell tab.
fn custom_session(
    custom: &CustomCommand,
    integration_dir: &Path,
) -> LocalSessionOptions {
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

    if !custom.args().is_empty() {
        return options;
    }

    match with_shell_integration(options.clone(), integration_dir) {
        Ok(integrated) => integrated,
        Err(err) => {
            log::warn!("shell integration for a launch failed: {err}");
            options
        },
    }
}

fn ssh_session(
    ssh: &SshCommand,
    secret: Option<&str>,
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
            passphrase: secret.map(String::from),
        })
        .unwrap_or_else(|| {
            SSHAuth::Password(secret.unwrap_or_default().to_string())
        });

    SSHSessionOptions::default()
        .with_host(&host)
        .with_user(&user)
        .with_auth(auth)
        .with_timeout(QUICK_LAUNCH_SSH_TIMEOUT)
        .with_cancel_token(cancel.clone())
}

fn validate_custom_runtime(custom: &CustomCommand) -> Result<(), LaunchError> {
    let program = custom.program().trim();
    let _ = find_program_path(program)?;

    if let Some(dir) = custom.working_directory() {
        let expanded = expand_home_dir(dir);
        let path = Path::new(&expanded);

        if !path.exists() {
            return Err(LaunchError::WorkingDirectoryNotFound(expanded));
        }

        if !path.is_dir() {
            return Err(LaunchError::WorkingDirectoryNotDirectory(expanded));
        }
    }

    Ok(())
}

fn validate_ssh_runtime(ssh: &SshCommand) -> Result<(), LaunchError> {
    if let Some(identity) = ssh.identity_file() {
        let identity = identity.trim();
        if !identity.is_empty() {
            let expanded = expand_home_dir(identity);
            let path = Path::new(&expanded);

            if !path.exists() {
                return Err(LaunchError::IdentityFileNotFound(expanded));
            }

            if !path.is_file() {
                return Err(LaunchError::IdentityFileNotFile(expanded));
            }
        }
    }

    Ok(())
}

fn find_program_path(program: &str) -> Result<PathBuf, LaunchError> {
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

    Err(LaunchError::ProgramNotFoundInPath(program))
}

fn validate_program_path(
    path: &Path,
    label: &str,
) -> Result<PathBuf, LaunchError> {
    if !path.exists() {
        return Err(LaunchError::ProgramNotFound(label.to_string()));
    }

    if path.is_dir() {
        return Err(LaunchError::ProgramIsDirectory(label.to_string()));
    }

    if !is_executable_path(path) {
        return Err(LaunchError::ProgramNotExecutable(label.to_string()));
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

/// Title of the dialog that asks for an SSH secret of the given kind.
fn secret_prompt_title(kind: SshSecretKind) -> &'static str {
    match kind {
        SshSecretKind::KeyPassphrase => {
            i18n::t(Key::SecretPromptPassphraseTitle)
        },
        SshSecretKind::Password => i18n::t(Key::SecretPromptPasswordTitle),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;

    use otty_app_domain::exit_behavior::ExitBehavior;
    use otty_app_testing::temp_dir::TempDir;

    use super::*;

    /// A missing file is the first start: an empty tree, nothing to tell.
    #[test]
    fn at_start_a_missing_file_gives_an_empty_tree_and_tells_nothing() {
        let root = TempDir::new("launches-start-missing");

        let (data, problem) =
            load_at_start_from(&root.path().join("quick_launches.json"));

        assert!(data.launch_count() == 0);
        assert!(problem.is_none());
    }

    #[test]
    fn at_start_a_read_file_gives_its_launches_and_tells_nothing() {
        let root = TempDir::new("launches-start-read");
        let path = root.path().join("quick_launches.json");
        let saved = serde_json::json!({
            "version": 1,
            "root": {
                "title": "Root",
                "expanded": true,
                "children": [{
                    "kind": "Command",
                    "title": "Run",
                    "spec": {
                        "type": "Custom",
                        "custom": {
                            "program": "bash",
                            "args": [],
                            "env": [],
                            "working_directory": null
                        }
                    }
                }]
            }
        });
        fs::write(&path, saved.to_string())
            .expect("launches file should be written");

        let (data, problem) = load_at_start_from(&path);

        assert_eq!(data.launch_count(), 1);
        assert!(problem.is_none());
    }

    /// A broken file starts with an empty tree, as before, and the user is
    /// told: the next edit writes over the file.
    #[test]
    fn at_start_a_broken_file_gives_an_empty_tree_and_a_problem() {
        let root = TempDir::new("launches-start-broken");
        let path = root.path().join("quick_launches.json");
        fs::write(&path, "{ \"root\": [")
            .expect("launches file should be written");

        let (data, problem) = load_at_start_from(&path);

        let error =
            load_from(&path).expect_err("a broken file should not load");
        let expected =
            i18n::with_error(Key::TplLaunchesReadFailed, &format!("{error}"));
        assert!(data.launch_count() == 0);
        assert_eq!(problem.as_deref(), Some(expected.as_str()));
    }

    #[test]
    fn given_empty_program_when_validating_then_error_returned() {
        let cmd = QuickLaunch::new(
            String::from("Bad"),
            CommandSpec::Custom {
                custom: CustomCommand::new(
                    String::new(),
                    Vec::new(),
                    Vec::new(),
                    None,
                ),
            },
            ExitBehavior::Close,
        );
        assert!(matches!(
            validate_command(&cmd),
            Err(LaunchError::ProgramRequired)
        ));
    }

    #[test]
    fn given_valid_custom_command_when_validating_then_ok() {
        let cmd = QuickLaunch::new(
            String::from("Good"),
            CommandSpec::Custom {
                custom: CustomCommand::new(
                    String::from("bash"),
                    Vec::new(),
                    Vec::new(),
                    None,
                ),
            },
            ExitBehavior::Close,
        );
        assert!(validate_command(&cmd).is_ok());
    }

    #[test]
    fn given_empty_ssh_host_when_validating_then_error_returned() {
        let cmd = QuickLaunch::new(
            String::from("SSH"),
            CommandSpec::Ssh {
                ssh: SshCommand::new(String::new(), 22, None, None, Vec::new()),
            },
            ExitBehavior::Close,
        );
        assert!(matches!(
            validate_command(&cmd),
            Err(LaunchError::HostRequired)
        ));
    }

    #[test]
    fn given_zero_ssh_port_when_validating_then_positive_port_error_returned() {
        let cmd = QuickLaunch::new(
            String::from("SSH"),
            CommandSpec::Ssh {
                ssh: SshCommand::new(
                    String::from("example.com"),
                    0,
                    None,
                    None,
                    Vec::new(),
                ),
            },
            ExitBehavior::Close,
        );

        assert!(matches!(
            validate_command(&cmd),
            Err(LaunchError::SshPortNotPositive)
        ));
    }

    #[test]
    fn given_missing_program_when_validating_then_error_returned() {
        let cmd = QuickLaunch::new(
            String::from("Missing"),
            CommandSpec::Custom {
                custom: CustomCommand::new(
                    String::from("otty-v3-test-bin-does-not-exist"),
                    Vec::new(),
                    Vec::new(),
                    None,
                ),
            },
            ExitBehavior::Close,
        );

        assert!(matches!(
            validate_command(&cmd),
            Err(LaunchError::ProgramNotFoundInPath(_))
        ));
    }

    #[test]
    fn given_custom_command_when_prepared_then_settings_use_saved_program() {
        let command = QuickLaunch::new(
            String::from("Run htop"),
            CommandSpec::Custom {
                custom: CustomCommand::new(
                    String::from("htop"),
                    vec![
                        String::from("--sort-key"),
                        String::from("PERCENT_CPU"),
                    ],
                    Vec::new(),
                    None,
                ),
            },
            ExitBehavior::Close,
        );

        let cancel = Arc::new(AtomicBool::new(false));
        let session = command_session(&command, None, &cancel);
        let SessionKind::Local(options) = session else {
            panic!("expected local session");
        };
        assert_eq!(options.program(), "htop");
        assert_eq!(
            options.args(),
            &[String::from("--sort-key"), String::from("PERCENT_CPU")]
        );
    }

    fn custom_command(program: &str, args: &[&str]) -> CustomCommand {
        CustomCommand::new(
            String::from(program),
            args.iter().map(|arg| String::from(*arg)).collect(),
            Vec::new(),
            Some(String::from("/tmp")),
        )
    }

    #[test]
    fn given_plain_zsh_launch_when_session_built_then_shell_integration_is_added()
     {
        let folder = TempDir::new("launch-zsh");
        let dir = folder.path();

        let options = custom_session(&custom_command("/usr/bin/zsh", &[]), dir);

        assert_eq!(options.program(), "/usr/bin/zsh");
        assert_eq!(
            options.envs().get("ZDOTDIR"),
            Some(&dir.to_string_lossy().to_string()),
        );
        assert_eq!(options.working_directory(), &Some(PathBuf::from("/tmp")));
    }

    #[test]
    fn given_zsh_launch_with_args_when_session_built_then_it_runs_as_saved() {
        let dir = std::env::temp_dir()
            .join(format!("otty-launch-zsh-args-{}", std::process::id()));

        let options = custom_session(
            &custom_command("/usr/bin/zsh", &["-c", "make"]),
            &dir,
        );

        assert_eq!(options.args(), &[String::from("-c"), String::from("make")]);
        assert!(options.envs().get("ZDOTDIR").is_none());
        assert!(!dir.exists());
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
        let cmd = QuickLaunch::new(
            String::from("Home"),
            CommandSpec::Custom {
                custom: CustomCommand::new(
                    String::from("bash"),
                    Vec::new(),
                    Vec::new(),
                    Some(String::from("$HOME")),
                ),
            },
            ExitBehavior::Close,
        );

        assert!(validate_command(&cmd).is_ok());
    }

    #[test]
    fn given_home_relative_paths_when_building_local_session_then_paths_are_expanded()
     {
        let home = std::env::var("HOME").expect("HOME should be set in tests");
        let custom = CustomCommand::new(
            String::from("~/bin/tool"),
            Vec::new(),
            Vec::new(),
            Some(String::from("$HOME/project")),
        );

        // `tool` is not a shell, so nothing is written to this folder.
        let options = custom_session(&custom, &std::env::temp_dir());

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
        let ssh = SshCommand::new(
            String::from("example.com"),
            22,
            Some(String::from("user")),
            Some(String::from("${HOME}/.ssh/id_ed25519")),
            Vec::new(),
        );
        let cancel = Arc::new(AtomicBool::new(false));

        let options = ssh_session(&ssh, None, &cancel);

        let SSHAuth::KeyFile {
            private_key_path, ..
        } = options.auth()
        else {
            panic!("expected key file auth");
        };
        assert_eq!(private_key_path, format!("{home}/.ssh/id_ed25519"));
    }

    fn ssh_command(identity_file: Option<&str>) -> SshCommand {
        SshCommand::new(
            String::from("example.com"),
            2222,
            Some(String::from("alice")),
            identity_file.map(String::from),
            Vec::new(),
        )
    }

    fn ssh_command_launch(identity_file: Option<&str>) -> QuickLaunch {
        QuickLaunch::new(
            String::from("Server"),
            CommandSpec::Ssh {
                ssh: ssh_command(identity_file),
            },
            ExitBehavior::Close,
        )
    }

    #[test]
    fn given_secret_and_identity_file_when_building_ssh_session_then_secret_is_passphrase()
     {
        let ssh = ssh_command(Some("/keys/id_ed25519"));
        let cancel = Arc::new(AtomicBool::new(false));

        let options = ssh_session(&ssh, Some("open sesame"), &cancel);

        let SSHAuth::KeyFile { passphrase, .. } = options.auth() else {
            panic!("expected key file auth");
        };
        assert_eq!(passphrase.as_deref(), Some("open sesame"));
    }

    #[test]
    fn given_secret_without_identity_file_when_building_ssh_session_then_secret_is_password()
     {
        let ssh = ssh_command(None);
        let cancel = Arc::new(AtomicBool::new(false));

        let options = ssh_session(&ssh, Some("hunter2"), &cancel);

        let SSHAuth::Password(password) = options.auth() else {
            panic!("expected password auth");
        };
        assert_eq!(password, "hunter2");
    }

    #[test]
    fn given_no_secret_without_identity_file_when_building_ssh_session_then_password_is_empty()
     {
        let ssh = ssh_command(None);
        let cancel = Arc::new(AtomicBool::new(false));

        let options = ssh_session(&ssh, None, &cancel);

        let SSHAuth::Password(password) = options.auth() else {
            panic!("expected password auth");
        };
        assert!(password.is_empty());
    }

    #[test]
    fn given_passphrase_required_when_mapping_ssh_error_then_passphrase_is_requested()
     {
        let ssh = ssh_command(Some("/keys/id_ed25519"));
        let options =
            ssh_session(&ssh, None, &Arc::new(AtomicBool::new(false)));

        let error =
            ssh_connection_error(SessionError::PassphraseRequired, &options);

        let LaunchError::SshSecretRequired(request) = error else {
            panic!("expected secret request");
        };
        assert_eq!(request.kind(), SshSecretKind::KeyPassphrase);
        assert_eq!(request.target(), "/keys/id_ed25519");
        assert!(!request.is_retry());
    }

    #[test]
    fn given_wrong_passphrase_when_mapping_ssh_error_then_passphrase_is_requested_again()
     {
        let ssh = ssh_command(Some("/keys/id_ed25519"));
        let options =
            ssh_session(&ssh, Some("bad"), &Arc::new(AtomicBool::new(false)));

        let error =
            ssh_connection_error(SessionError::WrongPassphrase, &options);

        let LaunchError::SshSecretRequired(request) = error else {
            panic!("expected secret request");
        };
        assert_eq!(request.kind(), SshSecretKind::KeyPassphrase);
        assert!(request.is_retry());
    }

    #[test]
    fn given_password_required_when_mapping_ssh_error_then_password_is_requested()
     {
        let ssh = ssh_command(None);
        let options =
            ssh_session(&ssh, None, &Arc::new(AtomicBool::new(false)));

        let error =
            ssh_connection_error(SessionError::PasswordRequired, &options);

        let LaunchError::SshSecretRequired(request) = error else {
            panic!("expected secret request");
        };
        assert_eq!(request.kind(), SshSecretKind::Password);
        assert_eq!(request.target(), "alice@example.com:2222");
        assert!(!request.is_retry());
    }

    #[test]
    fn given_rejected_password_when_mapping_ssh_error_then_password_is_requested_again()
     {
        let ssh = ssh_command(None);
        let options =
            ssh_session(&ssh, Some("bad"), &Arc::new(AtomicBool::new(false)));

        let error =
            ssh_connection_error(SessionError::PasswordRejected, &options);

        let LaunchError::SshSecretRequired(request) = error else {
            panic!("expected secret request");
        };
        assert_eq!(request.kind(), SshSecretKind::Password);
        assert!(request.is_retry());
    }

    #[test]
    fn given_other_ssh_error_when_mapping_then_connection_failed() {
        let ssh = ssh_command(None);
        let options =
            ssh_session(&ssh, None, &Arc::new(AtomicBool::new(false)));

        let error = ssh_connection_error(SessionError::NoAddresses, &options);

        assert!(matches!(error, LaunchError::SshConnectionFailed(_)));
    }

    #[test]
    fn given_secret_request_when_probe_fails_then_outcome_asks_for_secret() {
        let request = SshSecretRequest::new(
            SshSecretKind::Password,
            String::from("alice@example.com:2222"),
            false,
        );

        let outcome = probe_failure_outcome(
            vec![String::from("Server")],
            7,
            ssh_command_launch(None),
            LaunchError::SshSecretRequired(request.clone()),
        );

        let QuickLaunchSetupOutcome::SecretRequired {
            path,
            launch_id,
            request: outcome_request,
            ..
        } = outcome
        else {
            panic!("expected secret required outcome");
        };
        assert_eq!(path, vec![String::from("Server")]);
        assert_eq!(launch_id, 7);
        assert_eq!(outcome_request, request);
    }

    #[test]
    fn given_other_error_when_probe_fails_then_outcome_is_failed() {
        let outcome = probe_failure_outcome(
            vec![String::from("Server")],
            7,
            ssh_command_launch(None),
            LaunchError::HostRequired,
        );

        assert!(matches!(outcome, QuickLaunchSetupOutcome::Failed { .. }));
    }

    #[test]
    fn a_finished_save_reaches_the_feature() {
        assert!(matches!(
            save_finished(Ok(())),
            AppEvent::QuickLaunch(QuickLaunchIntent::PersistCompleted)
        ));
    }

    #[test]
    fn a_failed_save_reaches_the_feature_with_its_reason() {
        assert!(matches!(
            save_finished(Err(String::from("disk full"))),
            AppEvent::QuickLaunch(QuickLaunchIntent::PersistFailed(message))
                if message == "disk full"
        ));
    }

    /// A launch killed before its check ran comes back canceled, and
    /// nothing is checked.
    #[test]
    fn a_killed_launch_comes_back_canceled() {
        let command = QuickLaunch::new(
            String::from("htop"),
            CommandSpec::Custom {
                custom: CustomCommand::new(
                    String::from("htop"),
                    Vec::new(),
                    Vec::new(),
                    None,
                ),
            },
            ExitBehavior::Close,
        );

        let outcome = prepare_launch(
            command,
            vec![String::from("htop")],
            7,
            &Settings::default(),
            None,
            &Arc::new(AtomicBool::new(true)),
        );

        assert!(matches!(
            setup_finished(outcome),
            AppEvent::QuickLaunch(QuickLaunchIntent::SetupCompleted(
                QuickLaunchSetupOutcome::Canceled { launch_id: 7, .. }
            ))
        ));
    }

    /// A local launch that can run comes back prepared, with the terminal
    /// settings of its program.
    #[test]
    fn a_local_launch_that_can_run_comes_back_with_its_settings() {
        let command = QuickLaunch::new(
            String::from("sh"),
            CommandSpec::Custom {
                custom: CustomCommand::new(
                    String::from("/bin/sh"),
                    vec![String::from("-c"), String::from("true")],
                    Vec::new(),
                    None,
                ),
            },
            ExitBehavior::KeepOpen,
        );

        let outcome = prepare_launch(
            command,
            vec![String::from("sh")],
            3,
            &Settings::default(),
            None,
            &Arc::new(AtomicBool::new(false)),
        );

        let QuickLaunchSetupOutcome::Prepared {
            launch_id,
            settings,
            ..
        } = outcome
        else {
            panic!("the launch should be prepared");
        };
        let SessionKind::Local(options) = &settings.backend.session else {
            panic!("expected a local session");
        };
        assert_eq!(launch_id, 3);
        assert_eq!(options.program(), "/bin/sh");
    }

    /// A launch whose program is missing comes back failed, with why in the
    /// user's language.
    #[test]
    fn a_launch_that_can_not_run_comes_back_failed_with_why() {
        let command = QuickLaunch::new(
            String::from("Missing"),
            CommandSpec::Custom {
                custom: CustomCommand::new(
                    String::from("otty-v3-test-bin-does-not-exist"),
                    Vec::new(),
                    Vec::new(),
                    None,
                ),
            },
            ExitBehavior::Close,
        );

        let outcome = prepare_launch(
            command,
            vec![String::from("Missing")],
            1,
            &Settings::default(),
            None,
            &Arc::new(AtomicBool::new(false)),
        );

        let expected =
            i18n::program_not_found_in_path("otty-v3-test-bin-does-not-exist");
        assert!(matches!(
            outcome,
            QuickLaunchSetupOutcome::Failed { error, .. } if error == expected
        ));
    }

    /// The launches are written to the file and read back.
    #[test]
    fn saved_launches_are_read_back() {
        let root = TempDir::new("launches-save");
        let path = root.path().join("otty").join("quick_launches.json");
        let saved: QuickLaunchFile =
            serde_json::from_value(serde_json::json!({
                "version": 1,
                "root": {"title": "Root", "expanded": true, "children": [{
                    "kind": "Command", "title": "Run",
                    "spec": {"type": "Custom", "custom": {"program": "bash",
                        "args": [], "env": [], "working_directory": null}}
                }]}
            }))
            .expect("the launches should parse");

        save_to(&path, &saved).expect("the launches should be written");
        let read = load_from(&path).expect("the launches should be read");

        assert_eq!(read.launch_count(), 1);
        assert!(!path.with_extension("json.tmp").exists());
    }
}
