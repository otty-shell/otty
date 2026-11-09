//! Delivers the intents of the command palette. What it lists is built
//! here from the tabs view model, the saved launches, the pinned modes and
//! the commands that can run now.

use iced::Task;

use super::AppEvent;
use super::command::command_ctx;
use crate::app::App;
use crate::commands::CommandCtx;
use crate::domain::mode::Mode;
use crate::widgets::palette::event::PaletteIntent;
use crate::widgets::palette::{PaletteCtx, PaletteLaunch, PaletteSession};
use crate::widgets::quick_launch::types::LaunchSummary;
use crate::widgets::tabs::TabsViewModel;

/// Deliver a palette intent to the widget's reducer, with what the palette
/// lists.
pub(crate) fn deliver(app: &mut App, intent: PaletteIntent) -> Task<AppEvent> {
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
        app.widgets.quick_launch.launches(),
        pinned_modes,
        command_ctx(app),
    );

    app.widgets
        .palette
        .reduce(intent, &ctx)
        .map(AppEvent::Palette)
}

/// Build what the palette lists from the tabs view model and the launches:
/// sessions keep the machine grouping of the Sessions panel, launches
/// keep tree order. `pinned_modes` is Terminal and the pinned modes in
/// switcher order, for the mode shortcut labels. `commands` decides which
/// app commands are listed.
fn palette_ctx<'a>(
    tabs: &TabsViewModel<'a>,
    launches: Vec<LaunchSummary<'a>>,
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

#[cfg(test)]
mod tests {
    use otty_ui_kit::components::target_dot::TargetKind;

    use super::*;
    use crate::commands::AppCommand;
    use crate::testing::shell_tab::open_shell_tab;
    use crate::testing::window::open_launch_editor;
    use crate::widgets::modes::ModesWidget;
    use crate::widgets::modes::event::ModesIntent;
    use crate::widgets::palette::types::PaletteTarget;
    use crate::widgets::tabs::{
        SessionListGroupViewModel, SessionListRowViewModel,
    };

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
            LaunchSummary {
                path: vec![String::from("htop")],
                title: "htop",
                is_ssh: false,
            },
            LaunchSummary {
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

        let _ = deliver(&mut app, PaletteIntent::Open);

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
        let editor_tab = open_launch_editor(&mut app);

        let _ = deliver(&mut app, PaletteIntent::Open);

        let vm = app.widgets.palette.vm().expect("palette should be open");
        assert!(vm.list.matches().any(|(item, _)| *item.key()
            == PaletteTarget::Session { tab_id: editor_tab }));
    }

    #[test]
    fn with_a_terminal_tab_the_palette_offers_both_splits() {
        let (mut app, _) = App::new();
        let _ = open_shell_tab(&mut app);

        let _ = deliver(&mut app, PaletteIntent::Open);

        let vm = app.widgets.palette.vm().expect("palette should be open");
        for command in [AppCommand::SplitRight, AppCommand::SplitDown] {
            assert!(
                vm.list
                    .matches()
                    .any(|(item, _)| *item.key()
                        == PaletteTarget::Command(command)),
                "{command:?}"
            );
        }
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

        let _ = deliver(&mut app, PaletteIntent::Open);

        let vm = app.widgets.palette.vm().expect("palette should be open");
        let targets: Vec<&PaletteTarget> =
            vm.list.matches().map(|(item, _)| item.key()).collect();
        for command in [
            AppCommand::SplitRight,
            AppCommand::SplitDown,
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
}
