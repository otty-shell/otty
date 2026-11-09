//! The startup effects: the I/O the app does once the window is ready,
//! before the first tab opens.

use iced::Task;

use crate::app::App;
use crate::event::AppEvent;
use crate::services;
use crate::shell_integration::{
    integrated_shell_session, shell_integration_dir,
};
use crate::widgets::tabs::event::{TabsEvent, TabsIntent};

/// Write the shell integration files and read the host name, send both to
/// the app, then open the first tab. The events arrive in this order, so the
/// first terminal already starts with shell integration.
pub(crate) fn run(app: &App) -> Task<AppEvent> {
    let shell_path = app.widgets.settings.saved().terminal_shell();
    let shell_session =
        integrated_shell_session(shell_path, &shell_integration_dir());
    let host_name = services::read_host_name();

    Task::done(AppEvent::ShellIntegrated(shell_session))
        .chain(Task::done(AppEvent::HostNameRead(host_name)))
        .chain(Task::done(AppEvent::Tabs(TabsEvent::Intent(
            TabsIntent::OpenTerminalTab,
        ))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::task_outputs::collect_outputs;

    /// The first terminal starts with the shell session in the app, so the
    /// shell session and the host name go to the app before the first tab
    /// is asked for.
    #[test]
    fn the_shell_session_and_host_name_go_to_the_app_before_the_first_tab() {
        let (app, _) = App::new();

        let events = collect_outputs(run(&app));

        assert!(matches!(
            events.as_slice(),
            [
                AppEvent::ShellIntegrated(_),
                AppEvent::HostNameRead(host_name),
                AppEvent::Tabs(TabsEvent::Intent(TabsIntent::OpenTerminalTab)),
            ] if *host_name == services::read_host_name()
        ));
    }
}
