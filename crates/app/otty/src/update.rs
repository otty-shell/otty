use iced::Task;
use otty_app_domain::commands::AppCommand;
use otty_app_domain::keys::Platform;
use otty_app_domain::shortcuts::AppShortcut;
use otty_app_notifications::NotificationsIntent;
use otty_app_slots::OverlayKind;

use crate::app::App;
use crate::dispatch;
use crate::guards::{
    MenuGuard, Overlay, command_ctx, context_menu_guard, is_escape_press,
    top_overlay,
};
use crate::keys::shortcut_for_key_press;
use crate::message::AppEvent;
use crate::modes::ModesIntent;
use crate::sidebar::SidebarIntent;

/// Handle input from the user. Open menus decide first what happens to it,
/// once, on what the user did.
pub(crate) fn update_input(app: &mut App, event: AppEvent) -> Task<AppEvent> {
    let event = as_command(event);

    filter_input(app, event)
}

/// Handle an event that does not come from the user directly: it skips the
/// checks of [`update_input`], so it can not get lost.
pub(crate) fn update_internal(
    app: &mut App,
    event: AppEvent,
) -> Task<AppEvent> {
    // The containers feature (issue 13) is told here, after every event,
    // whether the right panel is on screen, as the old `finish_update` did.
    dispatch::dispatch(app, event)
}

/// Check input from the user before it runs. A shortcut arrives here as a
/// command, and this is where a shortcut whose command can not run now is
/// dropped (`AppCommand::is_available`), after it closed the open menus
/// like any other shortcut.
fn filter_input(app: &mut App, event: AppEvent) -> Task<AppEvent> {
    let top = top_overlay(app);

    // Keys typed under a menu or a dialog belong to it, even if a terminal
    // still had keyboard focus: the terminal feature (issue 11) adds that
    // check here, first.

    // Esc closes the overlay on top. A focused terminal keeps Esc for its
    // shell, so every overlay takes the focus from the terminals when it
    // opens.
    if let Some(top) = top
        && is_escape_press(&event)
    {
        return dispatch::dispatch(app, top.escape_event());
    }

    // A dialog waits for its answer: Settings (issue 09), the SSH secret
    // prompt (issue 12) and the Containers dialogs (issue 14) add the dialog
    // check here.

    // Mode shortcuts still work while the All modes menu is open: the menu
    // shortcut closes it and a mode shortcut switches, which closes it too.
    // Every other key stays ignored like for the other menus.
    if app.modes.is_menu_open() && is_mode_shortcut(&event) {
        return dispatch::dispatch(app, event);
    }

    // The palette shortcut under the open palette (issue 08), an inline edit
    // of the Launches tree (issue 12) and the choices of a header menu
    // (issues 10 to 12) are checked here.

    let mut pre_dispatch_tasks = Vec::new();

    if top.map(Overlay::kind) == Some(OverlayKind::Menu) {
        match context_menu_guard(&event) {
            MenuGuard::Allow => {},
            MenuGuard::Ignore => return Task::none(),
            MenuGuard::Dismiss => return close_all_menus(app),
            MenuGuard::DismissAndRun => {
                pre_dispatch_tasks.push(close_all_menus(app));
            },
        }
    }

    // A shortcut whose command can not run now (for example Close session
    // without tabs) does nothing more. The menus it closed stay closed.
    if !can_run_now(app, &event) {
        return Task::batch(pre_dispatch_tasks);
    }

    pre_dispatch_tasks.push(dispatch::dispatch(app, event));
    Task::batch(pre_dispatch_tasks)
}

/// Turn a key press that is an app shortcut into its command, so the checks
/// see a shortcut once, as a command. A key no widget captured comes from
/// the keyboard subscription; the terminal feature (issue 11) adds the
/// binding a focused terminal reports. Every other event stays as it is.
fn as_command(event: AppEvent) -> AppEvent {
    let command = match &event {
        AppEvent::Keyboard(key) => {
            shortcut_for_key_press(key, Platform::current())
                .map(AppShortcut::command)
        },
        _ => None,
    };

    match command {
        Some(command) => AppEvent::Command(command),
        None => event,
    }
}

