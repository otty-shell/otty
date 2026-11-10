//! Delivers the intents of the command palette. What it lists and which
//! commands can run now are gathered here, from the other features, into
//! its `Ctx` before every intent.

use iced::Task;
use iced::widget::operation;
use otty_app_domain::commands::CommandCtx;
use otty_app_modes::Modes;
use otty_app_palette::{
    Palette, PaletteCtx, PaletteIntent, PaletteLaunch, PaletteOutcome,
    PaletteSession,
};
use otty_app_quick_launch::{QuickLaunchIntent, QuickLaunches};
use otty_app_tabs::{Tabs, TabsIntent};
use otty_app_terminal::TerminalIntent;
use otty_ui_kit::components::command_palette::input_id;

use super::{Delivery, command, deliver};
use crate::app::App;
use crate::guards::command_ctx;
use crate::message::AppEvent;

/// The palette gets what it lists, and reads what it reports.
impl Delivery for PaletteIntent {
    type Component = Palette;

    fn parts(app: &mut App) -> (&mut Palette, PaletteCtx<'_>) {
        let commands = command_ctx(app);
        // The ctx borrows the open tabs and the launches while the palette
        // is borrowed mutably, so it is built from the fields of `App` next
        // to the palette it is given to.
        let ctx =
            palette_ctx(&app.tabs, &app.quick_launch, &app.modes, commands);

        (&mut app.palette, ctx)
    }

    fn read(app: &mut App, outcome: PaletteOutcome) -> Task<AppEvent> {
        match outcome {
            // Moving the focus to the input also takes it from the terminal
            // panes, so typed keys reach the palette.
            PaletteOutcome::FocusInput => operation::focus(input_id()),
            // The tab comes to the front, as a press on its row in the
            // Sessions section does.
            PaletteOutcome::SessionPicked { tab_id } => {
                deliver(app, TabsIntent::ActivateTab { tab_id })
            },
            // Quick launch runs the launch, as the New session menu does.
            PaletteOutcome::LaunchPicked { path } => {
                deliver(app, QuickLaunchIntent::LaunchNode { path })
            },
            PaletteOutcome::CommandPicked(picked) => {
                command::deliver(app, picked)
            },
            // The terminal decides: in Terminal mode the active pane gets
            // its keys back; in Containers mode nothing takes them.
            PaletteOutcome::KeyboardReleased => {
                deliver(app, TerminalIntent::FocusActive)
            },
        }
    }
}

/// Gather what the palette lists: the open tabs, grouped by machine as in
/// the Sessions section; every launch in tree order; the pinned modes, for
/// the mode shortcut labels; and the commands that can run now
/// (`commands`). The palette lists and runs only those (see
/// `AppCommand::is_available`).
fn palette_ctx<'a>(
    tabs: &'a Tabs,
    launches: &'a QuickLaunches,
    modes: &Modes,
    commands: CommandCtx,
) -> PaletteCtx<'a> {
    let sessions = tabs
        .sessions()
        .into_iter()
        .map(|session| PaletteSession {
            tab_id: session.tab_id(),
            title: session.title(),
            machine: session.machine_name(),
            target: session.machine_target(),
            is_active: session.is_active(),
        })
        .collect();
    let launches = launches
        .launches()
        .into_iter()
        .map(|launch| PaletteLaunch {
            title: launch.title(),
            is_ssh: launch.is_ssh(),
            path: launch.into_path(),
        })
        .collect();
    let pinned_modes = modes.switcher_modes();

    PaletteCtx {
        sessions,
        launches,
        pinned_modes,
        commands,
    }
}

#[cfg(test)]
mod tests {
    use otty_app_domain::exit_behavior::ExitBehavior;
    use otty_app_domain::mode::Mode;
    use otty_app_domain::tab::TabMachine;
    use otty_app_i18n::{self as i18n, Key};
    use otty_app_modes::ModesIntent;
    use otty_app_slots::Component;
    use otty_ui_kit::components::command_palette::PaletteAction;
    use otty_ui_kit::components::target_dot::TargetKind;
    use otty_ui_term::settings::Settings;

    use super::*;
    use crate::frame::sidebar::SidebarIntent;
    use crate::testing::task_outputs::{focuses_widget, runs_widget_operation};
    use crate::testing::window::{
        app, app_with_launches, open_palette, open_shell_tab,
    };

