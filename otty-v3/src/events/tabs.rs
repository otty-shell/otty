use iced::Task;
use iced::widget::operation::snap_to_end;

use super::AppEvent;
use crate::app::{App, PendingQuickLaunchWizard};
use crate::widgets::explorer::{ExplorerEvent, ExplorerIntent};
use crate::widgets::quick_launch::{QuickLaunchEvent, QuickLaunchIntent};
use crate::widgets::settings::{SettingsEvent, SettingsIntent};
use crate::widgets::tabs::view::tab_bar::TAB_BAR_SCROLL_ID;
use crate::widgets::tabs::{TabsEffect, TabsEvent};
use crate::widgets::terminal_workspace::model::TerminalKind;
use crate::widgets::terminal_workspace::services::terminal_settings_for_session;
use crate::widgets::terminal_workspace::{
    TerminalWorkspaceEvent, TerminalWorkspaceIntent,
};

pub(crate) fn handle(app: &mut App, event: TabsEvent) -> Task<AppEvent> {
    match event {
        TabsEvent::Intent(event) => {
            app.widgets.tabs.reduce(event).map(AppEvent::Tabs)
        },
        TabsEvent::Effect(effect) => handle_effect(app, effect),
    }
}

fn handle_effect(app: &mut App, effect: TabsEffect) -> Task<AppEvent> {
    match effect {
        TabsEffect::Activated { tab_id } => {
            let mut tasks: Vec<Task<AppEvent>> = Vec::new();

            // Focus the terminal in the activated tab.
            tasks.push(Task::done(AppEvent::TerminalWorkspace(
                TerminalWorkspaceEvent::Intent(
                    TerminalWorkspaceIntent::FocusActive,
                ),
            )));
            tasks.push(Task::done(AppEvent::TerminalWorkspace(
                TerminalWorkspaceEvent::Intent(
                    TerminalWorkspaceIntent::SyncSelection { tab_id },
                ),
            )));

            // Sync explorer from terminal CWD.
            let active_tab_id = Some(tab_id);
            if let Some(cwd) = app
                .widgets
                .terminal_workspace
                .shell_cwd_for_active_tab(active_tab_id)
            {
                tasks.push(Task::done(AppEvent::Explorer(
                    ExplorerEvent::Intent(ExplorerIntent::SyncRoot { cwd }),
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
            tasks.push(Task::done(AppEvent::TerminalWorkspace(
                TerminalWorkspaceEvent::Intent(
                    TerminalWorkspaceIntent::TabClosed { tab_id },
                ),
            )));

            // Notify quick launch of tab closure.
            tasks.push(Task::done(AppEvent::QuickLaunch(
                QuickLaunchEvent::Intent(QuickLaunchIntent::TabClosed {
                    tab_id,
                }),
            )));

            // Sync explorer for the new active tab.
            if let Some(active_id) = new_active_id {
                if let Some(cwd) = app
                    .widgets
                    .terminal_workspace
                    .shell_cwd_for_active_tab(Some(active_id))
                {
                    tasks.push(Task::done(AppEvent::Explorer(
                        ExplorerEvent::Intent(ExplorerIntent::SyncRoot { cwd }),
                    )));
                }
            }

            if remaining > 0 {
                tasks.push(Task::done(AppEvent::TerminalWorkspace(
                    TerminalWorkspaceEvent::Intent(
                        TerminalWorkspaceIntent::FocusActive,
                    ),
                )));
                if let Some(active_id) = new_active_id {
                    tasks.push(Task::done(AppEvent::TerminalWorkspace(
                        TerminalWorkspaceEvent::Intent(
                            TerminalWorkspaceIntent::SyncSelection {
                                tab_id: active_id,
                            },
                        ),
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

            Task::done(AppEvent::TerminalWorkspace(
                TerminalWorkspaceEvent::Intent(
                    TerminalWorkspaceIntent::OpenTab {
                        tab_id,
                        terminal_id,
                        default_title: title,
                        settings,
                        kind: TerminalKind::Shell,
                        sync_explorer: true,
                    },
                ),
            ))
        },
        TabsEffect::CommandTabOpened {
            tab_id,
            terminal_id,
            title,
            settings,
        } => Task::done(AppEvent::TerminalWorkspace(
            TerminalWorkspaceEvent::Intent(TerminalWorkspaceIntent::OpenTab {
                tab_id,
                terminal_id,
                default_title: title,
                settings,
                kind: TerminalKind::Command,
                sync_explorer: false,
            }),
        )),
        TabsEffect::SettingsTabOpened => Task::done(AppEvent::Settings(
            SettingsEvent::Intent(SettingsIntent::Reload),
        )),
        TabsEffect::WizardTabOpened { tab_id } => {
            match app.pending_workflows.pop_quick_launch_wizard() {
                Some(PendingQuickLaunchWizard::Create { parent_path }) => {
                    Task::done(AppEvent::QuickLaunch(QuickLaunchEvent::Intent(
                        QuickLaunchIntent::WizardInitializeCreate {
                            tab_id,
                            parent_path,
                        },
                    )))
                },
                Some(PendingQuickLaunchWizard::Edit { path, command }) => {
                    Task::done(AppEvent::QuickLaunch(QuickLaunchEvent::Intent(
                        QuickLaunchIntent::WizardInitializeEdit {
                            tab_id,
                            path,
                            command,
                        },
                    )))
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
            Task::done(AppEvent::QuickLaunch(QuickLaunchEvent::Intent(
                QuickLaunchIntent::OpenErrorTab {
                    tab_id,
                    title,
                    message,
                },
            )))
        },
        TabsEffect::ScrollBarToEnd => snap_to_end(TAB_BAR_SCROLL_ID),
    }
}

#[cfg(test)]
mod tests {
    use super::handle;
    use crate::app::{App, PendingQuickLaunchWizard};
    use crate::widgets::tabs::{TabsEffect, TabsEvent};

    #[test]
    fn given_wizard_opened_effect_when_pending_wizard_exists_then_queue_is_consumed()
     {
        let (mut app, _) = App::new();
        app.pending_workflows
            .push_quick_launch_wizard_create(vec![String::from("Demo")]);

        let _ = handle(
            &mut app,
            TabsEvent::Effect(TabsEffect::WizardTabOpened { tab_id: 7 }),
        );

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

        let _ = handle(
            &mut app,
            TabsEvent::Effect(TabsEffect::ErrorTabOpened { tab_id: 11 }),
        );

        assert!(app.pending_workflows.pop_quick_launch_error_tab().is_none());
    }

    #[test]
    fn given_wizard_opened_effect_without_pending_then_queue_stays_empty() {
        let (mut app, _) = App::new();

        let _ = handle(
            &mut app,
            TabsEvent::Effect(TabsEffect::WizardTabOpened { tab_id: 1 }),
        );

        match app.pending_workflows.pop_quick_launch_wizard() {
            Some(PendingQuickLaunchWizard::Create { .. })
            | Some(PendingQuickLaunchWizard::Edit { .. }) => {
                panic!("queue should stay empty")
            },
            None => {},
        }
    }
}