/// Return `false` for a command that can not run now; every other event can
/// run.
fn can_run_now(app: &App, event: &AppEvent) -> bool {
    match event {
        AppEvent::Command(command) => command.is_available(&command_ctx(app)),
        _ => true,
    }
}

fn is_mode_shortcut(event: &AppEvent) -> bool {
    matches!(
        event,
        AppEvent::Command(
            AppCommand::SelectModeAt { .. } | AppCommand::ToggleModesMenu
        )
    )
}

/// Close every menu: the one on top and any left open under it. The
/// palette (issue 08), the context menus of the terminal (issue 11) and the
/// Launches tree (issue 12) and the Containers row menu (issue 14) close
/// here too.
fn close_all_menus(app: &mut App) -> Task<AppEvent> {
    Task::batch([
        dispatch::dispatch(
            app,
            AppEvent::Notifications(NotificationsIntent::CloseList),
        ),
        dispatch::dispatch(
            app,
            AppEvent::Sidebar(SidebarIntent::HeaderMenuDismiss),
        ),
        dispatch::dispatch(app, AppEvent::Modes(ModesIntent::CloseMenu)),
    ])
}

#[cfg(test)]
mod tests {
    use otty_app_domain::mode::Mode;
    use otty_app_slots::Feature;

    use super::*;
    use crate::chrome::ChromeIntent;
    use crate::sidebar::types::HeaderMenu;
    use crate::testing::keyboard::{escape_press, key_press, shortcut_press};
    use crate::testing::task_outputs::collect_outputs;
    use crate::testing::window::{app, open_notification_list, show_toast};

    fn app_with_open_modes_menu() -> App {
        let mut app = app();
        let _ =
            update_input(&mut app, AppEvent::Modes(ModesIntent::ToggleMenu));
        assert!(app.modes.is_menu_open());
        app
    }

    fn app_with_open_header_menu() -> App {
        let mut app = app();
        let _ = update_input(
            &mut app,
            AppEvent::Sidebar(SidebarIntent::HeaderMenuOpen(
                HeaderMenu::Sessions,
            )),
        );
        assert!(app.sidebar.has_header_menu_open());
        app
    }

    fn app_with_open_notification_list() -> App {
        let mut app = app();
        open_notification_list(&mut app);
        assert!(app.notifications.is_list_open());
        app
    }

    fn escape() -> AppEvent {
        AppEvent::Keyboard(escape_press())
    }

    /// Press `shortcut` as the keyboard subscription delivers it.
    fn press(app: &mut App, shortcut: AppShortcut) {
        let _ = update_input(app, AppEvent::Keyboard(shortcut_press(shortcut)));
    }

    #[test]
    fn all_modes_shortcut_closes_the_open_modes_menu() {
        let mut app = app_with_open_modes_menu();

        press(&mut app, AppShortcut::AllModesMenu);

        assert!(!app.modes.is_menu_open());
    }

    #[test]
    fn mode_shortcut_switches_while_the_modes_menu_is_open() {
        let mut app = app_with_open_modes_menu();

        press(&mut app, AppShortcut::PinnedMode1);

        assert_eq!(app.modes.active(), Mode::Containers);
        assert!(!app.modes.is_menu_open());
    }

    #[test]
    fn other_shortcuts_close_the_modes_menu_and_run() {
        let mut app = app_with_open_modes_menu();

        press(&mut app, AppShortcut::FocusMode);

        assert!(!app.modes.is_menu_open());
        assert!(app.sidebar.vm().is_focus_mode);
    }

    #[test]
    fn esc_closes_each_menu() {
        for mut app in [
            app_with_open_notification_list(),
            app_with_open_modes_menu(),
            app_with_open_header_menu(),
        ] {
            let opened = top_overlay(&app);

            let _ = update_input(&mut app, escape());

            assert!(opened.is_some());
            assert_eq!(top_overlay(&app), None, "{opened:?}");
        }
    }

