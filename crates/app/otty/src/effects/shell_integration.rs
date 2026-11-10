//! Writing the OTTY shell integration files of the app's shell, at start and
//! when the shell changes in Settings. The files and the session come from
//! `otty-app-shell`; here they are only written in the background.

#![allow(
    clippy::disallowed_methods,
    reason = "effects/ is where the binary starts background work"
)]

use iced::Task;
use otty_app_shell::{integrated_shell_session, shell_integration_dir};

use crate::message::AppEvent;

/// Write the shell integration files for the shell at `shell_path` in the
/// background, and send the session new terminals start to the app as
/// [`AppEvent::ShellIntegrated`].
pub(crate) fn integrate(shell_path: String) -> Task<AppEvent> {
    Task::perform(
        async move {
            integrated_shell_session(&shell_path, &shell_integration_dir())
        },
        AppEvent::ShellIntegrated,
    )
}
