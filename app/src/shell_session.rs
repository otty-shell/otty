//! The session new terminal tabs start: the user's shell, in the home folder,
//! with the OTTY shell integration when its files can be written.

use std::env;
use std::path::PathBuf;

use otty_ui_term::settings::{LocalSessionOptions, SessionKind};

use crate::shell_integration::{
    shell_integration_dir, shell_name, with_shell_integration,
};

/// Shell session information needed to start a terminal backend.
#[derive(Debug, Clone)]
pub(crate) struct ShellSession {
    name: String,
    session: SessionKind,
}

impl ShellSession {
    /// Return shell label shown in tab titles.
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    /// Return backend session descriptor used by terminal settings.
    pub(crate) fn session(&self) -> &SessionKind {
        &self.session
    }

    /// Create shell session metadata for terminal startup.
    fn new(name: String, session: SessionKind) -> Self {
        Self { name, session }
    }
}

/// Build the session of the shell at `shell_path`, started in the home
/// folder. The OTTY shell integration is added when its files can be
/// written; otherwise the shell starts without it.
pub(crate) fn shell_session(shell_path: &str) -> ShellSession {
    match setup_shell_session_with_shell(shell_path) {
        Ok(session) => session,
        Err(err) => {
            log::warn!("shell integration setup failed: {err}");
            fallback_shell_session_with_shell(shell_path)
        },
    }
}

/// Build a shell session configured with OTTY integrations for a given shell.
fn setup_shell_session_with_shell(
    shell_path: &str,
) -> std::io::Result<ShellSession> {
    let shell_name = shell_name(shell_path);
    let options = with_shell_integration(
        shell_session_options(shell_path),
        &shell_integration_dir(),
    )?;

    Ok(ShellSession::new(
        shell_name,
        SessionKind::from_local_options(options),
    ))
}

/// Build a shell session without writing shell integration files.
fn fallback_shell_session_with_shell(shell_path: &str) -> ShellSession {
    let shell_name = shell_name(shell_path);
    let options = shell_session_options(shell_path);

    ShellSession::new(shell_name, SessionKind::from_local_options(options))
}

fn shell_session_options(shell_path: &str) -> LocalSessionOptions {
    let options = LocalSessionOptions::default().with_program(shell_path);

    if let Some(home) = env::var_os("HOME") {
        return options.with_working_directory(PathBuf::from(home));
    }

    options
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use otty_ui_term::settings::{LocalSessionOptions, SessionKind};

    use super::{
        ShellSession, fallback_shell_session_with_shell, shell_session,
    };

    #[test]
    fn given_shell_session_when_constructed_then_getters_return_values() {
        let session_kind = SessionKind::from_local_options(
            LocalSessionOptions::default().with_program("/bin/sh"),
        );
        let session = ShellSession::new(String::from("shell"), session_kind);

        assert_eq!(session.name(), "shell");
        match session.session() {
            SessionKind::Local(options) => {
                assert_eq!(options.program(), "/bin/sh");
            },
            SessionKind::Ssh(_) => {
                panic!("expected local session kind");
            },
        }
    }

    #[test]
    fn given_shell_without_integration_when_session_built_then_it_runs_that_shell_in_home()
     {
        let home = std::env::var_os("HOME").map(PathBuf::from);

        let session = shell_session("/bin/sh");

        assert_eq!(session.name(), "sh");
        match session.session() {
            SessionKind::Local(options) => {
                assert_eq!(options.program(), "/bin/sh");
                assert_eq!(options.working_directory(), &home);
            },
            SessionKind::Ssh(_) => {
                panic!("expected local session kind");
            },
        }
    }

    #[test]
    fn given_shell_path_when_fallback_session_created_then_uses_local_program()
    {
        let session = fallback_shell_session_with_shell("/usr/bin/bash");

        assert_eq!(session.name(), "bash");
        match session.session() {
            SessionKind::Local(options) => {
                assert_eq!(options.program(), "/usr/bin/bash");
            },
            SessionKind::Ssh(_) => {
                panic!("expected local session kind");
            },
        }
    }

    #[test]
    fn given_home_when_fallback_session_created_then_starts_in_home() {
        let home = std::env::var_os("HOME").map(PathBuf::from);

        let session = fallback_shell_session_with_shell("/bin/zsh");

        match session.session() {
            SessionKind::Local(options) => {
                assert_eq!(options.working_directory(), &home);
            },
            SessionKind::Ssh(_) => {
                panic!("expected local session kind");
            },
        }
    }
}