    /// The modes menu over a header menu: each Esc closes the one on top
    /// and leaves the other open.
    #[test]
    fn esc_closes_only_the_overlay_on_top() {
        let mut app = app_with_open_header_menu();
        let _ = app.modes.reduce(ModesIntent::ToggleMenu, ());
        let mut on_top = Vec::new();

        for _ in 0..2 {
            let _ = update_input(&mut app, escape());
            on_top.push(top_overlay(&app));
        }

        assert_eq!(on_top, [Some(Overlay::HeaderMenu), None]);
    }

    #[test]
    fn esc_with_nothing_open_changes_nothing() {
        let mut app = app();

        let events = collect_outputs(update_input(&mut app, escape()));

        assert!(events.is_empty());
        assert_eq!(top_overlay(&app), None);
    }

    #[test]
    fn pin_and_unpin_keep_the_modes_menu_open() {
        let mut app = app_with_open_modes_menu();

        let _save = update_input(
            &mut app,
            AppEvent::Modes(ModesIntent::Unpin(Mode::Containers)),
        );

        assert!(app.modes.is_menu_open());
        assert_eq!(app.modes.vm().pinned_count, 0);
    }

    #[test]
    fn choosing_a_mode_in_the_menu_switches_and_closes_it() {
        let mut app = app_with_open_modes_menu();

        let _ = update_input(
            &mut app,
            AppEvent::Modes(ModesIntent::Select(Mode::Containers)),
        );

        assert_eq!(app.modes.active(), Mode::Containers);
        assert!(!app.modes.is_menu_open());
    }

    #[test]
    fn another_app_action_closes_the_modes_menu_like_the_header_menus() {
        let mut app = app_with_open_modes_menu();
        let was_left_open = app.sidebar.vm().is_left_open;

        let _ = update_input(
            &mut app,
            AppEvent::Sidebar(SidebarIntent::ToggleLeft),
        );

        assert!(!app.modes.is_menu_open());
        assert_eq!(app.sidebar.vm().is_left_open, was_left_open);
    }

    #[test]
    fn a_panel_button_under_an_open_menu_only_closes_the_menu() {
        let presses = [
            ChromeIntent::ToggleLeftPanel,
            ChromeIntent::ToggleFilesPanel,
            ChromeIntent::ToggleFocusMode,
        ];

        for opened in [
            app_with_open_notification_list,
            app_with_open_modes_menu,
            app_with_open_header_menu,
        ] {
            for press in presses.clone() {
                let mut app = opened();
                let panels = app.sidebar.vm();

                let _ = update_input(&mut app, AppEvent::Chrome(press));

                let after = app.sidebar.vm();
                assert_eq!(top_overlay(&app), None);
                assert_eq!(after.is_left_open, panels.is_left_open);
                assert_eq!(after.is_right_open, panels.is_right_open);
                assert_eq!(after.is_focus_mode, panels.is_focus_mode);
            }
        }
    }

    /// A header button runs its command in the same update as the press.
    #[test]
    fn a_panel_button_toggles_its_panel() {
        let mut app = app();

        let _ = update_input(
            &mut app,
            AppEvent::Chrome(ChromeIntent::ToggleLeftPanel),
        );

        assert!(!app.sidebar.vm().is_left_open);
    }

    #[test]
    fn a_shortcut_under_an_open_header_menu_closes_it_and_runs() {
        let mut app = app_with_open_header_menu();

        press(&mut app, AppShortcut::LeftPanel);

        assert!(!app.sidebar.has_header_menu_open());
        assert!(!app.sidebar.vm().is_left_open);
    }

    #[test]
    fn a_plain_key_under_an_open_menu_keeps_it() {
        let mut app = app_with_open_modes_menu();
        let plain = key_press(
            iced::keyboard::key::Physical::Code(
                iced::keyboard::key::Code::KeyA,
            ),
            "a",
            iced::keyboard::Modifiers::empty(),
        );

        let _ = update_input(&mut app, AppEvent::Keyboard(plain));

        assert!(app.modes.is_menu_open());
    }

    #[test]
    fn a_window_button_leaves_the_menu_open() {
        let mut app = app_with_open_modes_menu();

        let _task = update_input(
            &mut app,
            AppEvent::Chrome(ChromeIntent::MinimizeWindow),
        );

        assert!(app.modes.is_menu_open());
    }

