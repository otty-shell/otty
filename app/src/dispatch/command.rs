//! Delivers app commands, whichever way the user asked for them: a
//! shortcut, a command palette entry or a header button. This is the one
//! place that knows which widget intents a command stands for. It only
//! translates: whether a command can run now is checked before it gets
//! here, by the source that sent it (see [`AppCommand::is_available`]).

use iced::Task;
use iced::widget::pane_grid;

use crate::commands::AppCommand;
use crate::event::AppEvent;
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

/// Turn `command` into the widget intents it stands for. The source of the
/// command already checked that it can run now.
pub(crate) fn deliver(command: AppCommand) -> Task<AppEvent> {
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
    use crate::domain::mode::Mode;
    use crate::testing::task_outputs::collect_outputs;

    fn delivered(command: AppCommand) -> Vec<AppEvent> {
        collect_outputs(deliver(command))
    }

    #[test]
    fn open_palette_opens_the_palette() {
        let events = delivered(AppCommand::OpenPalette);

        assert!(matches!(
            events.as_slice(),
            [AppEvent::Palette(PaletteEvent::Intent(PaletteIntent::Open))]
        ));
    }

    #[test]
    fn panel_commands_toggle_their_panel() {
        let left = delivered(AppCommand::ToggleLeftPanel);
        let files = delivered(AppCommand::ToggleFilesPanel);
        let focus = delivered(AppCommand::ToggleFocusMode);

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

    /// Settings opens as a popup, with no mode switch.
    #[test]
    fn open_settings_opens_the_settings_popup() {
        let events = delivered(AppCommand::OpenSettings);

        assert!(matches!(
            events.as_slice(),
            [AppEvent::Settings(SettingsEvent::Intent(
                SettingsIntent::Open
            ))]
        ));
    }

    #[test]
    fn open_notifications_opens_the_list() {
        let events = delivered(AppCommand::OpenNotifications);

        assert!(matches!(
            events.as_slice(),
            [AppEvent::Notifications(NotificationsEvent::Intent(
                NotificationsIntent::OpenList
            ))]
        ));
    }

    #[test]
    fn new_terminal_opens_a_shell_tab() {
        let events = delivered(AppCommand::NewTerminal);

        assert!(matches!(
            events.as_slice(),
            [AppEvent::Tabs(TabsEvent::Intent(
                TabsIntent::OpenTerminalTab
            ))]
        ));
    }

    #[test]
    fn close_session_closes_the_active_tab() {
        let events = delivered(AppCommand::CloseSession);

        assert!(matches!(
            events.as_slice(),
            [AppEvent::Tabs(TabsEvent::Intent(TabsIntent::CloseActive))]
        ));
    }

    #[test]
    fn split_right_splits_the_focused_pane_to_the_right() {
        let events = delivered(AppCommand::SplitRight);

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
    fn split_down_splits_the_focused_pane_down() {
        let events = delivered(AppCommand::SplitDown);

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
    fn select_mode_at_asks_the_modes_widget_for_that_position() {
        let events = delivered(AppCommand::SelectModeAt { position: 2 });

        assert!(matches!(
            events.as_slice(),
            [AppEvent::Modes(ModesEvent::Intent(ModesIntent::SelectAt {
                position: 2
            }))]
        ));
    }

    #[test]
    fn switch_mode_selects_that_mode() {
        let events = delivered(AppCommand::SwitchMode(Mode::Containers));

        assert!(matches!(
            events.as_slice(),
            [AppEvent::Modes(ModesEvent::Intent(ModesIntent::Select(
                Mode::Containers
            )))]
        ));
    }

    #[test]
    fn toggle_modes_menu_toggles_the_menu() {
        let events = delivered(AppCommand::ToggleModesMenu);

        assert!(matches!(
            events.as_slice(),
            [AppEvent::Modes(ModesEvent::Intent(ModesIntent::ToggleMenu))]
        ));
    }

    #[test]
    fn new_launch_runs_the_launches_header_action() {
        let events = delivered(AppCommand::NewLaunch);

        assert!(matches!(
            events.as_slice(),
            [AppEvent::QuickLaunch(QuickLaunchEvent::Intent(
                QuickLaunchIntent::HeaderCreateCommand
            ))]
        ));
    }

    #[test]
    fn new_folder_reveals_the_launches_section_before_starting_the_folder() {
        let events = delivered(AppCommand::NewFolder);

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
}
