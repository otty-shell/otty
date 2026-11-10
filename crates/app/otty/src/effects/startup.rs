//! The startup work: the I/O the app does once, before the first tab opens.

#![allow(
    clippy::disallowed_methods,
    reason = "effects/ is where the binary starts background work"
)]

use iced::Task;

use super::host_name::read_host_name;
use super::shell_integration::{
    integrated_shell_session, shell_integration_dir,
};
use crate::message::AppEvent;

/// Write the shell integration files for the shell at `shell_path`, then
/// read the host name, and send both answers to the app in this order. The
/// tabs feature (issue 10) chains the first tab after them, so the first
/// terminal already starts with shell integration.
pub(crate) fn run(shell_path: String) -> Task<AppEvent> {
    let shell_session = Task::perform(
        async move {
            integrated_shell_session(&shell_path, &shell_integration_dir())
        },
        AppEvent::ShellIntegrated,
    );
    let host_name =
        Task::perform(async { read_host_name() }, AppEvent::HostNameRead);

    shell_session.chain(host_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::task_outputs::collect_outputs;

    /// A shell without OTTY shell integration, so building its session
    /// writes no files.
    const TEST_SHELL_PATH: &str = "/opt/otty-test/otty-test-shell";

    /// The first terminal starts with the shell session in the app, so the
    /// shell session goes to the app first, then the host name.
    #[test]
    fn the_shell_session_and_then_the_host_name_go_to_the_app() {
        let events = collect_outputs(run(String::from(TEST_SHELL_PATH)));

        assert!(matches!(
            events.as_slice(),
            [
                AppEvent::ShellIntegrated(session),
                AppEvent::HostNameRead(host_name),
            ] if session.name() == "otty-test-shell"
                && *host_name == read_host_name()
        ));
    }
}
