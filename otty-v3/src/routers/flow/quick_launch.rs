use iced::Task;

use crate::app::{App, AppEvent};
use crate::routers;
use crate::widgets::quick_launch::QuickLaunchCommand;
use crate::widgets::quick_launch::model::{NodePath, QuickLaunch};
use crate::widgets::tabs::{TabsCommand, TabsEffect};
use crate::widgets::terminal_workspace::TerminalWorkspaceCommand;
use crate::widgets::terminal_workspace::model::TerminalKind;

/// Open a wizard tab in create mode for a quick launch command.
pub(crate) fn open_wizard_create_tab(
    app: &mut App,
    parent_path: NodePath,
) -> Task<AppEvent> {
    let title = String::from("Create Quick Launch");
    routers::tabs::route_command(app, TabsCommand::OpenWizardTab { title })
        .map(move |event| match event {
            AppEvent::TabsEffect(TabsEffect::WizardTabOpened { tab_id }) => {
                AppEvent::Flow(crate::app::AppFlowEvent::QuickLaunchWizardCreateTabOpened {
                    tab_id,
                    parent_path: parent_path.clone(),
                })
            },
            _ => event,
        })
}

/// Open a wizard tab in edit mode for an existing command.
pub(crate) fn open_wizard_edit_tab(
    app: &mut App,
    path: NodePath,
    command: Box<QuickLaunch>,
) -> Task<AppEvent> {
    let title = format!("Edit: {}", command.title());
    routers::tabs::route_command(app, TabsCommand::OpenWizardTab { title }).map(
        move |event| match event {
            AppEvent::TabsEffect(TabsEffect::WizardTabOpened { tab_id }) => {
                AppEvent::Flow(
                    crate::app::AppFlowEvent::QuickLaunchWizardEditTabOpened {
                        tab_id,
                        path: path.clone(),
                        command: command.clone(),
                    },
                )
            },
            _ => event,
        },
    )
}

/// Initialize the quick launch wizard create editor for an opened tab.
pub(crate) fn wizard_create_tab_opened(
    app: &mut App,
    tab_id: u64,
    parent_path: NodePath,
) -> Task<AppEvent> {
    routers::quick_launch::route_command(
        app,
        QuickLaunchCommand::WizardInitializeCreate {
            tab_id,
            parent_path,
        },
    )
}

/// Initialize the quick launch wizard edit editor for an opened tab.
pub(crate) fn wizard_edit_tab_opened(
    app: &mut App,
    tab_id: u64,
    path: NodePath,
    command: Box<QuickLaunch>,
) -> Task<AppEvent> {
    routers::quick_launch::route_command(
        app,
        QuickLaunchCommand::WizardInitializeEdit {
            tab_id,
            path,
            command,
        },
    )
}

/// Open a terminal tab from a prepared quick launch command.
pub(crate) fn open_command_terminal_tab(
    app: &mut App,
    title: String,
    settings: otty_ui_term::settings::Settings,
    _command: QuickLaunch,
) -> Task<AppEvent> {
    let terminal_id = app.widgets.terminal_workspace.allocate_terminal_id();
    routers::tabs::route_command(
        app,
        TabsCommand::OpenTerminalTab {
            terminal_id,
            title: title.clone(),
        },
    )
    .map(move |event| match event {
        AppEvent::TabsEffect(TabsEffect::TerminalTabOpened {
            tab_id,
            terminal_id,
            title,
        }) => AppEvent::Flow(
            crate::app::AppFlowEvent::QuickLaunchCommandTerminalTabOpened {
                tab_id,
                terminal_id,
                title,
                settings: settings.clone(),
            },
        ),
        _ => event,
    })
}

/// Open the terminal workspace tab for a prepared quick launch command.
pub(crate) fn command_terminal_tab_opened(
    app: &mut App,
    tab_id: u64,
    terminal_id: u64,
    title: String,
    settings: otty_ui_term::settings::Settings,
) -> Task<AppEvent> {
    routers::terminal_workspace::route_command(
        app,
        TerminalWorkspaceCommand::OpenTab {
            tab_id,
            terminal_id,
            default_title: title,
            settings: Box::new(settings),
            kind: TerminalKind::Command,
            sync_explorer: false,
        },
    )
}

/// Open an error tab for a failed quick launch.
pub(crate) fn open_error_tab(
    app: &mut App,
    title: String,
    message: String,
) -> Task<AppEvent> {
    routers::tabs::route_command(
        app,
        TabsCommand::OpenErrorTab {
            title: title.clone(),
        },
    )
    .map(move |event| match event {
        AppEvent::TabsEffect(TabsEffect::ErrorTabOpened { tab_id }) => {
            AppEvent::Flow(
                crate::app::AppFlowEvent::QuickLaunchErrorTabOpened {
                    tab_id,
                    title: title.clone(),
                    message: message.clone(),
                },
            )
        },
        _ => event,
    })
}

/// Initialize quick launch error payload for an opened error tab.
pub(crate) fn error_tab_opened(
    app: &mut App,
    tab_id: u64,
    title: String,
    message: String,
) -> Task<AppEvent> {
    routers::quick_launch::route_command(
        app,
        QuickLaunchCommand::OpenErrorTab {
            tab_id,
            title,
            message,
        },
    )
}

/// Close a tab by id.
pub(crate) fn close_tab(app: &mut App, tab_id: u64) -> Task<AppEvent> {
    routers::tabs::route_command(app, TabsCommand::Close { tab_id })
}
