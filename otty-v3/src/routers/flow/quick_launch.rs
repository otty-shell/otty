use iced::Task;

use crate::app::{App, AppFlowEvent, AppEvent};
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
                AppEvent::Flow(AppFlowEvent::QuickLaunchWizardCreateTabOpened {
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
                    AppFlowEvent::QuickLaunchWizardEditTabOpened {
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
        TabsCommand::OpenCommandTab {
            terminal_id,
            title: title.clone(),
            settings: Box::new(settings.clone()),
        },
    )
    .map(move |event| match event {
        AppEvent::TabsEffect(TabsEffect::CommandTabOpened {
            tab_id,
            terminal_id,
            title,
            ..
        }) => AppEvent::Flow(
            AppFlowEvent::QuickLaunchCommandTerminalTabOpened {
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
                AppFlowEvent::QuickLaunchErrorTabOpened {
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

#[cfg(test)]
mod tests {
    use super::{
        error_tab_opened, open_error_tab, open_wizard_create_tab,
        wizard_create_tab_opened,
    };
    use crate::app::App;
    use crate::widgets::quick_launch::QuickLaunchCommand;

    #[test]
    fn given_wizard_create_flow_when_opened_then_editor_state_is_initialized() {
        let (mut app, _) = App::new();
        app.widgets.quick_launch =
            crate::widgets::quick_launch::QuickLaunchWidget::new();

        let _ = open_wizard_create_tab(&mut app, Vec::new());
        let tab_id = app
            .widgets
            .tabs
            .active_tab_id()
            .expect("wizard tab should be active");

        let _ = wizard_create_tab_opened(&mut app, tab_id, Vec::new());

        let editor = app.widgets.quick_launch.wizard_editor(tab_id);
        assert!(editor.is_some());

        let _ = crate::routers::quick_launch::route_command(
            &mut app,
            QuickLaunchCommand::WizardUpdateTitle {
                tab_id,
                value: String::from("Demo"),
            },
        );
        let _ = crate::routers::quick_launch::route_command(
            &mut app,
            QuickLaunchCommand::WizardUpdateProgram {
                tab_id,
                value: String::from("bash"),
            },
        );
        let _ = crate::routers::quick_launch::route_command(
            &mut app,
            QuickLaunchCommand::WizardSave { tab_id },
        );

        let path = vec![String::from("Demo")];
        assert!(
            app.widgets
                .quick_launch
                .tree_vm()
                .data
                .node(&path)
                .is_some()
        );
    }

    #[test]
    fn given_error_tab_flow_when_initialized_and_closed_then_payload_is_removed()
     {
        let (mut app, _) = App::new();
        app.widgets.quick_launch =
            crate::widgets::quick_launch::QuickLaunchWidget::new();

        let _ = open_error_tab(
            &mut app,
            String::from("Failed"),
            String::from("boom"),
        );
        let tab_id = app
            .widgets
            .tabs
            .active_tab_id()
            .expect("error tab should be active");

        let _ = error_tab_opened(
            &mut app,
            tab_id,
            String::from("Failed"),
            String::from("boom"),
        );
        assert!(app.widgets.quick_launch.error_tab(tab_id).is_some());

        let _ = crate::routers::quick_launch::route_command(
            &mut app,
            QuickLaunchCommand::TabClosed { tab_id },
        );
        assert!(app.widgets.quick_launch.error_tab(tab_id).is_none());
    }
}