    /// Type `query` in the open palette and pick the entry on top.
    fn pick(app: &mut App, query: &str) -> Task<AppEvent> {
        let _ = deliver(
            app,
            PaletteIntent::Action(PaletteAction::QueryChanged(String::from(
                query,
            ))),
        );

        deliver(app, PaletteIntent::Action(PaletteAction::RunActive))
    }

    /// The ctx the palette gets in `app` now.
    fn ctx_of(app: &App) -> PaletteCtx<'_> {
        palette_ctx(&app.tabs, &app.quick_launch, &app.modes, command_ctx(app))
    }

    /// Open a tab that runs `title` on `machine`, as a launch does.
    fn open_command_tab(app: &mut App, title: &str, machine: TabMachine) {
        let _ = deliver(
            app,
            TabsIntent::OpenCommandTab {
                title: String::from(title),
                settings: Box::new(Settings::default()),
                exit_behavior: ExitBehavior::Close,
                machine,
            },
        );
    }

    #[test]
    fn the_palette_lists_sessions_by_machine() {
        let mut app = app();
        let _ = deliver(
            &mut app,
            TabsIntent::HostNameRead(Some(String::from("fedora"))),
        );
        open_command_tab(&mut app, "zsh", TabMachine::ThisComputer);
        open_command_tab(
            &mut app,
            "api-prod",
            TabMachine::Ssh(String::from("prod-api")),
        );
        open_command_tab(&mut app, "nvim", TabMachine::ThisComputer);

        let ctx = ctx_of(&app);

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
                    tab_id: 2,
                    title: "nvim",
                    machine: "fedora",
                    target: TargetKind::Local,
                    is_active: true,
                },
                PaletteSession {
                    tab_id: 1,
                    title: "api-prod",
                    machine: "prod-api",
                    target: TargetKind::Ssh,
                    is_active: false,
                },
            ]
        );
        assert!(ctx.commands.has_active_tab);
    }

    /// Every launch of the tree, folders left out, in tree order, each with
    /// the path that runs it.
    #[test]
    fn the_palette_lists_the_launches_in_tree_order() {
        let saved = serde_json::from_value(serde_json::json!({
            "version": 1,
            "root": {"title": "Root", "expanded": true, "children": [
                {"kind": "Folder", "title": "servers", "expanded": false,
                 "children": [{"kind": "Command", "title": "laptop",
                   "spec": {"type": "Ssh", "ssh": {"host": "laptop",
                     "port": 22, "user": null, "identity_file": null,
                     "extra_args": []}}}]},
                {"kind": "Command", "title": "htop",
                 "spec": {"type": "Custom", "custom": {"program": "htop",
                   "args": [], "env": [], "working_directory": null}}}
            ]}
        }))
        .expect("the launches should parse");
        let app = app_with_launches(saved);

        let ctx = ctx_of(&app);

        assert_eq!(
            ctx.launches,
            vec![
                PaletteLaunch {
                    path: vec![String::from("servers"), String::from("laptop")],
                    title: "laptop",
                    is_ssh: true,
                },
                PaletteLaunch {
                    path: vec![String::from("htop")],
                    title: "htop",
                    is_ssh: false,
                },
            ]
        );
    }

    /// A picked session comes to the front, as a press on its row does.
    #[test]
    fn a_picked_session_becomes_the_active_tab() {
        let mut app = app();
        open_command_tab(&mut app, "htop", TabMachine::ThisComputer);
        open_command_tab(&mut app, "logs", TabMachine::ThisComputer);
        open_palette(&mut app);

        let _ = pick(&mut app, "htop");

        assert!(!app.palette.is_open());
        assert_eq!(app.tabs.active_tab_title(), Some("htop"));
    }

    #[test]
    fn the_palette_lists_terminal_and_the_pinned_modes_and_reads_the_mode() {
        let mut app = app();
        let in_terminal = ctx_of(&app);
        let (terminal_modes, terminal_commands) =
            (in_terminal.pinned_modes, in_terminal.commands);
        let _ = app.modes.reduce(ModesIntent::Unpin(Mode::Containers), ());
        let _ = app.modes.reduce(ModesIntent::Select(Mode::Containers), ());

        let in_containers = ctx_of(&app);

        assert_eq!(terminal_modes, [Mode::Terminal, Mode::Containers]);
        assert_eq!(
            terminal_commands,
            CommandCtx {
                is_terminal_mode: true,
                has_active_tab: false,
                has_focused_pane: false,
            }
        );
        assert_eq!(in_containers.pinned_modes, [Mode::Terminal]);
        assert!(!in_containers.commands.is_terminal_mode);
    }

    #[test]
    fn an_opened_palette_gives_its_input_the_keyboard() {
        let mut app = app();

        let task = deliver(&mut app, PaletteIntent::Open);

        assert!(app.palette.is_open());
        assert!(focuses_widget(task, &input_id()));
    }

    /// A picked command runs in the same update, through the same path as
    /// its shortcut.
    #[test]
    fn a_picked_command_runs_in_the_same_update() {
        let mut app = app();
        open_palette(&mut app);

        let _ = pick(&mut app, i18n::t(Key::WindowFocusMode));

        assert!(!app.palette.is_open());
        assert!(app.sidebar.vm().is_focus_mode);
    }

    /// The palette entry of the bell opens the notifications list, which
    /// takes the keyboard as any menu does.
    #[test]
    fn the_notifications_entry_opens_the_list() {
        let mut app = app();
        open_palette(&mut app);

        let _ = pick(&mut app, i18n::t(Key::NotificationsTitle));

        assert!(!app.palette.is_open());
        assert!(app.notifications.is_list_open());
    }

    /// The palette entry of the gear opens Settings in the same update.
    #[test]
    fn the_settings_entry_opens_settings() {
        let mut app = app();
        open_palette(&mut app);

        let _ = pick(&mut app, i18n::t(Key::SettingsTitle));

        assert!(!app.palette.is_open());
        assert!(app.settings.is_open());
    }

    /// Every mode can be switched to from the palette, pinned or not.
    #[test]
    fn a_picked_mode_switches_to_it() {
        let mut app = app();
        let _ = app.modes.reduce(ModesIntent::Unpin(Mode::Containers), ());
        open_palette(&mut app);

        let _ = pick(&mut app, i18n::t(Key::PaletteSwitchToContainersMode));

        assert_eq!(app.modes.active(), Mode::Containers);
    }

    /// New folder acts on the Launches panel, which shows only in Terminal
    /// mode: the palette does not list it in Containers mode.
    #[test]
    fn a_command_that_can_not_run_now_is_not_listed() {
        let mut app = app();
        let _ = app.modes.reduce(ModesIntent::Select(Mode::Containers), ());
        let _ = app.sidebar.reduce(SidebarIntent::ToggleLeft, ());
        open_palette(&mut app);

        let _ = pick(&mut app, i18n::t(Key::PaletteNewFolder));

        assert!(app.palette.is_open());
        assert!(!app.sidebar.vm().is_left_open);
    }

    /// Without a terminal tab nothing takes the keyboard after the palette
    /// closes.
    #[test]
    fn a_closed_palette_without_a_terminal_tab_moves_no_focus() {
        let mut app = app();
        open_palette(&mut app);

        let task = deliver(&mut app, PaletteIntent::Dismiss);

        assert!(!app.palette.is_open());
        assert_eq!(task.units(), 0);
    }

    /// In Terminal mode the active pane gets its keys back when the palette
    /// is dismissed.
    #[test]
    fn a_dismissed_palette_gives_the_pane_its_keys() {
        let mut app = app();
        let _tab = open_shell_tab(&mut app);
        open_palette(&mut app);

        let task = deliver(&mut app, PaletteIntent::Dismiss);

        assert!(!app.palette.is_open());
        assert!(runs_widget_operation(task));
    }

    /// With a terminal tab the palette offers both splits, and a picked
    /// split splits the focused pane in the same update.
    #[test]
    fn with_a_terminal_tab_the_palette_offers_both_splits() {
        for key in [Key::PaletteSplitRight, Key::PaletteSplitDown] {
            let mut app = app();
            let _tab = open_shell_tab(&mut app);
            open_palette(&mut app);

            let _ = pick(&mut app, i18n::t(key));

            assert!(!app.palette.is_open(), "{key:?}");
            assert_eq!(app.terminal.pane_count(), Some(2), "{key:?}");
        }
    }

    /// Outside Terminal mode the tab is hidden, so the palette does not
    /// offer its splits.
    #[test]
    fn outside_terminal_mode_the_palette_hides_the_splits_of_the_hidden_tab() {
        let mut app = app();
        let _tab = open_shell_tab(&mut app);
        let _ = app.modes.reduce(ModesIntent::Select(Mode::Containers), ());
        open_palette(&mut app);

        let _ = pick(&mut app, i18n::t(Key::PaletteSplitRight));

        assert!(app.palette.is_open());
        assert_eq!(app.terminal.pane_count(), Some(1));
    }
}
