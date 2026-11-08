//! Routes the command palette: builds what it lists from the other
//! widgets' view models, and turns a chosen entry into the event the owning
//! widget already handles for the same button.

use iced::Task;

use super::AppEvent;
use super::command::command_ctx;
use crate::app::App;
use crate::commands::{AppCommand, CommandCtx};
use crate::domain::mode::Mode;
use crate::widgets::palette::event::{
    PaletteEffect, PaletteEvent, PaletteIntent,
};
use crate::widgets::palette::types::PaletteTarget;
use crate::widgets::palette::{PaletteCtx, PaletteLaunch, PaletteSession};
use crate::widgets::quick_launch::LaunchViewModel;
use crate::widgets::quick_launch::event::{
    QuickLaunchEvent, QuickLaunchIntent,
};
use crate::widgets::tabs::TabsViewModel;
use crate::widgets::tabs::event::{TabsEvent, TabsIntent};
use crate::widgets::terminal_workspace::event::{
    TerminalWorkspaceEvent, TerminalWorkspaceIntent,
};

/// Route a command palette event: reduce its intents and run the entry
/// it chose.
pub(crate) fn route(app: &mut App, event: PaletteEvent) -> Task<AppEvent> {
    match event {
        PaletteEvent::Intent(intent) => handle_intent(app, intent),
        PaletteEvent::Effect(effect) => handle_effect(effect),
    }
}

/// Build what the palette lists from the tabs and launches view models:
/// sessions keep the machine grouping of the Sessions panel, launches
/// keep tree order. `pinned_modes` is Terminal and the pinned modes in
/// switcher order, for the mode shortcut labels. `commands` decides which
/// app commands are listed.
fn palette_ctx<'a>(
    tabs: &TabsViewModel<'a>,
    launches: Vec<LaunchViewModel<'a>>,
    pinned_modes: Vec<Mode>,
    commands: CommandCtx,
) -> PaletteCtx<'a> {
    let sessions = tabs
        .groups
        .iter()
        .flat_map(|machine| {
            machine.sessions.iter().map(|session| PaletteSession {
                tab_id: session.tab_id,
                title: session.title,
                machine: machine.name,
                target: machine.target,
                is_active: session.is_active,
            })
        })
        .collect();
    let launches = launches
        .into_iter()
        .map(|launch| PaletteLaunch {
            path: launch.path,
            title: launch.title,
            is_ssh: launch.is_ssh,
        })
        .collect();

    PaletteCtx {
        sessions,
        launches,
        pinned_modes,
        commands,
    }
}

fn handle_intent(app: &mut App, intent: PaletteIntent) -> Task<AppEvent> {
    let pinned_modes = app
        .widgets
        .modes
        .vm()
        .pinned_rows
        .iter()
        .map(|row| row.mode)
        .collect();
    let ctx = palette_ctx(
        &app.widgets.tabs.vm(app.host_name.as_deref()),
        app.widgets.quick_launch.vm().launches(),
        pinned_modes,
        command_ctx(app),
    );

    app.widgets
        .palette
        .reduce(intent, &ctx)
        .map(AppEvent::Palette)
}

fn handle_effect(effect: PaletteEffect) -> Task<AppEvent> {
    match effect {
        PaletteEffect::Run(target) => run_target(target),
        // The palette input had keyboard focus; give it back to the
        // terminal the user was in.
        PaletteEffect::Dismissed => focus_active_terminal(),
    }
}

/// Send the event the owning widget's own button sends for `target`.
fn run_target(target: PaletteTarget) -> Task<AppEvent> {
    match target {
        PaletteTarget::Session { tab_id } => Task::done(AppEvent::Tabs(
            TabsEvent::Intent(TabsIntent::ActivateTab { tab_id }),
        )),
        // The same single message the New session menu sends for a launch.
        // The launch is still starting, so the active terminal gets back
        // the keyboard focus the palette input took.
        PaletteTarget::Launch { path } => Task::batch(vec![
            Task::done(AppEvent::QuickLaunch(QuickLaunchEvent::Intent(
                QuickLaunchIntent::LaunchNode { path },
            ))),
            focus_active_terminal(),
        ]),
        PaletteTarget::Command(command) => run_command(command),
    }
}

/// Ask the app to run `command`. The commands that do not move keyboard
/// focus themselves (the panel toggles, and Terminal mode, whose terminal
/// comes back on screen) also give it back to the active terminal, which
/// lost it to the palette input.
fn run_command(command: AppCommand) -> Task<AppEvent> {
    let run = Task::done(AppEvent::Command(command));

    match command {
        AppCommand::ToggleLeftPanel
        | AppCommand::ToggleFilesPanel
        | AppCommand::ToggleFocusMode
        | AppCommand::SwitchMode(Mode::Terminal) => {
            Task::batch(vec![run, focus_active_terminal()])
        },
        _ => run,
    }
}

