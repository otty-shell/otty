//! The startup work: the I/O the app does once, before the first tab opens.
//! It writes the shell integration files and reads this computer's host
//! name. After the first tab it asks Containers mode and Agents mode to read
//! what they show, so they are ready when they open (spec Decision 10 of
//! Agents mode). This is the only file of the binary that starts background
//! work itself; a feature's I/O lives in its `io` crate.

#![allow(
    clippy::disallowed_methods,
    reason = "startup.rs is where the binary starts background work"
)]

use std::path::PathBuf;

use iced::Task;
use otty_app_agents::AgentsIntent;
use otty_app_containers::ContainersIntent;
use otty_app_plugins::PluginsIntent;
use otty_app_tabs::TabsIntent;

use crate::message::AppEvent;

/// Write the shell integration files for the shell at `shell_path` to
/// `config_dir`, then read the host name, send both answers to the app, and
/// then ask for the first tab. The events arrive in this order, so the first
/// terminal already starts with shell integration and its machine group has
/// its name. Last, Containers mode starts looking for the engines and Agents
/// mode reading the profiles, both with the mode away: the agents' reads
/// find the programs through the shell session, so they come after it.
/// Then the daily update check of the plugins runs when it is due. OTTY
/// installs no plugin by itself: the user installs them on the Plugins
/// page.
pub(crate) fn run(shell_path: String, config_dir: PathBuf) -> Task<AppEvent> {
    let shell_session = otty_app_shell::integrate(shell_path, config_dir)
        .map(AppEvent::ShellIntegrated);
    let host_name =
        Task::perform(async { read_host_name() }, AppEvent::HostNameRead);
    let first_tab = Task::done(AppEvent::Tabs(TabsIntent::OpenTerminalTab {
        folder: None,
    }));
    let containers =
        Task::done(AppEvent::Containers(ContainersIntent::LoadInBackground));
    let agents = Task::done(AppEvent::Agents(AgentsIntent::LoadInBackground));
    let update_check = Task::done(AppEvent::Plugins(PluginsIntent::UpdateTick));

    shell_session
        .chain(host_name)
        .chain(first_tab)
        .chain(containers)
        .chain(agents)
        .chain(update_check)
}

/// Read this computer's host name, such as "fedora". Returns `None` when it
/// cannot be read; the status bar then calls the machine "This computer".
fn read_host_name() -> Option<String> {
    #[cfg(unix)]
    {
        match nix::unistd::gethostname() {
            Ok(name) => name.into_string().ok().filter(|name| !name.is_empty()),
            Err(err) => {
                log::warn!("host name read failed: {err}");
                None
            },
        }
    }

    #[cfg(not(unix))]
    {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::task_outputs::collect_outputs;
    use crate::testing::window::test_config_dir;

    /// A shell without OTTY shell integration, so building its session
    /// writes no files.
    const TEST_SHELL_PATH: &str = "/opt/otty-test/otty-test-shell";

    /// The first terminal starts with the shell session in the app, so the
    /// shell session and the host name go to the app before the first tab
    /// is asked for. Then Containers mode and Agents mode start reading
    /// what they show, with neither mode on screen; the agents' reads need
    /// the shell session to find the programs. Last the daily update check
    /// of the plugins is asked for; OTTY installs no plugin by itself
    /// (issue 28 of spec 2026-11-19-agent-plugins).
    #[test]
    fn the_shell_session_and_host_name_go_to_the_app_before_the_first_tab() {
        let events = collect_outputs(run(
            String::from(TEST_SHELL_PATH),
            test_config_dir(),
        ));

        assert!(matches!(
            events.as_slice(),
            [
                AppEvent::ShellIntegrated(session),
                AppEvent::HostNameRead(host_name),
                AppEvent::Tabs(TabsIntent::OpenTerminalTab { folder: None }),
                AppEvent::Containers(ContainersIntent::LoadInBackground),
                AppEvent::Agents(AgentsIntent::LoadInBackground),
                AppEvent::Plugins(PluginsIntent::UpdateTick),
            ] if session.name() == "otty-test-shell"
                && *host_name == read_host_name()
        ));
    }
}
