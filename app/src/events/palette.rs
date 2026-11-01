//! Routes the command palette: builds what it lists from the other
//! widgets' view models, and turns a chosen entry into the event the owning
//! widget already handles for the same button.

use iced::Task;
use iced::widget::pane_grid;

use super::AppEvent;
use crate::app::App;
use crate::widgets::palette::types::PaletteTarget;
use crate::widgets::palette::{
    PaletteCtx, PaletteEffect, PaletteEvent, PaletteIntent, PaletteLaunch,
    PaletteSession,
};
use crate::widgets::quick_launch::model::LaunchViewModel;
use crate::widgets::quick_launch::{QuickLaunchEvent, QuickLaunchIntent};
use crate::widgets::sidebar::{SidebarEvent, SidebarIntent};
use crate::widgets::tabs::model::TabsViewModel;
use crate::widgets::tabs::{TabsEvent, TabsIntent};
use crate::widgets::terminal_workspace::{
    TerminalWorkspaceEvent, TerminalWorkspaceIntent,
};

/// Route a command palette event: reduce its intents and run the entry
/// it chose.
pub(crate) fn handle(app: &mut App, event: PaletteEvent) -> Task<AppEvent> {
    match event {
        PaletteEvent::Intent(intent) => handle_intent(app, intent),
        PaletteEvent::Effect(effect) => handle_effect(app, effect),
    }
}

/// Build what the palette lists from the tabs and launches view models:
/// sessions keep the machine grouping of the Sessions panel, launches
/// keep tree order.
pub(crate) fn palette_ctx<'a>(
    tabs: &TabsViewModel<'a>,
    launches: Vec<LaunchViewModel<'a>>,
    can_split_pane: bool,
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
        can_split_pane,
    }
}

fn handle_intent(app: &mut App, intent: PaletteIntent) -> Task<AppEvent> {
    let ctx = palette_ctx(
        &app.widgets.tabs.vm(app.host_name.as_deref()),
        app.widgets.quick_launch.vm().launches(),
        focused_pane(app).is_some(),
    );

    app.widgets
        .palette
        .reduce(intent, &ctx)
        .map(AppEvent::Palette)
}

fn handle_effect(app: &App, effect: PaletteEffect) -> Task<AppEvent> {
    match effect {
        PaletteEffect::Run(target) => run_target(app, target),
        // The palette input had keyboard focus; give it back to the
        // terminal the user was in.
        PaletteEffect::Dismissed => focus_active_terminal(),
    }
}

/// Send the event the owning widget's own button sends for `target`.
///
/// Entries that do not move keyboard focus themselves (a launch still
/// starting, the panel toggles) also give it back to the active terminal,
/// which lost it to the palette input.
fn run_target(app: &App, target: PaletteTarget) -> Task<AppEvent> {
    match target {
        PaletteTarget::Session { tab_id } => {
            tabs_intent(TabsIntent::ActivateTab { tab_id })
        },
        // The same single message the New session menu sends for a launch.
        PaletteTarget::Launch { path } => Task::batch(vec![
            quick_launch_intent(QuickLaunchIntent::LaunchNode { path }),
            focus_active_terminal(),
        ]),
        PaletteTarget::NewTerminal => tabs_intent(TabsIntent::OpenTerminalTab),
        PaletteTarget::NewLaunch => {
            quick_launch_intent(QuickLaunchIntent::HeaderCreateCommand)
        },
        // The folder name field lives in the Launches section, so show the
        // section first; the field then gets keyboard focus on screen.
        PaletteTarget::NewFolder => Task::done(AppEvent::Sidebar(
            SidebarEvent::Intent(SidebarIntent::RevealLaunches),
        ))
        .chain(quick_launch_intent(QuickLaunchIntent::HeaderCreateFolder)),
        PaletteTarget::Settings => tabs_intent(TabsIntent::OpenSettingsTab),
        PaletteTarget::SplitPane => match focused_pane(app) {
            Some((tab_id, pane)) => Task::done(AppEvent::TerminalWorkspace(
                TerminalWorkspaceEvent::Intent(
                    TerminalWorkspaceIntent::SplitPane {
                        tab_id,
                        pane,
                        axis: pane_grid::Axis::Vertical,
                    },
                ),
            )),
            None => Task::none(),
        },
        PaletteTarget::CloseSession => match app.widgets.tabs.active_tab_id() {
            Some(tab_id) => tabs_intent(TabsIntent::CloseTab { tab_id }),
            None => Task::none(),
        },
        PaletteTarget::ToggleLeftPanel => {
            toggle_panel(SidebarIntent::ToggleLeft)
        },
        PaletteTarget::ToggleFilesPanel => {
            toggle_panel(SidebarIntent::ToggleRight)
        },
        PaletteTarget::ToggleFocusMode => {
            toggle_panel(SidebarIntent::ToggleFocusMode)
        },
    }
}