    /// The bell opens the list, which marks every notification seen: the
    /// badge goes away.
    #[test]
    fn the_bell_opens_the_list_and_marks_every_notification_seen() {
        let mut app = app();
        show_toast(&mut app, "disk full");
        show_toast(&mut app, "busy");
        let unseen = app.notifications.unseen_count();

        let _ = update_input(
            &mut app,
            AppEvent::Chrome(ChromeIntent::OpenNotifications),
        );

        assert_eq!(unseen, 2);
        assert!(app.notifications.is_list_open());
        assert_eq!(app.notifications.unseen_count(), 0);
        assert_eq!(top_overlay(&app), Some(Overlay::NotificationsList));
    }

    /// The bell is a header button like the panel toggles: under any open
    /// menu, the list too, it only closes the menus.
    #[test]
    fn the_bell_under_an_open_menu_only_closes_it() {
        for mut app in [
            app_with_open_notification_list(),
            app_with_open_modes_menu(),
        ] {
            let _ = update_input(
                &mut app,
                AppEvent::Chrome(ChromeIntent::OpenNotifications),
            );

            assert_eq!(top_overlay(&app), None);
        }
    }

    #[test]
    fn a_shortcut_under_the_list_closes_it_and_runs() {
        let mut app = app_with_open_notification_list();

        press(&mut app, AppShortcut::FocusMode);

        assert!(!app.notifications.is_list_open());
        assert!(app.sidebar.vm().is_focus_mode);
    }

    /// Owner: the cross of a row removes only that notification, and the
    /// list stays open for the next one.
    #[test]
    fn a_row_cross_keeps_the_list_open() {
        let mut app = app();
        show_toast(&mut app, "disk full");
        show_toast(&mut app, "busy");
        open_notification_list(&mut app);

        // The first notification gets the first id.
        let _ = update_input(
            &mut app,
            AppEvent::Notifications(NotificationsIntent::Remove(0)),
        );

        let mut ui = iced_test::simulator(crate::view::view(&app));
        assert!(app.notifications.is_list_open());
        assert_eq!(top_overlay(&app), Some(Overlay::NotificationsList));
        assert!(ui.find("disk full").is_err());
        assert!(ui.find("busy").is_ok());
    }

    #[test]
    fn a_shortcut_key_becomes_its_command() {
        let event = as_command(AppEvent::Keyboard(shortcut_press(
            AppShortcut::Settings,
        )));

        assert!(matches!(event, AppEvent::Command(AppCommand::OpenSettings)));
    }

    #[test]
    fn a_plain_key_stays_a_key() {
        let plain = key_press(
            iced::keyboard::key::Physical::Code(
                iced::keyboard::key::Code::KeyA,
            ),
            "a",
            iced::keyboard::Modifiers::empty(),
        );

        let event = as_command(AppEvent::Keyboard(plain));

        assert!(matches!(event, AppEvent::Keyboard(_)));
    }

    #[test]
    fn a_shortcut_key_from_the_subscription_runs_its_command() {
        let mut app = app();

        press(&mut app, AppShortcut::FilesPanel);

        assert!(!app.sidebar.vm().is_right_open);
    }

    #[test]
    fn a_shortcut_that_can_not_run_under_an_open_menu_still_closes_it() {
        // Without tabs Close session can not run.
        let mut app = app_with_open_header_menu();

        let events = collect_outputs(update_input(
            &mut app,
            AppEvent::Keyboard(shortcut_press(AppShortcut::CloseSession)),
        ));

        assert!(!app.sidebar.has_header_menu_open());
        assert!(events.is_empty());
    }

    /// New folder acts on the Launches panel, which shows only in Terminal
    /// mode.
    #[test]
    fn a_command_that_can_not_run_now_does_nothing() {
        let mut app = app();
        let _ = app.modes.reduce(ModesIntent::Select(Mode::Containers), ());
        let _ = app.sidebar.reduce(SidebarIntent::ToggleLeft, ());

        let _ =
            update_input(&mut app, AppEvent::Command(AppCommand::NewFolder));

        assert!(!app.sidebar.vm().is_left_open);
    }
}
