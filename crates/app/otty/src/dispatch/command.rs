//! Runs app commands, whichever way the user asked for them: a
//! shortcut, a command palette entry or a header button. This is the one
//! place that knows which intents a command stands for. It only translates:
//! whether a command can run now is checked before it gets here, by the
//! source that sent it (see `AppCommand::is_available`).

use iced::Task;
use iced::widget::pane_grid;
use otty_app_domain::commands::AppCommand;
use otty_app_modes::ModesIntent;
use otty_app_notifications::NotificationsIntent;
use otty_app_palette::PaletteIntent;
use otty_app_quick_launch::QuickLaunchIntent;
use otty_app_settings::SettingsIntent;
use otty_app_tabs::TabsIntent;
use otty_app_terminal::TerminalIntent;

use super::deliver;
use crate::app::App;
use crate::frame::sidebar::SidebarIntent;
use crate::message::AppEvent;

/// Run `command` through the intents it stands for, in the same update. The
/// source of the command already checked that it can run now.
pub(crate) fn run(app: &mut App, command: AppCommand) -> Task<AppEvent> {
    // Every command is listed, so a new one can not be forgotten here.
    match command {
        AppCommand::ToggleLeftPanel => deliver(app, SidebarIntent::ToggleLeft),
        AppCommand::ToggleFilesPanel => {
            deliver(app, SidebarIntent::ToggleRight)
        },
        AppCommand::ToggleFocusMode => {
            deliver(app, SidebarIntent::ToggleFocusMode)
        },
        AppCommand::SelectModeAt { position } => {
            deliver(app, ModesIntent::SelectAt { position })
        },
        AppCommand::SwitchMode(mode) => deliver(app, ModesIntent::Select(mode)),
        AppCommand::ToggleModesMenu => deliver(app, ModesIntent::ToggleMenu),
        AppCommand::NewFolder => name_new_folder(app),
        AppCommand::OpenPalette => deliver(app, PaletteIntent::Open),
        AppCommand::OpenSettings => deliver(app, SettingsIntent::Open),
        AppCommand::OpenNotifications => {
            deliver(app, NotificationsIntent::OpenList)
        },
        AppCommand::NewTerminal => {
            deliver(app, TabsIntent::OpenTerminalTab { folder: None })
        },
        AppCommand::CloseSession => deliver(app, TabsIntent::CloseActive),
        AppCommand::SplitRight => deliver(
            app,
            TerminalIntent::SplitFocusedPane {
                axis: pane_grid::Axis::Vertical,
            },
        ),
        AppCommand::SplitDown => deliver(
            app,
            TerminalIntent::SplitFocusedPane {
                axis: pane_grid::Axis::Horizontal,
            },
        ),
        AppCommand::NewLaunch => {
            deliver(app, QuickLaunchIntent::HeaderCreateCommand)
        },
    }
}

/// The name field of a new folder lives in the Launches section, so the
/// section shows first, then quick launch starts the folder.
fn name_new_folder(app: &mut App) -> Task<AppEvent> {
    let revealed = deliver(app, SidebarIntent::RevealLaunches);
    let named = deliver(app, QuickLaunchIntent::HeaderCreateFolder);

    Task::batch([revealed, named])
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

        let _left = run(&mut app, AppCommand::ToggleLeftPanel);
        let _files = run(&mut app, AppCommand::ToggleFilesPanel);

        let after = app.sidebar.vm();
        assert_ne!(after.is_left_open, before.is_left_open);
        assert_ne!(after.is_right_open, before.is_right_open);

        let _focus = run(&mut app, AppCommand::ToggleFocusMode);

        assert!(app.sidebar.vm().is_focus_mode);
    }

    #[test]
    fn mode_commands_switch_the_mode_and_open_the_menu() {
        let mut app = app();

        let _at = run(&mut app, AppCommand::SelectModeAt { position: 1 });
        let at_one = app.modes.active();
        let _switch = run(&mut app, AppCommand::SwitchMode(Mode::Terminal));
        let switched = app.modes.active();
        let _menu = run(&mut app, AppCommand::ToggleModesMenu);

        assert_eq!(at_one, Mode::Containers);
        assert_eq!(switched, Mode::Terminal);
        assert!(app.modes.is_menu_open());
    }

    /// The palette opens and its input takes the keyboard.
    #[test]
    fn open_palette_opens_the_palette() {
        let mut app = app();

        let task = run(&mut app, AppCommand::OpenPalette);

        assert!(app.palette.is_open());
        assert!(focuses_widget(task, &input_id()));
    }

    /// The bell and the palette entry open the list, which marks every
    /// notification seen.
    #[test]
    fn open_notifications_opens_the_list() {
        let mut app = app();
        let _push = deliver(
            &mut app,
            NotificationsIntent::Push(String::from("disk full")),
        );

        let _open = run(&mut app, AppCommand::OpenNotifications);

        assert!(app.notifications.is_list_open());
        assert_eq!(app.notifications.unseen_count(), 0);
    }

    /// The gear, the shortcut and the palette entry open Settings over the
    /// active mode, which stays, and it takes the keyboard.
    #[test]
    fn open_settings_opens_the_dialog_and_keeps_the_mode() {
        for mode in [Mode::Terminal, Mode::Containers] {
            let mut app = app();
            let _ = run(&mut app, AppCommand::SwitchMode(mode));

            let task = run(&mut app, AppCommand::OpenSettings);

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
        let _first = run(&mut app, AppCommand::NewTerminal);
        let _second = run(&mut app, AppCommand::NewTerminal);
        let opened = app.tabs.len();

        let _close = run(&mut app, AppCommand::CloseSession);

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
        let _tab = run(&mut app, AppCommand::NewTerminal);

        let _right = run(&mut app, AppCommand::SplitRight);
        let _down = run(&mut app, AppCommand::SplitDown);

        assert_eq!(app.terminal.pane_count(), Some(3));
    }

    /// The Launches section shows first, so the name field of the new
    /// folder is on screen.
    #[test]
    fn new_folder_reveals_the_launches_section() {
        let mut app = app();
        let _hide = run(&mut app, AppCommand::ToggleLeftPanel);
        assert!(!app.sidebar.vm().is_left_open);

        let _folder = run(&mut app, AppCommand::NewFolder);

        let sidebar = app.sidebar.vm();
        assert!(sidebar.is_left_open);
        assert!(sidebar.is_launches_open);
    }
}
