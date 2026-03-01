use iced::Task;
use iced::widget::operation::snap_to_end;

use crate::app::{App, AppEvent, PendingQuickLaunchWizard};
use crate::widgets::explorer::ExplorerCommand;
use crate::widgets::quick_launch::QuickLaunchCommand;
use crate::widgets::settings::SettingsCommand;
use crate::widgets::tabs::view::tab_bar::TAB_BAR_SCROLL_ID;
use crate::widgets::tabs::{TabsCommand, TabsEffect, TabsEvent};
use crate::widgets::terminal_workspace::TerminalWorkspaceCommand;
use crate::widgets::terminal_workspace::model::TerminalKind;
use crate::widgets::terminal_workspace::services::terminal_settings_for_session;

/// Route a tabs UI event through the widget reducer and map effects.
pub(crate) fn route_event(app: &mut App, event: TabsEvent) -> Task<AppEvent> {
    let command = map_tabs_event_to_command(event);
    route_command(app, command)
}

/// Route a tabs command through the widget reducer and map effects.
pub(crate) fn route_command(
    app: &mut App,
    command: TabsCommand,
) -> Task<AppEvent> {
    app.widgets.tabs.reduce(command).map(AppEvent::TabsEffect)
}

/// Route a tabs effect event to an app-level task.
pub(crate) fn route_effect(
    app: &mut App,
    effect: TabsEffect,
) -> Task<AppEvent> {
    match effect {
        TabsEffect::Activated { tab_id } => {
            let mut tasks: Vec<Task<AppEvent>> = Vec::new();

            // Focus the terminal in the activated tab.
            tasks.push(Task::done(AppEvent::TerminalWorkspaceCommand(
                TerminalWorkspaceCommand::FocusActive,
            )));
            tasks.push(Task::done(AppEvent::TerminalWorkspaceCommand(
                TerminalWorkspaceCommand::SyncSelection { tab_id },
            )));

            // Sync explorer from terminal CWD.
            let active_tab_id = Some(tab_id);
            if let Some(cwd) = app
                .widgets
                .terminal_workspace
                .shell_cwd_for_active_tab(active_tab_id)
            {
                tasks.push(Task::done(AppEvent::ExplorerCommand(
                    ExplorerCommand::SyncRoot { cwd },
                )));
            }

            Task::batch(tasks)
        },
        TabsEffect::Closed {
            tab_id,
            new_active_id,
            remaining,
        } => {
            let mut tasks: Vec<Task<AppEvent>> = Vec::new();

            // Notify terminal workspace of tab closure.
            tasks.push(Task::done(AppEvent::TerminalWorkspaceCommand(
                TerminalWorkspaceCommand::TabClosed { tab_id },
            )));

            // Notify quick launch of tab closure.
            tasks.push(Task::done(AppEvent::QuickLaunchCommand(
                QuickLaunchCommand::TabClosed { tab_id },
            )));

            // Sync explorer for the new active tab.
            if let Some(active_id) = new_active_id {
                if let Some(cwd) = app
                    .widgets
                    .terminal_workspace
                    .shell_cwd_for_active_tab(Some(active_id))
                {
                    tasks.push(Task::done(AppEvent::ExplorerCommand(
                        ExplorerCommand::SyncRoot { cwd },
                    )));
                }
            }

            if remaining > 0 {
                tasks.push(Task::done(AppEvent::TerminalWorkspaceCommand(
                    TerminalWorkspaceCommand::FocusActive,
                )));
                if let Some(active_id) = new_active_id {
                    tasks.push(Task::done(AppEvent::TerminalWorkspaceCommand(
                        TerminalWorkspaceCommand::SyncSelection {
                            tab_id: active_id,
                        },
                    )));
                }
            }

            Task::batch(tasks)
        },
        TabsEffect::TerminalTabOpened {
            tab_id,
            terminal_id,
            title,
        } => {
            let settings = Box::new(terminal_settings_for_session(
                &app.terminal_settings,
                app.shell_session.session().clone(),
            ));

            Task::done(AppEvent::TerminalWorkspaceCommand(
                TerminalWorkspaceCommand::OpenTab {
                    tab_id,
                    terminal_id,
                    default_title: title,
                    settings,
                    kind: TerminalKind::Shell,
                    sync_explorer: true,
                },
            ))
        },
        TabsEffect::CommandTabOpened {
            tab_id,
            terminal_id,
            title,
            settings,
        } => Task::done(AppEvent::TerminalWorkspaceCommand(
            TerminalWorkspaceCommand::OpenTab {
                tab_id,
                terminal_id,
                default_title: title,
                settings,
                kind: TerminalKind::Command,
                sync_explorer: false,
            },
        )),
        TabsEffect::SettingsTabOpened => {
            Task::done(AppEvent::SettingsCommand(SettingsCommand::Reload))
        },
        TabsEffect::WizardTabOpened { tab_id } => {
            match app.pending_workflows.pop_quick_launch_wizard() {
                Some(PendingQuickLaunchWizard::Create { parent_path }) => {
                    Task::done(AppEvent::QuickLaunchCommand(
                        QuickLaunchCommand::WizardInitializeCreate {
                            tab_id,
                            parent_path,
                        },
                    ))
                },
                Some(PendingQuickLaunchWizard::Edit { path, command }) => {
                    Task::done(AppEvent::QuickLaunchCommand(
                        QuickLaunchCommand::WizardInitializeEdit {
                            tab_id,
                            path,
                            command,
                        },
                    ))
                },
                None => Task::none(),
            }
        },
        TabsEffect::ErrorTabOpened { tab_id } => {
            let Some(payload) =
                app.pending_workflows.pop_quick_launch_error_tab()
            else {
                return Task::none();
            };
            let (title, message) = payload.into_parts();
            Task::done(AppEvent::QuickLaunchCommand(
                QuickLaunchCommand::OpenErrorTab {
                    tab_id,
                    title,
                    message,
                },
            ))
        },
        TabsEffect::ScrollBarToEnd => snap_to_end(TAB_BAR_SCROLL_ID),
    }
}

