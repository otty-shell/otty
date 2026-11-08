use std::env;
use std::path::PathBuf;

use otty_ui_term::settings::{LocalSessionOptions, SessionKind};

use super::errors::TerminalWorkspaceError;
use super::types::ShellSession;
use crate::shell_integration::{
    shell_integration_dir, shell_name, with_shell_integration,
};

/// Build a shell session configured with OTTY integrations for a given shell.
pub(crate) fn setup_shell_session_with_shell(
    shell_path: &str,
) -> Result<ShellSession, TerminalWorkspaceError> {
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
pub(crate) fn fallback_shell_session_with_shell(
    shell_path: &str,
) -> ShellSession {
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

    use otty_ui_term::settings::SessionKind;

    use super::fallback_shell_session_with_shell;

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
