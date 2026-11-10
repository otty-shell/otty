//! The session new terminal tabs start: the user's shell, in the home
//! folder. Building it writes nothing; the OTTY shell integration is added by
//! `effects::shell_integration::integrated_shell_session` at start.

use std::env;
use std::path::PathBuf;

use otty_app_domain::shell_session::shell_name;
use otty_ui_term::settings::{LocalSessionOptions, SessionKind};

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

    /// Build the session of the shell at `shell_path` from its launch
    /// `options`, named after the shell.
    pub(crate) fn from_options(
        shell_path: &str,
        options: LocalSessionOptions,
    ) -> Self {
        Self {
            name: shell_name(shell_path),
            session: SessionKind::from_local_options(options),
        }
    }

    /// Build the session of the shell at `shell_path`, started in the home
    /// folder, without the OTTY shell integration.
    pub(crate) fn without_integration(shell_path: &str) -> Self {
        Self::from_options(shell_path, shell_session_options(shell_path))
    }
}

/// Return the launch options of the shell at `shell_path`: that program,
/// started in the home folder.
pub(crate) fn shell_session_options(shell_path: &str) -> LocalSessionOptions {
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

    use super::ShellSession;

    #[test]
    fn given_shell_session_when_constructed_then_getters_return_values() {
        let options = LocalSessionOptions::default().with_program("/bin/sh");
        let session = ShellSession::from_options("/bin/sh", options);

        assert_eq!(session.name(), "sh");
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
    fn given_shell_path_when_session_without_integration_built_then_uses_local_program()
     {
        let session = ShellSession::without_integration("/usr/bin/bash");

        assert_eq!(session.name(), "bash");
        match session.session() {
            SessionKind::Local(options) => {
                assert_eq!(options.program(), "/usr/bin/bash");
                assert!(options.args().is_empty());
            },
            SessionKind::Ssh(_) => {
                panic!("expected local session kind");
            },
        }
    }

    #[test]
    fn given_home_when_session_without_integration_built_then_starts_in_home() {
        let home = std::env::var_os("HOME").map(PathBuf::from);

        let session = ShellSession::without_integration("/bin/zsh");

        match session.session() {
            SessionKind::Local(options) => {
                assert_eq!(options.working_directory(), &home);
                assert!(options.envs().get("ZDOTDIR").is_none());
            },
            SessionKind::Ssh(_) => {
                panic!("expected local session kind");
            },
        }
    }
}
