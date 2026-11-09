//! Runs app commands, whichever way the user asked for them: a shortcut, a
//! command palette entry or a header button. This is the one place that
//! knows which widget intents a command stands for.

use iced::Task;
use iced::widget::pane_grid;

use super::AppEvent;
use crate::app::App;
use crate::commands::{AppCommand, CommandCtx};
use crate::domain::mode::Mode;
use crate::widgets::modes::event::{ModesEvent, ModesIntent};
use crate::widgets::notifications::event::{
    NotificationsEvent, NotificationsIntent,
};
use crate::widgets::palette::event::{PaletteEvent, PaletteIntent};
use crate::widgets::quick_launch::event::{
    QuickLaunchEvent, QuickLaunchIntent,
};
use crate::widgets::settings::event::{SettingsEvent, SettingsIntent};
use crate::widgets::sidebar::event::{SidebarEvent, SidebarIntent};
use crate::widgets::tabs::event::{TabsEvent, TabsIntent};
use crate::widgets::terminal_workspace::event::{
    TerminalWorkspaceEvent, TerminalWorkspaceIntent,
};

/// Run `command` if it can run now; a command that can not run (for
/// example Close session in Containers mode) does nothing.
pub(crate) fn route(app: &App, command: AppCommand) -> Task<AppEvent> {
    if !command.is_available(&command_ctx(app)) {
        return Task::none();
    }

    // Every command is listed, so a new one can not be forgotten here.
    match command {
        AppCommand::OpenPalette => Task::done(AppEvent::Palette(
            PaletteEvent::Intent(PaletteIntent::Open),
        )),
        AppCommand::ToggleLeftPanel => {
            sidebar_intent(SidebarIntent::ToggleLeft)
        },
        AppCommand::ToggleFilesPanel => {
            sidebar_intent(SidebarIntent::ToggleRight)
        },
        AppCommand::ToggleFocusMode => {
            sidebar_intent(SidebarIntent::ToggleFocusMode)
        },
        AppCommand::OpenSettings => Task::done(AppEvent::Settings(
            SettingsEvent::Intent(SettingsIntent::Open),
        )),
        AppCommand::OpenNotifications => Task::done(AppEvent::Notifications(
            NotificationsEvent::Intent(NotificationsIntent::OpenList),
        )),
        AppCommand::NewTerminal => tabs_intent(TabsIntent::OpenTerminalTab),
        AppCommand::CloseSession => tabs_intent(TabsIntent::CloseActive),
        AppCommand::SplitRight => split_focused_pane(pane_grid::Axis::Vertical),
        AppCommand::SplitDown => {
            split_focused_pane(pane_grid::Axis::Horizontal)
        },
        AppCommand::SelectModeAt { position } => {
            modes_intent(ModesIntent::SelectAt { position })
        },
        AppCommand::SwitchMode(mode) => modes_intent(ModesIntent::Select(mode)),
        AppCommand::ToggleModesMenu => modes_intent(ModesIntent::ToggleMenu),
        AppCommand::NewLaunch => {
            quick_launch_intent(QuickLaunchIntent::HeaderCreateCommand)
        },
        AppCommand::NewFolder => new_folder(),
    }
}

/// Gather what decides whether a command can run: the mode on screen, the
/// active tab and its focused pane.
pub(super) fn command_ctx(app: &App) -> CommandCtx {
    let active_tab_id = app.widgets.tabs.active_tab_id();

    CommandCtx {
        is_terminal_mode: app.widgets.modes.active() == Mode::Terminal,
        has_active_tab: active_tab_id.is_some(),
        has_focused_pane: app
            .widgets
            .terminal_workspace
            .focused_pane(active_tab_id)
            .is_some(),
    }
}

/// Start a new launch folder. Its name field lives in the Launches section,
/// so show the section first; the field then gets keyboard focus on screen.
fn new_folder() -> Task<AppEvent> {
    sidebar_intent(SidebarIntent::RevealLaunches)
        .chain(quick_launch_intent(QuickLaunchIntent::HeaderCreateFolder))
}

/// Split the focused pane of the active tab along `axis`. The terminal
/// workspace picks the tab and the pane.
fn split_focused_pane(axis: pane_grid::Axis) -> Task<AppEvent> {
    Task::done(AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
        TerminalWorkspaceIntent::SplitFocusedPane { axis },
    )))
}

fn sidebar_intent(intent: SidebarIntent) -> Task<AppEvent> {
    Task::done(AppEvent::Sidebar(SidebarEvent::Intent(intent)))
}