/// The active terminal tab and its focused pane, the pane the pane menu's
/// "Split right" would split.
fn focused_pane(app: &App) -> Option<(u64, pane_grid::Pane)> {
    let tab_id = app.widgets.tabs.active_tab_id();

    app.widgets.terminal_workspace.focused_pane(tab_id)
}

fn toggle_panel(intent: SidebarIntent) -> Task<AppEvent> {
    Task::batch(vec![
        Task::done(AppEvent::Sidebar(SidebarEvent::Intent(intent))),
        focus_active_terminal(),
    ])
}

fn focus_active_terminal() -> Task<AppEvent> {
    Task::done(AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
        TerminalWorkspaceIntent::FocusActive,
    )))
}

fn tabs_intent(intent: TabsIntent) -> Task<AppEvent> {
    Task::done(AppEvent::Tabs(TabsEvent::Intent(intent)))
}

fn quick_launch_intent(intent: QuickLaunchIntent) -> Task<AppEvent> {
    Task::done(AppEvent::QuickLaunch(QuickLaunchEvent::Intent(intent)))
}

#[cfg(test)]
mod tests {
    use iced::{Point, Size};
    use otty_ui_kit::components::target_dot::TargetKind;
    use otty_ui_term::settings::{LocalSessionOptions, SessionKind, Settings};

    use super::*;
    use crate::testing::task_outputs::collect_outputs;
    use crate::widgets::tabs::TabsCtx;
    use crate::widgets::tabs::model::{
        SessionGroupViewModel, SessionViewModel,
    };
    use crate::widgets::terminal_workspace::TerminalWorkspaceCtx;
    use crate::widgets::terminal_workspace::types::TerminalKind;

    /// Context the tests reduce tabs intents with.
    const SH: TabsCtx<'static> = TabsCtx { shell_name: "sh" };

    #[cfg(unix)]
    const TEST_SHELL_PATH: &str = "/bin/sh";
    #[cfg(target_os = "windows")]
    const TEST_SHELL_PATH: &str = "cmd.exe";

