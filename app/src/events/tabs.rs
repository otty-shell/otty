use iced::Task;
use iced::widget::operation::snap_to_end;

use super::AppEvent;
use crate::app::App;
use crate::domain::quick_launch::WizardTabInit;
use crate::services::terminal_settings_for_session;
use crate::widgets::explorer::{ExplorerEvent, ExplorerIntent};
use crate::widgets::quick_launch::{QuickLaunchEvent, QuickLaunchIntent};
use crate::widgets::settings::{SettingsEvent, SettingsIntent};
use crate::widgets::tabs::view::session_list::SESSION_LIST_SCROLL_ID;
use crate::widgets::tabs::{TabsEffect, TabsEvent};
use crate::widgets::terminal_workspace::types::TerminalKind;
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
        TabsEffect::SettingsTabOpened => Task::done(AppEvent::Settings(
            SettingsEvent::Intent(SettingsIntent::Reload),
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
        TabsEffect::ErrorTabOpened {
            tab_id,
            title,
            message,
        } => Task::done(AppEvent::QuickLaunch(QuickLaunchEvent::Intent(
            QuickLaunchIntent::OpenErrorTab {
                tab_id,
                title,
                message,
            },
        ))),
        TabsEffect::ScrollBarToEnd => snap_to_end(SESSION_LIST_SCROLL_ID),
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    use iced::{Point, Size};
    use otty_libterm::surface::{BlockMeta, BlockSnapshot, SnapshotOwned};
    use otty_ui_term::settings::{LocalSessionOptions, SessionKind, Settings};

    use super::handle;
    use crate::app::App;
    use crate::domain::exit_behavior::ExitBehavior;
    use crate::domain::quick_launch::WizardTabInit;
    use crate::events::AppEvent;
    use crate::testing::task_outputs::collect_outputs;
    use crate::widgets::explorer::{ExplorerEvent, ExplorerIntent};
    use crate::widgets::quick_launch::{QuickLaunchEvent, QuickLaunchIntent};
    use crate::widgets::tabs::{TabsEffect, TabsEvent, TabsIntent};
    use crate::widgets::terminal_workspace::types::TerminalKind;
    use crate::widgets::terminal_workspace::{
        TerminalWorkspaceCtx, TerminalWorkspaceEvent, TerminalWorkspaceIntent,
    };

    #[cfg(unix)]
    const TEST_SHELL_PATH: &str = "/bin/sh";
    #[cfg(target_os = "windows")]
    const TEST_SHELL_PATH: &str = "cmd.exe";

    fn shell_settings() -> Settings {
        let mut settings = Settings::default();
        settings.backend = settings.backend.clone().with_session(
            SessionKind::from_local_options(
                LocalSessionOptions::default().with_program(TEST_SHELL_PATH),
            ),
        );
        settings
    }

    fn workspace_ctx(active_tab_id: u64) -> TerminalWorkspaceCtx {
        TerminalWorkspaceCtx {
            active_tab_id: Some(active_tab_id),
            pane_grid_size: Size::new(800.0, 600.0),
            screen_size: Size::new(800.0, 600.0),
            sidebar_cursor: Point::ORIGIN,
            equalize_panes: true,
        }
    }

    /// Open a shell tab and make its terminal report `cwd`, the way shell
    /// integration does after a prompt.
    fn open_shell_tab_in(app: &mut App, cwd: &str) -> u64 {
        open_tab_in(app, cwd, TerminalKind::Shell)
    }

    /// Open a local terminal tab of `kind` and make its terminal report
    /// `cwd`.
    fn open_tab_in(app: &mut App, cwd: &str, kind: TerminalKind) -> u64 {
        let _ = app.widgets.tabs.reduce(TabsIntent::OpenTerminalTab {
            title: String::from("sh"),
        });
        let tab_id = app
            .widgets
            .tabs
            .active_tab_id()
            .expect("shell tab should open");
        let ctx = workspace_ctx(tab_id);

        let _ = app.widgets.terminal_workspace.reduce(
            TerminalWorkspaceIntent::OpenTab {
                tab_id,
                default_title: String::from("sh"),
                settings: Box::new(shell_settings()),
                kind,
                sync_explorer: false,
            },
            &ctx,
        );

        let terminal_id = app
            .widgets
            .terminal_workspace
            .tab(tab_id)
            .and_then(|tab| tab.focused_terminal_id())
            .expect("shell tab should have a terminal");
        let mut frame = SnapshotOwned::default();
        frame.blocks.push(BlockSnapshot {
            meta: BlockMeta {
                cwd: Some(String::from(cwd)),
                ..BlockMeta::default()
            },
            ..BlockSnapshot::default()
        });

        let _ = app.widgets.terminal_workspace.reduce(
            TerminalWorkspaceIntent::Widget(otty_ui_term::Event::ContentSync {
                id: terminal_id,
                frame: Arc::new(frame),
            }),
            &ctx,
        );

        tab_id
    }

    /// Reduce a tabs intent, then route the tabs effects it produced.
    fn route_intent(app: &mut App, intent: TabsIntent) -> Vec<AppEvent> {
        let effects = collect_outputs(handle(app, TabsEvent::Intent(intent)));

        effects
            .into_iter()
            .flat_map(|event| match event {
                AppEvent::Tabs(TabsEvent::Effect(effect)) => {
                    collect_outputs(handle(app, TabsEvent::Effect(effect)))
                },
                _ => Vec::new(),
            })
            .collect()
    }

    fn explorer_roots(events: &[AppEvent]) -> Vec<PathBuf> {
        events
            .iter()
            .filter_map(|event| match event {
                AppEvent::Explorer(ExplorerEvent::Intent(
                    ExplorerIntent::SyncRoot { cwd },
                )) => Some(cwd.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn given_two_shell_tabs_when_background_tab_activated_then_explorer_follows_its_folder()
     {
        let (mut app, _) = App::new();
        let first_id = open_shell_tab_in(&mut app, "/tmp/otty-first");
        let _second_id = open_shell_tab_in(&mut app, "/tmp/otty-second");

        let events = route_intent(
            &mut app,
            TabsIntent::ActivateTab { tab_id: first_id },
        );

        assert_eq!(
            explorer_roots(&events),
            vec![Path::new("/tmp/otty-first").to_path_buf()]
        );
        assert!(events.iter().any(|event| matches!(
            event,
            AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
                TerminalWorkspaceIntent::FocusActive
            ))
        )));
        assert!(events.iter().any(|event| matches!(
            event,
            AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
                TerminalWorkspaceIntent::SyncSelection { tab_id }
            )) if *tab_id == first_id
        )));
    }

    #[test]
    fn given_local_command_tab_reporting_a_folder_when_activated_then_explorer_follows_it()
     {
        let (mut app, _) = App::new();
        let launch_id = open_tab_in(
            &mut app,
            "/tmp/otty-launch",
            TerminalKind::Command {
                exit_behavior: ExitBehavior::Close,
            },
        );
        let _shell_id = open_shell_tab_in(&mut app, "/tmp/otty-shell");

        let events = route_intent(
            &mut app,
            TabsIntent::ActivateTab { tab_id: launch_id },
        );

        assert_eq!(
            explorer_roots(&events),
            vec![Path::new("/tmp/otty-launch").to_path_buf()]
        );
    }

    #[test]
    fn given_active_shell_tab_when_closed_then_explorer_follows_the_new_active_tab()
     {
        let (mut app, _) = App::new();
        let first_id = open_shell_tab_in(&mut app, "/tmp/otty-first");
        let second_id = open_shell_tab_in(&mut app, "/tmp/otty-second");

        let events =
            route_intent(&mut app, TabsIntent::CloseTab { tab_id: second_id });

        assert_eq!(app.widgets.tabs.active_tab_id(), Some(first_id));
        assert_eq!(
            explorer_roots(&events),
            vec![Path::new("/tmp/otty-first").to_path_buf()]
        );
        assert!(events.iter().any(|event| matches!(
            event,
            AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
                TerminalWorkspaceIntent::TabClosed { tab_id }
            )) if *tab_id == second_id
        )));
        assert!(events.iter().any(|event| matches!(
            event,
            AppEvent::QuickLaunch(QuickLaunchEvent::Intent(
                QuickLaunchIntent::TabClosed { tab_id }
            )) if *tab_id == second_id
        )));
    }

    #[test]
    fn given_settings_tab_when_activated_then_explorer_keeps_its_folder() {
        let (mut app, _) = App::new();
        let _shell_id = open_shell_tab_in(&mut app, "/tmp/otty-first");
        let _ = app.widgets.tabs.reduce(TabsIntent::OpenSettingsTab);
        let settings_id = app
            .widgets
            .tabs
            .active_tab_id()
            .expect("settings tab should open");

        let events = route_intent(
            &mut app,
            TabsIntent::ActivateTab {
                tab_id: settings_id,
            },
        );

        assert!(explorer_roots(&events).is_empty());
    }

    #[test]
    fn given_last_tab_when_closed_then_focus_is_not_requested() {
        let (mut app, _) = App::new();
        let tab_id = open_shell_tab_in(&mut app, "/tmp/otty-first");

        let events = route_intent(&mut app, TabsIntent::CloseTab { tab_id });

        assert!(explorer_roots(&events).is_empty());
        assert!(!events.iter().any(|event| matches!(
            event,
            AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
                TerminalWorkspaceIntent::FocusActive
            ))
        )));
    }

    #[test]
    fn given_wizard_opened_effect_when_create_init_exists_then_task_is_emitted()
    {
        let (mut app, _) = App::new();

        let task = handle(
            &mut app,
            TabsEvent::Effect(TabsEffect::WizardTabOpened {
                tab_id: 7,
                init: WizardTabInit::Create {
                    parent_path: vec![String::from("Demo")],
                },
            }),
        );
        assert_eq!(task.units(), 1);
    }

    #[test]
    fn given_error_tab_opened_effect_then_task_is_emitted() {
        let (mut app, _) = App::new();

        let task = handle(
            &mut app,
            TabsEvent::Effect(TabsEffect::ErrorTabOpened {
                tab_id: 11,
                title: String::from("Failed"),
                message: String::from("boom"),
            }),
        );
        assert_eq!(task.units(), 1);
    }
}