fn tabs_intent(intent: TabsIntent) -> Task<AppEvent> {
    Task::done(AppEvent::Tabs(TabsEvent::Intent(intent)))
}

fn modes_intent(intent: ModesIntent) -> Task<AppEvent> {
    Task::done(AppEvent::Modes(ModesEvent::Intent(intent)))
}

fn quick_launch_intent(intent: QuickLaunchIntent) -> Task<AppEvent> {
    Task::done(AppEvent::QuickLaunch(QuickLaunchEvent::Intent(intent)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::shell_tab::open_shell_tab;
    use crate::testing::task_outputs::collect_outputs;
    use crate::testing::window::open_launch_editor;
    use crate::widgets::modes::ModesWidget;

    fn run(app: &App, command: AppCommand) -> Vec<AppEvent> {
        collect_outputs(route(app, command))
    }

    /// App on first start: Terminal active, Containers pinned, without
    /// reading `modes.json`.
    fn first_start_app() -> App {
        let (mut app, _) = App::new();
        app.widgets.modes = ModesWidget::new();
        app
    }

    fn switch_to_containers(app: &mut App) {
        let _ = app
            .widgets
            .modes
            .reduce(ModesIntent::Select(Mode::Containers));
    }

    #[test]
    fn open_palette_opens_the_palette() {
        let app = first_start_app();

        let events = run(&app, AppCommand::OpenPalette);

        assert!(matches!(
            events.as_slice(),
            [AppEvent::Palette(PaletteEvent::Intent(PaletteIntent::Open))]
        ));
    }

    #[test]
    fn panel_commands_toggle_their_panel() {
        let app = first_start_app();

        let left = run(&app, AppCommand::ToggleLeftPanel);
        let files = run(&app, AppCommand::ToggleFilesPanel);
        let focus = run(&app, AppCommand::ToggleFocusMode);

        assert!(matches!(
            left.as_slice(),
            [AppEvent::Sidebar(SidebarEvent::Intent(
                SidebarIntent::ToggleLeft
            ))]
        ));
        assert!(matches!(
            files.as_slice(),
            [AppEvent::Sidebar(SidebarEvent::Intent(
                SidebarIntent::ToggleRight
            ))]
        ));
        assert!(matches!(
            focus.as_slice(),
            [AppEvent::Sidebar(SidebarEvent::Intent(
                SidebarIntent::ToggleFocusMode
            ))]
        ));
    }

    /// Settings opens as a popup in either mode, with no mode switch.
    #[test]
    fn open_settings_opens_the_settings_popup_in_either_mode() {
        let mut app = first_start_app();
        let in_terminal = run(&app, AppCommand::OpenSettings);
        switch_to_containers(&mut app);

        let in_containers = run(&app, AppCommand::OpenSettings);

        for events in [in_terminal, in_containers] {
            assert!(matches!(
                events.as_slice(),
                [AppEvent::Settings(SettingsEvent::Intent(
                    SettingsIntent::Open
                ))]
            ));
        }
    }

    /// The list opens over every mode, like Settings.
    #[test]
    fn open_notifications_opens_the_list_in_either_mode() {
        let mut app = first_start_app();
        let in_terminal = run(&app, AppCommand::OpenNotifications);
        switch_to_containers(&mut app);

        let in_containers = run(&app, AppCommand::OpenNotifications);

        for events in [in_terminal, in_containers] {
            assert!(matches!(
                events.as_slice(),
                [AppEvent::Notifications(NotificationsEvent::Intent(
                    NotificationsIntent::OpenList
                ))]
            ));
        }
    }

    #[test]
    fn new_terminal_opens_a_shell_tab() {
        let app = first_start_app();

        let events = run(&app, AppCommand::NewTerminal);

        assert!(matches!(
            events.as_slice(),
            [AppEvent::Tabs(TabsEvent::Intent(
                TabsIntent::OpenTerminalTab
            ))]
        ));
    }

    #[test]
    fn close_session_closes_the_active_tab() {
        let mut app = first_start_app();
        let _ = open_launch_editor(&mut app);

        let events = run(&app, AppCommand::CloseSession);

        assert!(matches!(
            events.as_slice(),
            [AppEvent::Tabs(TabsEvent::Intent(TabsIntent::CloseActive))]
        ));
    }

    #[test]
    fn close_session_without_tabs_does_nothing() {
        let app = first_start_app();

        let events = run(&app, AppCommand::CloseSession);

        assert!(events.is_empty());
    }

    #[test]
    fn split_right_splits_the_focused_pane_to_the_right() {
        let mut app = first_start_app();
        let _ = open_shell_tab(&mut app);

        let events = run(&app, AppCommand::SplitRight);

        assert!(matches!(
            events.as_slice(),
            [AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
                TerminalWorkspaceIntent::SplitFocusedPane {
                    axis: pane_grid::Axis::Vertical,
                }
            ))]
        ));
    }

    #[test]
    fn split_right_without_a_terminal_tab_does_nothing() {
        let mut app = first_start_app();
        let _ = open_launch_editor(&mut app);

        let events = run(&app, AppCommand::SplitRight);

        assert!(events.is_empty());
    }

    #[test]
    fn split_down_splits_the_focused_pane_down() {
        let mut app = first_start_app();
        let _ = open_shell_tab(&mut app);

        let events = run(&app, AppCommand::SplitDown);

        assert!(matches!(
            events.as_slice(),
            [AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
                TerminalWorkspaceIntent::SplitFocusedPane {
                    axis: pane_grid::Axis::Horizontal,
                }
            ))]
        ));
    }

    #[test]
    fn split_down_without_a_terminal_tab_does_nothing() {
        let mut app = first_start_app();
        let _ = open_launch_editor(&mut app);

        let events = run(&app, AppCommand::SplitDown);

        assert!(events.is_empty());
    }

    #[test]
    fn select_mode_at_asks_the_modes_widget_for_that_position() {
        let app = first_start_app();

        let events = run(&app, AppCommand::SelectModeAt { position: 2 });

        assert!(matches!(
            events.as_slice(),
            [AppEvent::Modes(ModesEvent::Intent(ModesIntent::SelectAt {
                position: 2
            }))]
        ));
    }

    #[test]
    fn switch_mode_selects_that_mode() {
        let app = first_start_app();

        let events = run(&app, AppCommand::SwitchMode(Mode::Containers));

        assert!(matches!(
            events.as_slice(),
            [AppEvent::Modes(ModesEvent::Intent(ModesIntent::Select(
                Mode::Containers
            )))]
        ));
    }

    #[test]
    fn toggle_modes_menu_toggles_the_menu() {
        let app = first_start_app();

        let events = run(&app, AppCommand::ToggleModesMenu);

        assert!(matches!(
            events.as_slice(),
            [AppEvent::Modes(ModesEvent::Intent(ModesIntent::ToggleMenu))]
        ));
    }

    #[test]
    fn new_launch_runs_the_launches_header_action() {
        let app = first_start_app();

        let events = run(&app, AppCommand::NewLaunch);

        assert!(matches!(
            events.as_slice(),
            [AppEvent::QuickLaunch(QuickLaunchEvent::Intent(
                QuickLaunchIntent::HeaderCreateCommand
            ))]
        ));
    }

    #[test]
    fn new_folder_reveals_the_launches_section_before_starting_the_folder() {
        let app = first_start_app();

        let events = run(&app, AppCommand::NewFolder);

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
    fn outside_terminal_mode_the_hidden_tab_and_launches_are_left_alone() {
        // Also what a palette entry chosen from a list opened before a mode
        // switch does.
        let mut app = first_start_app();
        let _ = open_shell_tab(&mut app);
        switch_to_containers(&mut app);

        for command in [
            AppCommand::CloseSession,
            AppCommand::SplitRight,
            AppCommand::SplitDown,
            AppCommand::NewFolder,
        ] {
            let events = run(&app, command);

            assert!(events.is_empty(), "{command:?}");
        }
    }

    #[test]
    fn the_command_context_reads_the_mode_the_active_tab_and_its_pane() {
        let mut app = first_start_app();
        let without_tabs = command_ctx(&app);
        let _ = open_launch_editor(&mut app);
        let with_launch_editor = command_ctx(&app);
        let _ = open_shell_tab(&mut app);
        let with_shell = command_ctx(&app);
        switch_to_containers(&mut app);
        let in_containers = command_ctx(&app);

        assert_eq!(
            without_tabs,
            CommandCtx {
                is_terminal_mode: true,
                has_active_tab: false,
                has_focused_pane: false,
            }
        );
        assert_eq!(
            with_launch_editor,
            CommandCtx {
                is_terminal_mode: true,
                has_active_tab: true,
                has_focused_pane: false,
            }
        );
        assert_eq!(
            with_shell,
            CommandCtx {
                is_terminal_mode: true,
                has_active_tab: true,
                has_focused_pane: true,
            }
        );
        assert!(!in_containers.is_terminal_mode);
    }
}