    fn run(app: &mut App, target: PaletteTarget) -> Vec<AppEvent> {
        collect_outputs(handle(
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

    /// Open a shell tab with one real terminal and return its id.
    fn open_shell_tab(app: &mut App) -> u64 {
        let _ = app.widgets.tabs.reduce(TabsIntent::OpenTerminalTab, &SH);
        let tab_id = app
            .widgets
            .tabs
            .active_tab_id()
            .expect("shell tab should open");
        let mut settings = Settings::default();
        settings.backend = settings.backend.clone().with_session(
            SessionKind::from_local_options(
                LocalSessionOptions::default().with_program(TEST_SHELL_PATH),
            ),
        );
        let ctx = TerminalWorkspaceCtx {
            active_tab_id: Some(tab_id),
            pane_grid_size: Size::new(800.0, 600.0),
            screen_size: Size::new(800.0, 600.0),
            sidebar_cursor: Point::ORIGIN,
            equalize_panes: true,
        };

        let _ = app.widgets.terminal_workspace.reduce(
            TerminalWorkspaceIntent::OpenTab {
                tab_id,
                default_title: String::from("sh"),
                settings: Box::new(settings),
                kind: TerminalKind::Shell,
                sync_explorer: false,
            },
            &ctx,
        );

        tab_id
    }

    fn session_vm(
        tab_id: u64,
        title: &'static str,
        is_active: bool,
    ) -> SessionViewModel<'static> {
        SessionViewModel {
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
                SessionGroupViewModel {
                    name: "fedora",
                    target: TargetKind::Local,
                    sessions: vec![
                        session_vm(0, "zsh", false),
                        session_vm(3, "nvim", true),
                    ],
                },
                SessionGroupViewModel {
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

        let ctx = palette_ctx(&tabs, launches, true);

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
        assert!(ctx.can_split_pane);
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

        let _ = handle(&mut app, PaletteEvent::Intent(PaletteIntent::Open));

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
    fn new_terminal_opens_a_shell_tab() {
        let (mut app, _) = App::new();

        let events = run(&mut app, PaletteTarget::NewTerminal);

        assert!(matches!(
            events.as_slice(),
            [AppEvent::Tabs(TabsEvent::Intent(
                TabsIntent::OpenTerminalTab
            ))]
        ));
    }

    #[test]
    fn new_launch_runs_the_launches_header_action() {
        let (mut app, _) = App::new();

        let events = run(&mut app, PaletteTarget::NewLaunch);

        assert!(matches!(
            events.as_slice(),
            [AppEvent::QuickLaunch(QuickLaunchEvent::Intent(
                QuickLaunchIntent::HeaderCreateCommand
            ))]
        ));
    }

    #[test]
    fn new_folder_reveals_the_launches_section_before_starting_the_folder() {
        let (mut app, _) = App::new();

        let events = run(&mut app, PaletteTarget::NewFolder);

        // The name field only takes keyboard focus once it is on screen.
        assert!(matches!(
            events.as_slice(),
            [
                AppEvent::Sidebar(SidebarEvent::Intent(
                    SidebarIntent::RevealLaunches
                )),
                AppEvent::QuickLaunch(QuickLaunchEvent::Intent(
                    QuickLaunchIntent::HeaderCreateFolder
                )),
            ]
        ));
    }

    #[test]
    fn settings_opens_the_settings_tab() {
        let (mut app, _) = App::new();

        let events = run(&mut app, PaletteTarget::Settings);

        assert!(matches!(
            events.as_slice(),
            [AppEvent::Tabs(TabsEvent::Intent(
                TabsIntent::OpenSettingsTab
            ))]
        ));
    }

    #[test]
    fn split_pane_splits_the_focused_pane_to_the_right() {
        let (mut app, _) = App::new();
        let tab_id = open_shell_tab(&mut app);
        let focused = app
            .widgets
            .terminal_workspace
            .tab(tab_id)
            .and_then(|tab| tab.focus())
            .expect("shell tab should have a focused pane");

        let events = run(&mut app, PaletteTarget::SplitPane);

        assert!(matches!(
            events.as_slice(),
            [AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
                TerminalWorkspaceIntent::SplitPane {
                    tab_id: split_tab,
                    pane,
                    axis: pane_grid::Axis::Vertical,
                }
            ))] if *split_tab == tab_id && *pane == focused
        ));
    }

    #[test]
    fn split_pane_without_a_terminal_tab_does_nothing() {
        let (mut app, _) = App::new();
        let _ = app.widgets.tabs.reduce(TabsIntent::OpenSettingsTab, &SH);

        let events = run(&mut app, PaletteTarget::SplitPane);

        assert!(events.is_empty());
    }

    #[test]
    fn close_session_closes_the_active_tab() {
        let (mut app, _) = App::new();
        let _ = app.widgets.tabs.reduce(TabsIntent::OpenSettingsTab, &SH);
        let active = app
            .widgets
            .tabs
            .active_tab_id()
            .expect("settings tab should be active");

        let events = run(&mut app, PaletteTarget::CloseSession);

        assert!(matches!(
            events.as_slice(),
            [AppEvent::Tabs(TabsEvent::Intent(TabsIntent::CloseTab { tab_id }))]
                if *tab_id == active
        ));
    }

    #[test]
    fn close_session_without_tabs_does_nothing() {
        let (mut app, _) = App::new();

        let events = run(&mut app, PaletteTarget::CloseSession);

        assert!(events.is_empty());
    }

    #[test]
    fn panel_toggles_run_the_sidebar_toggles_and_give_the_terminal_focus() {
        let cases = [
            (PaletteTarget::ToggleLeftPanel, SidebarIntent::ToggleLeft),
            (PaletteTarget::ToggleFilesPanel, SidebarIntent::ToggleRight),
            (
                PaletteTarget::ToggleFocusMode,
                SidebarIntent::ToggleFocusMode,
            ),
        ];

        for (target, expected) in cases {
            let (mut app, _) = App::new();

            let events = run(&mut app, target.clone());

            // `SidebarIntent` has no `PartialEq`; its debug text names it.
            let sidebar_intents: Vec<String> = events
                .iter()
                .filter_map(|event| match event {
                    AppEvent::Sidebar(SidebarEvent::Intent(intent)) => {
                        Some(format!("{intent:?}"))
                    },
                    _ => None,
                })
                .collect();
            assert_eq!(sidebar_intents, vec![format!("{expected:?}")]);
            assert!(events.iter().any(is_focus_active), "{target:?}");
            assert_eq!(events.len(), 2, "{target:?}");
        }
    }

    #[test]
    fn closing_without_running_gives_the_terminal_its_focus_back() {
        let (mut app, _) = App::new();

        let events = collect_outputs(handle(
            &mut app,
            PaletteEvent::Effect(PaletteEffect::Dismissed),
        ));

        assert!(matches!(events.as_slice(), [event] if is_focus_active(event)));
    }

    #[test]
    fn with_a_terminal_tab_the_palette_offers_split_pane() {
        let (mut app, _) = App::new();
        let _ = open_shell_tab(&mut app);

        let _ = handle(&mut app, PaletteEvent::Intent(PaletteIntent::Open));

        let vm = app.widgets.palette.vm().expect("palette should be open");
        assert!(
            vm.list
                .matches()
                .any(|(item, _)| *item.key() == PaletteTarget::SplitPane)
        );
    }
}