fn map_tabs_event_to_command(event: TabsEvent) -> TabsCommand {
    match event {
        TabsEvent::ActivateTab { tab_id } => TabsCommand::Activate { tab_id },
        TabsEvent::CloseTab { tab_id } => TabsCommand::Close { tab_id },
    }
}

#[cfg(test)]
mod tests {
    use super::route_effect;
    use crate::app::{App, PendingQuickLaunchWizard};
    use crate::widgets::tabs::TabsEffect;

    #[test]
    fn given_wizard_opened_effect_when_pending_wizard_exists_then_queue_is_consumed()
     {
        let (mut app, _) = App::new();
        app.pending_workflows
            .push_quick_launch_wizard_create(vec![String::from("Demo")]);

        let _ =
            route_effect(&mut app, TabsEffect::WizardTabOpened { tab_id: 7 });

        assert!(app.pending_workflows.pop_quick_launch_wizard().is_none());
    }

    #[test]
    fn given_error_tab_opened_effect_when_pending_payload_exists_then_queue_is_consumed()
     {
        let (mut app, _) = App::new();
        app.pending_workflows.push_quick_launch_error_tab(
            String::from("Failed"),
            String::from("boom"),
        );

        let _ =
            route_effect(&mut app, TabsEffect::ErrorTabOpened { tab_id: 11 });

        assert!(app.pending_workflows.pop_quick_launch_error_tab().is_none());
    }

    #[test]
    fn given_wizard_opened_effect_without_pending_then_queue_stays_empty() {
        let (mut app, _) = App::new();

        let _ =
            route_effect(&mut app, TabsEffect::WizardTabOpened { tab_id: 1 });

        match app.pending_workflows.pop_quick_launch_wizard() {
            Some(PendingQuickLaunchWizard::Create { .. })
            | Some(PendingQuickLaunchWizard::Edit { .. }) => {
                panic!("queue should stay empty")
            },
            None => {},
        }
    }
}
