use std::path::PathBuf;

use iced::Task;

use crate::app::{App, AppEvent};
use crate::routers;
use crate::widgets::tabs::TabsCommand;

/// Orchestrate opening a new terminal tab.
///
/// Allocates a terminal_id from the terminal workspace widget
/// and uses the configured shell name as the tab title.
pub(crate) fn open_terminal_tab(app: &mut App) -> Task<AppEvent> {
    let terminal_id = app.widgets.terminal_workspace.allocate_terminal_id();
    let title = String::from("Terminal");

    routers::tabs::route_command(
        app,
        TabsCommand::OpenTerminalTab { terminal_id, title },
    )
}

/// Orchestrate opening a new settings tab.
pub(crate) fn open_settings_tab(app: &mut App) -> Task<AppEvent> {
    routers::tabs::route_command(app, TabsCommand::OpenSettingsTab)
}

/// Orchestrate opening a file in a new terminal tab (from explorer).
pub(crate) fn open_file_terminal_tab(
    app: &mut App,
    file_path: PathBuf,
) -> Task<AppEvent> {
    let terminal_id = app.widgets.terminal_workspace.allocate_terminal_id();
    let title = file_path
        .file_name()
        .and_then(|name| name.to_str())
        .map(ToString::to_string)
        .unwrap_or_else(|| String::from("File"));

    routers::tabs::route_command(
        app,
        TabsCommand::OpenTerminalTab { terminal_id, title },
    )
}
