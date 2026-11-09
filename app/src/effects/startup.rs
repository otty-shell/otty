//! The startup effects: the I/O the app does once the window is ready,
//! before the first tab opens.

use iced::Task;

use crate::app::App;
use crate::dispatch::AppEvent;
use crate::services;
use crate::shell_integration::{
    integrated_shell_session, shell_integration_dir,
};
use crate::widgets::tabs::event::{TabsEvent, TabsIntent};

/// Write the shell integration files and read the host name, then open the
/// first tab, so its terminal already starts with shell integration.
pub(crate) fn run(app: &mut App) -> Task<AppEvent> {
    let shell_path = app.widgets.settings.saved().terminal_shell();
    app.shell_session =
        integrated_shell_session(shell_path, &shell_integration_dir());
    app.host_name = services::read_host_name();

    Task::done(AppEvent::Tabs(TabsEvent::Intent(
        TabsIntent::OpenTerminalTab,
    )))
}
