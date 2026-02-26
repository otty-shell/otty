use iced::Task;

use crate::app::{App, AppEvent};
use crate::routers;
use crate::widgets::quick_launch::QuickLaunchCommand;
use crate::widgets::quick_launch::model::{NodePath, QuickLaunch};
use crate::widgets::tabs::TabsCommand;

/// Open a wizard tab in create mode for a quick launch command.
pub(crate) fn open_wizard_create_tab(
    app: &mut App,
    parent_path: NodePath,
) -> Task<AppEvent> {
    let title = String::from("Create Quick Launch");
    let tab_task = routers::tabs::route_command(
        app,
        TabsCommand::OpenWizardTab {
            title: title.clone(),
        },
    );

    let init_task = routers::quick_launch::route_command(
        app,
        QuickLaunchCommand::WizardInitializeCreate {
            tab_id: 0,
            parent_path,
        },
    );

    Task::batch([tab_task, init_task])
}

/// Open a wizard tab in edit mode for an existing command.
pub(crate) fn open_wizard_edit_tab(
    app: &mut App,
    path: NodePath,
    command: Box<QuickLaunch>,
) -> Task<AppEvent> {
    let title = format!("Edit: {}", command.title());
    let tab_task =
        routers::tabs::route_command(app, TabsCommand::OpenWizardTab { title });

    let init_task = routers::quick_launch::route_command(
        app,
        QuickLaunchCommand::WizardInitializeEdit {
            tab_id: 0,
            path,
            command,
        },
    );

    Task::batch([tab_task, init_task])
}

/// Open a terminal tab from a prepared quick launch command.
pub(crate) fn open_command_terminal_tab(
    app: &mut App,
    title: String,
    _settings: otty_ui_term::settings::Settings,
    _command: QuickLaunch,
) -> Task<AppEvent> {
    let terminal_id = app.widgets.terminal_workspace.allocate_terminal_id();
    routers::tabs::route_command(
        app,
        TabsCommand::OpenTerminalTab { terminal_id, title },
    )
}

/// Open an error tab for a failed quick launch.
pub(crate) fn open_error_tab(
    app: &mut App,
    title: String,
    message: String,
) -> Task<AppEvent> {
    let tab_task = routers::tabs::route_command(
        app,
        TabsCommand::OpenErrorTab {
            title: title.clone(),
        },
    );

    let init_task = routers::quick_launch::route_command(
        app,
        QuickLaunchCommand::OpenErrorTab {
            tab_id: 0,
            title,
            message,
        },
    );

    Task::batch([tab_task, init_task])
}

/// Close a tab by id.
pub(crate) fn close_tab(app: &mut App, tab_id: u64) -> Task<AppEvent> {
    routers::tabs::route_command(app, TabsCommand::Close { tab_id })
}
