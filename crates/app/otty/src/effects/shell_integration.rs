//! Writing the OTTY shell integration files of the app's shell, at start and
//! when the shell changes in Settings. The files and the session come from
//! `otty-app-shell`; here they are only written in the background, to the
//! app's config folder.

#![allow(
    clippy::disallowed_methods,
    reason = "effects/ is where the binary starts background work"
)]

use std::path::PathBuf;

use iced::Task;
use otty_app_shell::integrated_shell_session;

use crate::message::AppEvent;

/// Write the shell integration files for the shell at `shell_path` to
/// `dir` in the background, and send the session new terminals start to the
/// app as [`AppEvent::ShellIntegrated`].
pub(crate) fn integrate(shell_path: String, dir: PathBuf) -> Task<AppEvent> {
    Task::perform(
        async move { integrated_shell_session(&shell_path, &dir) },
        AppEvent::ShellIntegrated,
    )
}
