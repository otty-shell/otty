//! Delivers app commands, whichever way the user asked for them: a
//! shortcut, a command palette entry or a header button. This is the one
//! place that knows which intents a command stands for. It only translates:
//! whether a command can run now is checked before it gets here, by the
//! source that sent it (see `AppCommand::is_available`).

use iced::Task;
use iced::widget::pane_grid;
use otty_app_domain::commands::AppCommand;
use otty_app_notifications::NotificationsIntent;
use otty_app_palette::PaletteIntent;
use otty_app_settings::SettingsIntent;
use otty_app_tabs::TabsIntent;
use otty_app_terminal::TerminalIntent;

use super::{modes, notifications, palette, settings, sidebar, tabs, terminal};
use crate::app::App;
use crate::message::AppEvent;
use crate::modes::ModesIntent;
use crate::sidebar::SidebarIntent;

/// Run `command` through the intents it stands for, in the same update. The
/// source of the command already checked that it can run now.
pub(crate) fn deliver(app: &mut App, command: AppCommand) -> Task<AppEvent> {
    // Every command is listed, so a new one can not be forgotten here.
    match command {
        AppCommand::ToggleLeftPanel => {
            sidebar::deliver(app, SidebarIntent::ToggleLeft)
        },
        AppCommand::ToggleFilesPanel => {
            sidebar::deliver(app, SidebarIntent::ToggleRight)
        },
        AppCommand::ToggleFocusMode => {
            sidebar::deliver(app, SidebarIntent::ToggleFocusMode)
        },
        AppCommand::SelectModeAt { position } => {
            modes::deliver(app, ModesIntent::SelectAt { position })
        },
        AppCommand::SwitchMode(mode) => {
            modes::deliver(app, ModesIntent::Select(mode))
        },
        AppCommand::ToggleModesMenu => {
            modes::deliver(app, ModesIntent::ToggleMenu)
        },
        // The name field of a new folder lives in the Launches section, so
        // the section shows first; the quick launch feature (issue 12) then
        // starts the folder.
        AppCommand::NewFolder => {
            sidebar::deliver(app, SidebarIntent::RevealLaunches)
        },
        AppCommand::OpenPalette => palette::deliver(app, PaletteIntent::Open),
        AppCommand::OpenSettings => {
            settings::deliver(app, SettingsIntent::Open)
        },
        AppCommand::OpenNotifications => {
            notifications::deliver(app, NotificationsIntent::OpenList)
        },
        AppCommand::NewTerminal => {
            tabs::deliver(app, TabsIntent::OpenTerminalTab)
        },
        AppCommand::CloseSession => tabs::deliver(app, TabsIntent::CloseActive),
        AppCommand::SplitRight => terminal::deliver(
            app,
            TerminalIntent::SplitFocusedPane {
                axis: pane_grid::Axis::Vertical,
            },
        ),
        AppCommand::SplitDown => terminal::deliver(
            app,
            TerminalIntent::SplitFocusedPane {
                axis: pane_grid::Axis::Horizontal,
            },
        ),
        // The quick launch feature (issue 12) opens the launch editor.
        AppCommand::NewLaunch => Task::none(),
    }
}

#[cfg(test)]
mod tests {
    use otty_app_domain::mode::Mode;
    use otty_ui_kit::components::command_palette::input_id;

    use super::*;
    use crate::testing::task_outputs::{focuses_nothing, focuses_widget};
    use crate::testing::window::app;

    #[test]
    fn panel_commands_toggle_their_panel() {
        let mut app = app();
        let before = app.sidebar.vm();

        let _left = deliver(&mut app, AppCommand::ToggleLeftPanel);
        let _files = deliver(&mut app, AppCommand::ToggleFilesPanel);

        let after = app.sidebar.vm();
        assert_ne!(after.is_left_open, before.is_left_open);
        assert_ne!(after.is_right_open, before.is_right_open);

        let _focus = deliver(&mut app, AppCommand::ToggleFocusMode);

        assert!(app.sidebar.vm().is_focus_mode);
    }

    #[test]
    fn mode_commands_switch_the_mode_and_open_the_menu() {
        let mut app = app();

        let _at = deliver(&mut app, AppCommand::SelectModeAt { position: 1 });
        let at_one = app.modes.active();
        let _switch = deliver(&mut app, AppCommand::SwitchMode(Mode::Terminal));
        let switched = app.modes.active();
        let _menu = deliver(&mut app, AppCommand::ToggleModesMenu);

        assert_eq!(at_one, Mode::Containers);
        assert_eq!(switched, Mode::Terminal);
        assert!(app.modes.is_menu_open());
    }

    /// The palette opens and its input takes the keyboard.
    #[test]
    fn open_palette_opens_the_palette() {
        let mut app = app();

        let task = deliver(&mut app, AppCommand::OpenPalette);

        assert!(app.palette.is_open());
        assert!(focuses_widget(task, &input_id()));
    }

    /// The bell and the palette entry open the list, which marks every
    /// notification seen.
    #[test]
    fn open_notifications_opens_the_list() {
        let mut app = app();
        let _push = notifications::deliver(
            &mut app,
            NotificationsIntent::Push(String::from("disk full")),
        );

        let _open = deliver(&mut app, AppCommand::OpenNotifications);

        assert!(app.notifications.is_list_open());
        assert_eq!(app.notifications.unseen_count(), 0);
    }

    /// The gear, the shortcut and the palette entry open Settings over the
    /// active mode, which stays, and it takes the keyboard.
    #[test]
    fn open_settings_opens_the_dialog_and_keeps_the_mode() {
        for mode in [Mode::Terminal, Mode::Containers] {
            let mut app = app();
            let _ = deliver(&mut app, AppCommand::SwitchMode(mode));

            let task = deliver(&mut app, AppCommand::OpenSettings);

            assert!(app.settings.is_open());
            assert_eq!(app.modes.active(), mode);
            assert!(focuses_nothing(task));
        }
    }

    /// New terminal opens a tab named after the shell, and Close session
    /// closes the active tab.
    #[test]
    fn new_terminal_opens_a_tab_and_close_session_closes_the_active_one() {
        let mut app = app();
        let _first = deliver(&mut app, AppCommand::NewTerminal);
        let _second = deliver(&mut app, AppCommand::NewTerminal);
        let opened = app.tabs.len();

        let _close = deliver(&mut app, AppCommand::CloseSession);

        assert_eq!(opened, 2);
        assert_eq!(app.tabs.len(), 1);
        assert_eq!(app.tabs.active_tab_id(), Some(0));
        assert_eq!(app.tabs.active_tab_title(), Some(app.shell_session.name()));
        assert_eq!(app.terminal.pane_count(), Some(1));
    }

    /// Split right and Split down split the focused pane of the tab in
    /// front.
    #[test]
    fn the_split_commands_split_the_focused_pane() {
        let mut app = app();
        let _tab = deliver(&mut app, AppCommand::NewTerminal);

        let _right = deliver(&mut app, AppCommand::SplitRight);
        let _down = deliver(&mut app, AppCommand::SplitDown);

        assert_eq!(app.terminal.pane_count(), Some(3));
    }

    /// The Launches section shows first, so the name field of the new
    /// folder is on screen.
    #[test]
    fn new_folder_reveals_the_launches_section() {
        let mut app = app();
        let _hide = deliver(&mut app, AppCommand::ToggleLeftPanel);
        assert!(!app.sidebar.vm().is_left_open);

        let _folder = deliver(&mut app, AppCommand::NewFolder);

        let sidebar = app.sidebar.vm();
        assert!(sidebar.is_left_open);
        assert!(sidebar.is_launches_open);
    }
}
