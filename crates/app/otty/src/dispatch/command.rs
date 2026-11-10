//! Delivers app commands, whichever way the user asked for them: a shortcut
//! or a header button (the palette, issue 08, comes here too). This is the
//! one place that knows which intents a command stands for. It only
//! translates: whether a command can run now is checked before it gets
//! here, by the source that sent it (see `AppCommand::is_available`).

use iced::Task;
use otty_app_domain::commands::AppCommand;
use otty_app_notifications::NotificationsIntent;

use super::{modes, notifications, sidebar};
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
        // The palette (issue 08) opens.
        AppCommand::OpenPalette => Task::none(),
        // The settings popup (issue 09) opens.
        AppCommand::OpenSettings => Task::none(),
        AppCommand::OpenNotifications => {
            notifications::deliver(app, NotificationsIntent::OpenList)
        },
        // The tabs feature (issue 10) opens or closes a tab.
        AppCommand::NewTerminal | AppCommand::CloseSession => Task::none(),
        // The terminal feature (issue 11) splits the focused pane.
        AppCommand::SplitRight | AppCommand::SplitDown => Task::none(),
        // The quick launch feature (issue 12) opens the launch editor.
        AppCommand::NewLaunch => Task::none(),
    }
}

#[cfg(test)]
mod tests {
    use otty_app_domain::mode::Mode;

    use super::*;
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
