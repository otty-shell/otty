//! The session new terminal tabs start: the user's shell, in the home
//! folder or in a folder the tab asks for. Building it writes nothing; the
//! OTTY shell integration is added by [`crate::integrated_shell_session`].

use std::env;
use std::path::{Path, PathBuf};

use otty_app_domain::shell_session::shell_name;
use otty_ui_term::settings::{LocalSessionOptions, SessionKind};

/// The user's shell that new terminal tabs start: its name and its launch
/// options. A shell always runs on this computer.
///
/// # Examples
///
/// ```
/// use std::path::Path;
///
/// use otty_app_shell::ShellSession;
/// use otty_ui_term::settings::SessionKind;
///
/// let shell = ShellSession::without_integration("/bin/sh");
/// assert_eq!(shell.name(), "sh");
///
/// let SessionKind::Local(options) = shell.session(Some(Path::new("/tmp")))
/// else {
///     panic!("a shell runs on this computer");
/// };
/// assert_eq!(options.working_directory(), &Some("/tmp".into()));
/// ```
#[derive(Debug, Clone)]
pub struct ShellSession {
    name: String,
    options: LocalSessionOptions,
}

impl ShellSession {
    /// Return shell label shown in tab titles.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Return the session a new terminal tab starts: the shell in `folder`,
    /// or without one in the folder its options start in, the home folder.
    pub fn session(&self, folder: Option<&Path>) -> SessionKind {
        let mut options = self.options.clone();
        if let Some(folder) = folder {
            options = options.with_working_directory(folder.to_path_buf());
        }

        SessionKind::from_local_options(options)
    }

    /// Build the session of the shell at `shell_path` from its launch
    /// `options`, named after the shell.
    pub fn from_options(
        shell_path: &str,
        options: LocalSessionOptions,
    ) -> Self {
        Self {
            name: shell_name(shell_path),
            options,
        }
    }

    /// Build the session of the shell at `shell_path`, started in the home
    /// folder, without the OTTY shell integration.
    pub fn without_integration(shell_path: &str) -> Self {
        Self::from_options(shell_path, shell_session_options(shell_path))
    }
}

/// Return the launch options of the shell at `shell_path`: that program,
/// started in the home folder.
///
/// # Examples
///
/// ```
/// use otty_app_shell::shell_session_options;
///
/// let options = shell_session_options("/bin/zsh");
///
/// assert_eq!(options.program(), "/bin/zsh");
/// ```
pub fn shell_session_options(shell_path: &str) -> LocalSessionOptions {
    let options = LocalSessionOptions::default().with_program(shell_path);

    if let Some(home) = env::var_os("HOME") {
        return options.with_working_directory(PathBuf::from(home));
    }

    options
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use otty_ui_term::settings::{LocalSessionOptions, SessionKind};

    use super::ShellSession;

    #[test]
    fn given_shell_session_when_constructed_then_getters_return_values() {
        let options = LocalSessionOptions::default().with_program("/bin/sh");
        let session = ShellSession::from_options("/bin/sh", options);

        assert_eq!(session.name(), "sh");
        match session.session(None) {
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
        match session.session(None) {
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

        match session.session(None) {
            SessionKind::Local(options) => {
                assert_eq!(options.working_directory(), &home);
                assert!(options.envs().get("ZDOTDIR").is_none());
            },
            SessionKind::Ssh(_) => {
                panic!("expected local session kind");
            },
        }
    }

    /// A session asked for in a folder starts there and keeps the rest of
    /// the shell's options: its program, arguments and environment.
    #[test]
    fn a_session_in_a_folder_starts_there_and_keeps_the_shell_options() {
        let options = LocalSessionOptions::default()
            .with_program("/bin/zsh")
            .with_args(vec![String::from("-l")])
            .with_env("ZDOTDIR", "/tmp/otty-integration")
            .with_working_directory(PathBuf::from("/home/user"));
        let shell = ShellSession::from_options("/bin/zsh", options);

        let session = shell.session(Some(Path::new("/tmp/project/src")));

        let SessionKind::Local(options) = session else {
            panic!("expected local session kind");
        };
        assert_eq!(options.program(), "/bin/zsh");
        assert_eq!(options.args(), ["-l"]);
        assert_eq!(
            options.envs().get("ZDOTDIR").map(String::as_str),
            Some("/tmp/otty-integration")
        );
        assert_eq!(
            options.working_directory(),
            &Some(PathBuf::from("/tmp/project/src"))
        );
    }

    /// Without a folder the session starts where its options say, the home
    /// folder for a shell of the user.
    #[test]
    fn a_session_without_a_folder_starts_in_the_folder_of_its_options() {
        let options = LocalSessionOptions::default()
            .with_program("/bin/sh")
            .with_working_directory(PathBuf::from("/home/user"));
        let shell = ShellSession::from_options("/bin/sh", options);

        let SessionKind::Local(options) = shell.session(None) else {
            panic!("expected local session kind");
        };
        assert_eq!(
            options.working_directory(),
            &Some(PathBuf::from("/home/user"))
        );
    }
}
