use iced::Task;
use iced::widget::operation::snap_to_end;

use crate::app::App;
use crate::dispatch::AppEvent;
use crate::domain::quick_launch::WizardTabInit;
use crate::services::terminal_settings_for_session;
use crate::widgets::explorer::event::{ExplorerEvent, ExplorerIntent};
use crate::widgets::modes::event::{ModesEvent, ModesIntent};
use crate::widgets::quick_launch::event::{
    QuickLaunchEvent, QuickLaunchIntent,
};
use crate::widgets::tabs::event::TabsEffect;
use crate::widgets::tabs::view::session_list::SESSION_LIST_SCROLL_ID;
use crate::widgets::terminal_workspace::event::{
    TerminalWorkspaceEvent, TerminalWorkspaceIntent,
};
use crate::widgets::terminal_workspace::types::TerminalKind;

/// Run a tabs effect. Tabs only show in Terminal mode (spec Decision 5), so
/// a tab that becomes active and the launch editor that opens also tell the
/// modes widget, which switches to Terminal mode when another mode is shown.
/// A new shell or launch tab tells it from `effects/terminal_workspace.rs`
/// once its terminal starts, so a tab whose terminal cannot start leaves the
/// mode as it was.
pub(crate) fn run(app: &App, effect: TabsEffect) -> Task<AppEvent> {
    let shows_tab = matches!(
        effect,
        TabsEffect::Activated { .. } | TabsEffect::WizardTabOpened { .. }
    );

    let run = run_effect(app, effect);
    if !shows_tab {
        return run;
    }

    let tab_shown =
        Task::done(AppEvent::Modes(ModesEvent::Intent(ModesIntent::TabShown)));
    Task::batch([run, tab_shown])
}

fn run_effect(app: &App, effect: TabsEffect) -> Task<AppEvent> {
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
                .local_cwd_for_active_tab(active_tab_id)
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
            if let Some(active_id) = new_active_id
                && let Some(cwd) = app
                    .widgets
                    .terminal_workspace
                    .local_cwd_for_active_tab(Some(active_id))
            {
                tasks.push(Task::done(AppEvent::Explorer(
                    ExplorerEvent::Intent(ExplorerIntent::SyncRoot { cwd }),
                )));
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
        TabsEffect::TerminalTabOpened { tab_id, title } => {
            let settings = Box::new(terminal_settings_for_session(
                &app.terminal_settings,
                app.shell_session.session().clone(),
            ));

            Task::done(AppEvent::TerminalWorkspace(
                TerminalWorkspaceEvent::Intent(
                    TerminalWorkspaceIntent::OpenTab {
                        tab_id,
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
            title,
            settings,
            exit_behavior,
        } => Task::done(AppEvent::TerminalWorkspace(
            TerminalWorkspaceEvent::Intent(TerminalWorkspaceIntent::OpenTab {
                tab_id,
                default_title: title,
                settings,
                kind: TerminalKind::Command { exit_behavior },
                sync_explorer: true,
            }),
        )),
        TabsEffect::WizardTabOpened { tab_id, init } => match init {
            WizardTabInit::Create { parent_path } => {
                Task::done(AppEvent::QuickLaunch(QuickLaunchEvent::Intent(
                    QuickLaunchIntent::WizardInitializeCreate {
                        tab_id,
                        parent_path,
                    },
                )))
            },
            WizardTabInit::Edit { path, command } => {
                Task::done(AppEvent::QuickLaunch(QuickLaunchEvent::Intent(
                    QuickLaunchIntent::WizardInitializeEdit {
                        tab_id,
                        path,
                        command,
                    },
                )))
            },
        },
        TabsEffect::ScrollBarToEnd => snap_to_end(SESSION_LIST_SCROLL_ID),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::task_outputs::collect_outputs;

    #[test]
    fn given_wizard_opened_effect_when_create_init_exists_then_task_is_emitted()
    {
        let (app, _) = App::new();

        let events = collect_outputs(run(
            &app,
            TabsEffect::WizardTabOpened {
                tab_id: 7,
                init: WizardTabInit::Create {
                    parent_path: vec![String::from("Demo")],
                },
            },
        ));

        assert!(events.iter().any(|event| matches!(
            event,
            AppEvent::QuickLaunch(QuickLaunchEvent::Intent(
                QuickLaunchIntent::WizardInitializeCreate { tab_id: 7, .. }
            ))
        )));
        let tabs_shown = events
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    AppEvent::Modes(ModesEvent::Intent(ModesIntent::TabShown))
                )
            })
            .count();
        assert_eq!(tabs_shown, 1);
    }
}