fn focus_active_terminal() -> Task<AppEvent> {
    Task::done(AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
        TerminalWorkspaceIntent::FocusActive,
    )))
}

#[cfg(test)]
mod tests {
    use otty_ui_kit::components::target_dot::TargetKind;

    use super::*;
    use crate::testing::shell_tab::open_shell_tab;
    use crate::testing::task_outputs::collect_outputs;
    use crate::widgets::modes::ModesWidget;
    use crate::widgets::modes::event::ModesIntent;
    use crate::widgets::tabs::{
        SessionListGroupViewModel, SessionListRowViewModel, TabsCtx,
    };

    /// Context the tests reduce tabs intents with.
    const SH: TabsCtx<'static> = TabsCtx { shell_name: "sh" };

    fn run(app: &mut App, target: PaletteTarget) -> Vec<AppEvent> {
        collect_outputs(route(
            app,
            PaletteEvent::Effect(PaletteEffect::Run(target)),
        ))
    }

    fn is_focus_active(event: &AppEvent) -> bool {
        matches!(
            event,
            AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
                TerminalWorkspaceIntent::FocusActive
            ))
        )
    }

    fn session_vm(
        tab_id: u64,
        title: &'static str,
        is_active: bool,
    ) -> SessionListRowViewModel<'static> {
        SessionListRowViewModel {
            tab_id,
            title,
            folder: None,
            state: None,
            is_active,
        }
    }

    #[test]
    fn the_palette_lists_sessions_by_machine_and_launches_in_tree_order() {
        let tabs = TabsViewModel {
            groups: vec![
                SessionListGroupViewModel {
                    name: "fedora",
                    target: TargetKind::Local,
                    sessions: vec![
                        session_vm(0, "zsh", false),
                        session_vm(3, "nvim", true),
                    ],
                },
                SessionListGroupViewModel {
                    name: "prod-api",
                    target: TargetKind::Ssh,
                    sessions: vec![session_vm(2, "api-prod", false)],
                },
            ],
            session_count: 3,
        };
        let launches = vec![
            LaunchViewModel {
                path: vec![String::from("htop")],
                title: "htop",
                is_ssh: false,
            },
            LaunchViewModel {
                path: vec![String::from("servers"), String::from("laptop")],
                title: "laptop",
                is_ssh: true,
            },
        ];

        let commands = CommandCtx {
            is_terminal_mode: false,
            has_active_tab: true,
            has_focused_pane: true,
        };

        let ctx = palette_ctx(
            &tabs,
            launches,
            vec![Mode::Terminal, Mode::Containers],
            commands,
        );

        assert_eq!(
            ctx.sessions,
            vec![
                PaletteSession {
                    tab_id: 0,
                    title: "zsh",
                    machine: "fedora",
                    target: TargetKind::Local,
                    is_active: false,
                },
                PaletteSession {
                    tab_id: 3,
                    title: "nvim",
                    machine: "fedora",
                    target: TargetKind::Local,
                    is_active: true,
                },
                PaletteSession {
                    tab_id: 2,
                    title: "api-prod",
                    machine: "prod-api",
                    target: TargetKind::Ssh,
                    is_active: false,
                },
            ]
        );
        assert_eq!(
            ctx.launches,
            vec![
                PaletteLaunch {
                    path: vec![String::from("htop")],
                    title: "htop",
                    is_ssh: false,
                },
                PaletteLaunch {
                    path: vec![String::from("servers"), String::from("laptop"),],
                    title: "laptop",
                    is_ssh: true,
                },
            ]
        );
        assert_eq!(ctx.pinned_modes, vec![Mode::Terminal, Mode::Containers]);
        assert_eq!(ctx.commands, commands);
    }

    #[test]
    fn opening_lists_the_pinned_modes_from_the_modes_widget() {
        let (mut app, _) = App::new();
        app.widgets.modes = ModesWidget::new();
        let _ = app
            .widgets
            .modes
            .reduce(ModesIntent::Unpin(Mode::Containers));

        let _ = route(&mut app, PaletteEvent::Intent(PaletteIntent::Open));

        let vm = app.widgets.palette.vm().expect("palette should be open");
        let containers = vm
            .list
            .matches()
            .find(|(item, _)| {
                *item.key()
                    == PaletteTarget::Command(AppCommand::SwitchMode(
                        Mode::Containers,
                    ))
            })
            .map(|(item, _)| item.clone())
            .expect("an unpinned mode is still listed");
        assert_eq!(containers.shortcut(), None);
    }

    #[test]
    fn opening_lists_the_open_tabs_from_the_tabs_widget() {
        let (mut app, _) = App::new();
        let _ = app.widgets.tabs.reduce(TabsIntent::OpenSettingsTab, &SH);
        let settings_tab = app
            .widgets
            .tabs
            .active_tab_id()
            .expect("settings tab should be active");

        let _ = route(&mut app, PaletteEvent::Intent(PaletteIntent::Open));

        let vm = app.widgets.palette.vm().expect("palette should be open");
        assert!(vm.list.matches().any(|(item, _)| *item.key()
            == PaletteTarget::Session {
                tab_id: settings_tab
            }));
    }

    #[test]
    fn a_session_switches_to_its_tab() {
        let (mut app, _) = App::new();

        let events = run(&mut app, PaletteTarget::Session { tab_id: 7 });

        assert!(matches!(
            events.as_slice(),
            [AppEvent::Tabs(TabsEvent::Intent(TabsIntent::ActivateTab {
                tab_id: 7
            }))]
        ));
    }

    #[test]
    fn a_launch_runs_with_one_intent_and_the_terminal_gets_focus() {
        let (mut app, _) = App::new();
        let path = vec![String::from("servers"), String::from("laptop")];

        let events =
            run(&mut app, PaletteTarget::Launch { path: path.clone() });

        let launches = events
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    AppEvent::QuickLaunch(QuickLaunchEvent::Intent(
                        QuickLaunchIntent::LaunchNode { path: launched }
                    )) if *launched == path
                )
            })
            .count();
        assert_eq!(launches, 1);
        assert!(events.iter().any(is_focus_active));
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn closing_without_running_gives_the_terminal_its_focus_back() {
        let (mut app, _) = App::new();

        let events = collect_outputs(route(
            &mut app,
            PaletteEvent::Effect(PaletteEffect::Dismissed),
        ));

        assert!(matches!(events.as_slice(), [event] if is_focus_active(event)));
    }

    #[test]
    fn with_a_terminal_tab_the_palette_offers_split_pane() {
        let (mut app, _) = App::new();
        let _ = open_shell_tab(&mut app);

        let _ = route(&mut app, PaletteEvent::Intent(PaletteIntent::Open));

        let vm = app.widgets.palette.vm().expect("palette should be open");
        assert!(vm.list.matches().any(|(item, _)| *item.key()
            == PaletteTarget::Command(AppCommand::SplitPane)));
    }

    #[test]
    fn outside_terminal_mode_the_palette_hides_the_commands_of_the_hidden_tab()
    {
        let (mut app, _) = App::new();
        let tab_id = open_shell_tab(&mut app);
        let _ = app
            .widgets
            .modes
            .reduce(ModesIntent::Select(Mode::Containers));

        let _ = route(&mut app, PaletteEvent::Intent(PaletteIntent::Open));

        let vm = app.widgets.palette.vm().expect("palette should be open");
        let targets: Vec<&PaletteTarget> =
            vm.list.matches().map(|(item, _)| item.key()).collect();
        for command in [
            AppCommand::SplitPane,
            AppCommand::CloseSession,
            AppCommand::NewFolder,
        ] {
            assert!(
                !targets.contains(&&PaletteTarget::Command(command)),
                "{command:?}"
            );
        }
        assert!(targets.contains(&&PaletteTarget::Session { tab_id }));
    }

    #[test]
    fn a_command_entry_asks_the_app_to_run_its_command() {
        for command in [
            AppCommand::NewTerminal,
            AppCommand::NewLaunch,
            AppCommand::NewFolder,
            AppCommand::OpenSettings,
            AppCommand::SplitPane,
            AppCommand::CloseSession,
            AppCommand::SwitchMode(Mode::Containers),
        ] {
            let (mut app, _) = App::new();

            let events = run(&mut app, PaletteTarget::Command(command));

            assert!(
                matches!(
                    events.as_slice(),
                    [AppEvent::Command(asked)] if *asked == command
                ),
                "{command:?}"
            );
        }
    }

    #[test]
    fn panel_toggle_entries_run_their_command_and_give_the_terminal_focus() {
        for command in [
            AppCommand::ToggleLeftPanel,
            AppCommand::ToggleFilesPanel,
            AppCommand::ToggleFocusMode,
        ] {
            let (mut app, _) = App::new();

            let events = run(&mut app, PaletteTarget::Command(command));

            assert!(
                events.iter().any(|event| matches!(
                    event,
                    AppEvent::Command(asked) if *asked == command
                )),
                "{command:?}"
            );
            assert!(events.iter().any(is_focus_active), "{command:?}");
            assert_eq!(events.len(), 2, "{command:?}");
        }
    }

    #[test]
    fn the_terminal_mode_entry_switches_and_gives_the_terminal_its_focus_back()
    {
        let (mut app, _) = App::new();

        let events = run(
            &mut app,
            PaletteTarget::Command(AppCommand::SwitchMode(Mode::Terminal)),
        );

        assert!(events.iter().any(|event| matches!(
            event,
            AppEvent::Command(AppCommand::SwitchMode(Mode::Terminal))
        )));
        assert!(events.iter().any(is_focus_active));
        assert_eq!(events.len(), 2);
    }
}
