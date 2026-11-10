use iced::Task;
use otty_app_containers::ContainersIntent;
use otty_app_domain::commands::AppCommand;
use otty_app_domain::keys::Platform;
use otty_app_domain::shortcuts::AppShortcut;
use otty_app_notifications::NotificationsIntent;
use otty_app_palette::PaletteIntent;
use otty_app_quick_launch::QuickLaunchIntent;
use otty_app_slots::OverlayKind;
use otty_app_terminal::TerminalIntent;

use crate::app::App;
use crate::dispatch;
use crate::guards::{
    MenuGuard, Overlay, command_ctx, context_menu_guard, dialog_guard,
    inline_edit_guard, is_escape_press, is_header_menu_choice,
    is_terminal_input_kept_out, top_overlay,
};
use crate::keys::shortcut_for_key_press;
use crate::message::AppEvent;
use crate::modes::ModesIntent;
use crate::sidebar::SidebarIntent;

/// Handle input from the user. Open menus, dialogs and an inline edit decide
/// first what happens to it, once, on what the user did.
pub(crate) fn update_input(app: &mut App, event: AppEvent) -> Task<AppEvent> {
    let event = as_command(event);
    let task = filter_input(app, event);

    finish_update(app, task)
}

/// Handle an event that does not come from the user directly: it skips the
/// checks of [`update_input`], so it can not get lost.
pub(crate) fn update_internal(
    app: &mut App,
    event: AppEvent,
) -> Task<AppEvent> {
    let task = dispatch::dispatch(app, event);

    finish_update(app, task)
}

/// Tell the containers feature when the update changed whether the right
/// panel is on screen. The sidebar alone knows that, and many paths change
/// it (the panel toggle, focus mode, a dragged edge), so it is read here,
/// after every update, instead of by each path.
fn finish_update(app: &mut App, task: Task<AppEvent>) -> Task<AppEvent> {
    let is_hidden = !app.sidebar.vm().is_right_open;
    if app.containers.is_right_panel_hidden() == is_hidden {
        return task;
    }

    let told = dispatch::containers::deliver(
        app,
        ContainersIntent::RightPanelHidden(is_hidden),
    );

    Task::batch([task, told])
}

/// Check input from the user before it runs. A shortcut arrives here as a
/// command, and this is where a shortcut whose command can not run now is
/// dropped (`AppCommand::is_available`), after it closed the open menus and
/// cancelled an inline edit like any other shortcut.
fn filter_input(app: &mut App, event: AppEvent) -> Task<AppEvent> {
    let top = top_overlay(app);

    // Keys typed under a menu or a dialog belong to it, even if a terminal
    // still had keyboard focus.
    if is_terminal_input_kept_out(top, &event) {
        return Task::none();
    }

    // Esc closes the overlay on top, or cancels it for a dialog and the
    // inline edit. A focused terminal keeps Esc for its shell, so every
    // overlay takes the focus from the terminals when it opens.
    if let Some(top) = top
        && is_escape_press(&event)
    {
        return dispatch::dispatch(app, top.escape_event());
    }

    // A dialog waits for its answer. It lies over any menu, and the menu
    // stays open under it until the dialog is answered. Pointer moves pass
    // under every dialog, a hover over a Files row too.
    if let Some(dialog) =
        top.filter(|top| top.kind() == Some(OverlayKind::Dialog))
    {
        return match dialog_guard(dialog, &event) {
            MenuGuard::Allow => dispatch::dispatch(app, event),
            _ => Task::none(),
        };
    }

    // Mode shortcuts still work while the All modes menu is open: the menu
    // shortcut closes it and a mode shortcut switches, which closes it too.
    // Every other key stays ignored like for the other menus.
    if app.modes.is_menu_open() && is_mode_shortcut(&event) {
        return dispatch::dispatch(app, event);
    }

    // The palette shortcut under the open palette gives its input the focus
    // again, and the palette keeps what was typed (owner, Q1 of the old
    // app).
    if top == Some(Overlay::Palette) && is_palette_shortcut(&event) {
        return dispatch::dispatch(app, event);
    }

    let mut pre_dispatch_tasks = Vec::new();

    // Any input but typing in the name field, pointer moves (a hover over a
    // Files row too) and terminal events cancels an inline edit first.
    if app.quick_launch.has_inline_edit() && inline_edit_guard(&event) {
        pre_dispatch_tasks.push(dispatch::dispatch(
            app,
            AppEvent::QuickLaunch(QuickLaunchIntent::CancelInlineEdit),
        ));
    }

    // A choice in the open header menu closes it and runs.
    if app.sidebar.has_header_menu_open() && is_header_menu_choice(&event) {
        pre_dispatch_tasks.push(dispatch::dispatch(
            app,
            AppEvent::Sidebar(SidebarIntent::HeaderMenuDismiss),
        ));
    } else if top.and_then(Overlay::kind) == Some(OverlayKind::Menu) {
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
    // without tabs) does nothing more. The menus it closed and the inline
    // edit it cancelled stay so.
    if !can_run_now(app, &event) {
        return Task::batch(pre_dispatch_tasks);
    }

    pre_dispatch_tasks.push(dispatch::dispatch(app, event));
    Task::batch(pre_dispatch_tasks)
}

/// Turn a key press that is an app shortcut into its command, on both
/// paths a key press can take: a key no widget captured, from the keyboard
/// subscription, and the binding a focused terminal reported (it keeps the
/// key, so the subscription never sees it). The checks then see a shortcut
/// once, as a command, whichever path it took. Every other event stays as
/// it is.
fn as_command(event: AppEvent) -> AppEvent {
    let command = match &event {
        AppEvent::Keyboard(key) => {
            shortcut_for_key_press(key, Platform::current())
                .map(AppShortcut::command)
        },
        AppEvent::Terminal(TerminalIntent::Widget(
            otty_ui_term::Event::BindingActionDispatched { action, .. },
        )) => Some(action.command()),
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

fn is_palette_shortcut(event: &AppEvent) -> bool {
    matches!(event, AppEvent::Command(AppCommand::OpenPalette))
}

/// Close every menu: the one on top and any left open under it. The
/// Containers row menu (issue 14) closes here too.
fn close_all_menus(app: &mut App) -> Task<AppEvent> {
    Task::batch([
        dispatch::dispatch(
            app,
            AppEvent::QuickLaunch(QuickLaunchIntent::ContextMenuDismiss),
        ),
        dispatch::dispatch(
            app,
            AppEvent::Terminal(TerminalIntent::CloseAllContextMenus),
        ),
        dispatch::dispatch(
            app,
            AppEvent::Notifications(NotificationsIntent::CloseList),
        ),
        dispatch::dispatch(app, AppEvent::Palette(PaletteIntent::Dismiss)),
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
    use otty_app_domain::quick_launch::WizardTabInit;
    use otty_app_explorer::ExplorerIntent;
    use otty_app_settings::SettingsIntent;
    use otty_app_slots::Feature;
    use otty_app_tabs::TabsIntent;
    use otty_ui_kit::components::command_palette::{PaletteAction, input_id};

    use super::*;
    use crate::chrome::ChromeIntent;
    use crate::sidebar::types::HeaderMenu;
    use crate::testing::keyboard::{escape_press, key_press, shortcut_press};
    use crate::testing::task_outputs::{
        collect_outputs, focuses_widget, runs_widget_operation,
    };
    use crate::testing::window::{
        app, edit_the_shell, open_launch_menu, open_notification_list,
        open_palette, open_pane_menu, open_secret_prompt, open_settings,
        open_shell_tab, open_terminal_tab, show_toast, start_inline_edit,
    };

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

    /// The open palette with `query` typed in it.
    fn app_with_palette_query(query: &str) -> App {
        let mut app = app();
        open_palette(&mut app);
        let _ = update_input(
            &mut app,
            AppEvent::Palette(PaletteIntent::Action(
                PaletteAction::QueryChanged(query.to_string()),
            )),
        );
        assert!(app.palette.is_open());
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

    fn app_with_open_pane_menu() -> App {
        let mut app = app();
        open_pane_menu(&mut app);
        assert!(app.terminal.has_context_menu());
        app
    }

    fn app_with_open_launch_menu() -> App {
        let mut app = app();
        open_launch_menu(&mut app);
        assert!(app.quick_launch.has_context_menu());
        app
    }

    #[test]
    fn esc_closes_each_menu() {
        for mut app in [
            app_with_open_notification_list(),
            app_with_palette_query("spl"),
            app_with_open_modes_menu(),
            app_with_open_header_menu(),
            app_with_open_pane_menu(),
            app_with_open_launch_menu(),
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

    /// Scenarios 5 and 6 of the old app: Esc means Cancel in the SSH
    /// prompt and in the inline edit.
    #[test]
    fn esc_cancels_the_secret_prompt_and_the_inline_edit() {
        let mut prompt = app();
        open_secret_prompt(&mut prompt);
        let mut edit = app();
        start_inline_edit(&mut edit);

        for mut app in [prompt, edit] {
            let opened = top_overlay(&app);

            let _ = update_input(&mut app, escape());

            assert!(opened.is_some());
            assert_eq!(top_overlay(&app), None, "{opened:?}");
        }
    }

    /// The SSH prompt over a menu over the inline edit: each Esc closes the
    /// one on top and leaves the others open.
    #[test]
    fn esc_closes_the_secret_prompt_then_the_menu_then_the_inline_edit() {
        let mut app = app();
        start_inline_edit(&mut app);
        let _ = app
            .sidebar
            .reduce(SidebarIntent::HeaderMenuOpen(HeaderMenu::Sessions), ());
        open_secret_prompt(&mut app);
        let mut on_top = Vec::new();

        for _ in 0..3 {
            let _ = update_input(&mut app, escape());
            on_top.push(top_overlay(&app));
        }

        assert_eq!(
            on_top,
            [Some(Overlay::HeaderMenu), Some(Overlay::InlineEdit), None]
        );
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
    fn hovering_a_files_row_under_an_open_menu_keeps_the_menu() {
        let mut app = app_with_open_modes_menu();

        let _ = update_input(
            &mut app,
            AppEvent::Explorer(ExplorerIntent::NodeHovered {
                path: Some(vec![String::from("src")]),
            }),
        );

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

    #[test]
    fn the_palette_shortcut_opens_the_palette() {
        let mut app = app();

        press(&mut app, AppShortcut::CommandPalette);

        assert!(app.palette.is_open());
        assert_eq!(top_overlay(&app), Some(Overlay::Palette));
    }

    #[test]
    fn the_palette_button_under_an_open_menu_only_closes_the_menu() {
        let mut app = app_with_open_modes_menu();

        let _ = update_input(
            &mut app,
            AppEvent::Chrome(ChromeIntent::OpenCommandPalette),
        );

        assert!(!app.modes.is_menu_open());
        assert!(!app.palette.is_open());
    }

    /// The palette is a menu, so a press on a header button closes it
    /// first.
    #[test]
    fn a_panel_button_while_the_palette_is_open_only_closes_it() {
        let presses = [
            ChromeIntent::ToggleLeftPanel,
            ChromeIntent::ToggleFilesPanel,
            ChromeIntent::ToggleFocusMode,
        ];

        for press in presses {
            let mut app = app_with_palette_query("");
            let panels = app.sidebar.vm();

            let _ = update_input(&mut app, AppEvent::Chrome(press));

            let after = app.sidebar.vm();
            assert!(!app.palette.is_open());
            assert_eq!(after.is_left_open, panels.is_left_open);
            assert_eq!(after.is_right_open, panels.is_right_open);
            assert_eq!(after.is_focus_mode, panels.is_focus_mode);
        }
    }

    #[test]
    fn the_search_button_while_the_palette_is_open_closes_it() {
        let mut app = app_with_palette_query("spl");

        let _ = update_input(
            &mut app,
            AppEvent::Chrome(ChromeIntent::OpenCommandPalette),
        );

        assert!(!app.palette.is_open());
    }

    /// The gear under an open menu, the palette and the list too, closes
    /// it and goes on: Settings opens in the same update.
    #[test]
    fn the_settings_button_under_an_open_menu_closes_it_and_opens_settings() {
        for mut app in [
            app_with_palette_query(""),
            app_with_open_notification_list(),
            app_with_open_modes_menu(),
            app_with_open_header_menu(),
        ] {
            let opened = top_overlay(&app);

            let _ = update_input(
                &mut app,
                AppEvent::Chrome(ChromeIntent::OpenSettings),
            );

            assert!(app.settings.is_open(), "{opened:?}");
            assert!(!app.palette.is_open(), "{opened:?}");
            assert!(!app.notifications.is_list_open(), "{opened:?}");
            assert!(!app.modes.is_menu_open(), "{opened:?}");
            assert!(!app.sidebar.has_header_menu_open(), "{opened:?}");
            assert_eq!(top_overlay(&app), Some(Overlay::Settings));
        }
    }

    /// Settings is a dialog over every mode: its shortcut opens it in
    /// Containers mode, which stays.
    #[test]
    fn the_settings_shortcut_in_containers_opens_the_dialog_and_keeps_the_mode()
    {
        let mut app = app();
        let _ = app.modes.reduce(ModesIntent::Select(Mode::Containers), ());

        press(&mut app, AppShortcut::Settings);

        assert!(app.settings.is_open());
        assert_eq!(app.modes.active(), Mode::Containers);
        assert_eq!(top_overlay(&app), Some(Overlay::Settings));
    }

    /// Esc means Cancel: Settings closes and drops the edits that were not
    /// saved.
    #[test]
    fn esc_closes_settings_and_drops_the_edits() {
        let mut app = app();
        open_settings(&mut app);
        edit_the_shell(&mut app, "/bin/edited");
        let edited = app.settings.has_unsaved_edits();

        let _ = update_input(&mut app, escape());

        assert!(edited);
        assert!(!app.settings.is_open());
        assert!(!app.settings.has_unsaved_edits());
        assert_eq!(top_overlay(&app), None);
    }

    /// Esc cancels Settings and leaves the menu under it open.
    #[test]
    fn esc_closes_settings_over_a_menu_and_keeps_the_menu() {
        let mut app = app_with_open_modes_menu();
        open_settings(&mut app);

        let _ = update_input(&mut app, escape());

        assert!(!app.settings.is_open());
        assert_eq!(top_overlay(&app), Some(Overlay::ModesMenu));
    }

    /// Owner, Q3 of the old app: the gear and the Settings shortcut while
    /// Settings is open do nothing, so the edits stay and the file is not
    /// read again.
    #[test]
    fn the_gear_and_the_shortcut_while_settings_is_open_keep_the_edits() {
        let opens = [
            AppEvent::Chrome(ChromeIntent::OpenSettings),
            AppEvent::Keyboard(shortcut_press(AppShortcut::Settings)),
        ];

        for open in opens {
            let mut app = app();
            open_settings(&mut app);
            edit_the_shell(&mut app, "/bin/edited");

            let task = update_input(&mut app, open);

            assert_eq!(task.units(), 0);
            assert!(app.settings.is_open());
            assert!(app.settings.has_unsaved_edits());
        }
    }

    #[test]
    fn a_shortcut_does_nothing_while_settings_is_open() {
        let mut app = app();
        open_settings(&mut app);

        press(&mut app, AppShortcut::FocusMode);

        assert!(app.settings.is_open());
        assert!(!app.sidebar.vm().is_focus_mode);
    }

    /// The bell, like any app button, does nothing while a dialog waits for
    /// its answer.
    #[test]
    fn the_bell_does_nothing_while_settings_is_open() {
        let mut app = app();
        show_toast(&mut app, "disk full");
        open_settings(&mut app);

        let _ = update_input(
            &mut app,
            AppEvent::Chrome(ChromeIntent::OpenNotifications),
        );

        assert!(!app.notifications.is_list_open());
        assert_eq!(app.notifications.unseen_count(), 1);
    }

    /// The title bar search and the palette shortcut do nothing under the
    /// dialog either.
    #[test]
    fn the_palette_does_not_open_while_settings_is_open() {
        let mut app = app();
        open_settings(&mut app);

        let _ = update_input(
            &mut app,
            AppEvent::Chrome(ChromeIntent::OpenCommandPalette),
        );
        press(&mut app, AppShortcut::CommandPalette);

        assert!(!app.palette.is_open());
        assert_eq!(top_overlay(&app), Some(Overlay::Settings));
    }

    /// A toast's cross lies under the dialog's dimmed area; were a press
    /// to reach it, it would do nothing until the dialog is answered.
    #[test]
    fn a_toast_cross_does_nothing_while_settings_is_open() {
        let mut app = app();
        show_toast(&mut app, "disk full");
        open_settings(&mut app);

        let _ = update_input(
            &mut app,
            AppEvent::Notifications(NotificationsIntent::DismissToast(0)),
        );

        let mut ui = iced_test::simulator(crate::view::view(&app));
        assert!(ui.find("disk full").is_ok());
    }

    /// The dialog's own answers pass: an edit, Save and the close cross.
    #[test]
    fn settings_takes_its_own_answers() {
        let mut app = app();
        open_settings(&mut app);

        let _ = update_input(
            &mut app,
            AppEvent::Settings(SettingsIntent::EditorChanged(String::from(
                "vim",
            ))),
        );
        let edited = app.settings.has_unsaved_edits();
        let save =
            update_input(&mut app, AppEvent::Settings(SettingsIntent::Save));
        let _ =
            update_input(&mut app, AppEvent::Settings(SettingsIntent::Close));

        assert!(edited);
        assert_eq!(save.units(), 1);
        assert!(!app.settings.is_open());
    }

    /// A hover over a Files row passes under the dialog.
    #[test]
    fn hovering_a_files_row_passes_under_settings() {
        let mut app = app();
        open_settings(&mut app);

        let _ = update_input(
            &mut app,
            AppEvent::Explorer(ExplorerIntent::NodeHovered {
                path: Some(vec![String::from("src")]),
            }),
        );

        assert!(app.settings.is_open());
    }

    /// A window button works under the dialog and leaves it open.
    #[test]
    fn a_window_button_leaves_settings_open() {
        let mut app = app();
        open_settings(&mut app);

        let _task = update_input(
            &mut app,
            AppEvent::Chrome(ChromeIntent::MinimizeWindow),
        );

        assert!(app.settings.is_open());
    }

    #[test]
    fn a_window_button_leaves_the_palette_open() {
        let presses = [
            ChromeIntent::MinimizeWindow,
            ChromeIntent::ToggleMaximize,
            ChromeIntent::PressTitleBar,
            ChromeIntent::MoveOnTitleBar,
            ChromeIntent::ReleaseTitleBar,
        ];

        for press in presses {
            let mut app = app_with_palette_query("spl");

            let _task = update_input(&mut app, AppEvent::Chrome(press));

            assert_eq!(app.palette.query(), Some("spl"));
        }
    }

    #[test]
    fn a_shortcut_while_the_palette_is_open_closes_it_and_runs() {
        let mut app = app_with_palette_query("spl");

        press(&mut app, AppShortcut::FocusMode);

        assert!(!app.palette.is_open());
        assert!(app.sidebar.vm().is_focus_mode);
    }

    /// Owner (Q1 of the old app): the palette shortcut while the palette is
    /// open keeps it and what was typed in it, and gives its input the
    /// keyboard again.
    #[test]
    fn the_palette_shortcut_while_the_palette_is_open_keeps_it_and_its_query() {
        let mut app = app_with_palette_query("spl");

        let task = update_input(
            &mut app,
            AppEvent::Keyboard(shortcut_press(AppShortcut::CommandPalette)),
        );

        assert_eq!(app.palette.query(), Some("spl"));
        assert!(focuses_widget(task, &input_id()));
    }

    #[test]
    fn typing_in_the_palette_keeps_it_open() {
        let app = app_with_palette_query("spl");

        assert_eq!(app.palette.query(), Some("spl"));
    }

    /// A picked entry runs its command in the same update, after the
    /// palette passed the checks as the menu on top.
    #[test]
    fn a_picked_palette_entry_runs_its_command() {
        let mut app = app_with_palette_query(otty_app_i18n::t(
            otty_app_i18n::Key::WindowFocusMode,
        ));

        let _ = update_input(
            &mut app,
            AppEvent::Palette(PaletteIntent::Action(PaletteAction::RunActive)),
        );

        assert!(!app.palette.is_open());
        assert!(app.sidebar.vm().is_focus_mode);
    }

    /// The Close session shortcut closes the active tab once one is open.
    #[test]
    fn the_close_session_shortcut_closes_the_active_tab() {
        let mut app = app();
        open_terminal_tab(&mut app);

        let _ = update_input(
            &mut app,
            AppEvent::Keyboard(shortcut_press(AppShortcut::CloseSession)),
        );

        assert_eq!(app.tabs.len(), 0);
    }

    #[test]
    fn a_tab_press_under_an_open_menu_closes_it_and_switches_the_tab() {
        let mut app = app();
        open_terminal_tab(&mut app);
        let first = app.tabs.active_tab_id().expect("a tab is open");
        open_terminal_tab(&mut app);
        let _ =
            update_input(&mut app, AppEvent::Modes(ModesIntent::ToggleMenu));

        let _ = update_input(
            &mut app,
            AppEvent::Tabs(TabsIntent::ActivateTab { tab_id: first }),
        );

        assert!(!app.modes.is_menu_open());
        assert_eq!(app.tabs.active_tab_id(), Some(first));
    }

    /// A tab that comes to the front in another mode shows Terminal mode,
    /// and the All modes menu closes as the mode switches (owner, Q2 of the
    /// old app).
    #[test]
    fn a_launch_editor_opened_in_containers_mode_shows_terminal_mode() {
        let mut app = app_with_open_modes_menu();
        let _ = app.modes.reduce(ModesIntent::Select(Mode::Containers), ());
        let _ = app.modes.reduce(ModesIntent::ToggleMenu, ());

        let _ = update_internal(
            &mut app,
            AppEvent::Tabs(TabsIntent::OpenWizardTab {
                title: String::from("New launch"),
                init: WizardTabInit::Create {
                    parent_path: Vec::new(),
                },
            }),
        );

        assert_eq!(app.tabs.len(), 1);
        assert_eq!(app.modes.active(), Mode::Terminal);
        assert!(!app.modes.is_menu_open());
    }

    /// New terminal in the New session menu closes the menu and opens the
    /// tab.
    #[test]
    fn new_terminal_in_the_header_menu_closes_it_and_opens_a_tab() {
        let mut app = app_with_open_header_menu();

        let _ =
            update_input(&mut app, AppEvent::Tabs(TabsIntent::OpenTerminalTab));

        assert!(!app.sidebar.has_header_menu_open());
        assert_eq!(app.tabs.len(), 1);
    }

    /// A tab opened while the All modes menu is open leaves the menu open:
    /// it is no press of the user.
    #[test]
    fn a_tab_opened_under_an_open_menu_keeps_the_menu() {
        let mut app = app_with_open_modes_menu();

        let _ = update_internal(
            &mut app,
            AppEvent::Tabs(TabsIntent::OpenTerminalTab),
        );

        assert_eq!(app.tabs.len(), 1);
        assert_eq!(app.terminal.pane_count(), Some(1));
        assert!(app.modes.is_menu_open());
    }

    /// A tab closed while the All modes menu is open ends its terminal and
    /// leaves the menu open.
    #[test]
    fn a_tab_closed_under_an_open_menu_lets_go_of_its_terminal() {
        let mut app = app();
        let tab_id = open_shell_tab(&mut app);
        let _ =
            update_input(&mut app, AppEvent::Modes(ModesIntent::ToggleMenu));

        let _ = update_internal(
            &mut app,
            AppEvent::Tabs(TabsIntent::CloseTab { tab_id }),
        );

        assert_eq!(app.tabs.len(), 0);
        assert_eq!(app.terminal.pane_count(), None);
        assert!(app.modes.is_menu_open());
    }

    /// What a focused terminal reports for `shortcut`. Terminal 999 is in
    /// no tab: the app turns the report into a command before the terminal
    /// could look it up.
    fn terminal_binding(shortcut: AppShortcut) -> AppEvent {
        AppEvent::Terminal(TerminalIntent::Widget(
            otty_ui_term::Event::BindingActionDispatched {
                id: 999,
                action: shortcut,
            },
        ))
    }

    /// Both events one press of `shortcut` can arrive as: a key from the
    /// keyboard subscription and the report of a focused terminal.
    fn both_key_paths(shortcut: AppShortcut) -> [AppEvent; 2] {
        [
            AppEvent::Keyboard(shortcut_press(shortcut)),
            terminal_binding(shortcut),
        ]
    }

    #[test]
    fn a_terminal_binding_becomes_its_command() {
        let event = as_command(terminal_binding(AppShortcut::PinnedMode2));

        assert!(matches!(
            event,
            AppEvent::Command(AppCommand::SelectModeAt { position: 2 })
        ));
    }

    #[test]
    fn a_terminal_binding_runs_its_command() {
        let mut app = app();

        let _ = update_input(&mut app, terminal_binding(AppShortcut::Settings));

        assert!(app.settings.is_open());
    }

    /// From the keyboard subscription no terminal had keyboard focus; the
    /// focused pane of the active tab is split all the same, once, in the
    /// same update.
    #[test]
    fn the_split_shortcuts_split_the_focused_pane_once_on_both_key_paths() {
        for shortcut in [AppShortcut::SplitRight, AppShortcut::SplitDown] {
            for press in both_key_paths(shortcut) {
                let mut app = app();
                let _ = open_shell_tab(&mut app);

                let _ = update_input(&mut app, press);

                assert_eq!(app.terminal.pane_count(), Some(2), "{shortcut:?}");
            }
        }
    }

    #[test]
    fn close_session_and_the_splits_do_nothing_in_containers_mode() {
        // The shell tab stays open, hidden by Containers mode.
        for shortcut in [
            AppShortcut::CloseSession,
            AppShortcut::SplitRight,
            AppShortcut::SplitDown,
        ] {
            for press in both_key_paths(shortcut) {
                let mut app = app();
                let _ = open_shell_tab(&mut app);
                let _ =
                    app.modes.reduce(ModesIntent::Select(Mode::Containers), ());

                let _ = update_input(&mut app, press);

                assert_eq!(app.tabs.len(), 1, "{shortcut:?}");
                assert_eq!(app.terminal.pane_count(), Some(1), "{shortcut:?}");
            }
        }
    }

    #[test]
    fn the_split_shortcuts_do_nothing_on_a_launch_editor_tab() {
        for shortcut in [AppShortcut::SplitRight, AppShortcut::SplitDown] {
            for press in both_key_paths(shortcut) {
                let mut app = app();
                let _ = open_shell_tab(&mut app);
                let _ = update_internal(
                    &mut app,
                    AppEvent::Tabs(TabsIntent::OpenWizardTab {
                        title: String::from("New launch"),
                        init: WizardTabInit::Create {
                            parent_path: Vec::new(),
                        },
                    }),
                );

                let _ = update_input(&mut app, press);

                assert_eq!(app.terminal.pane_count(), None, "{shortcut:?}");
                assert_eq!(app.tabs.len(), 2, "{shortcut:?}");
            }
        }
    }

    #[test]
    fn a_shortcut_under_an_open_menu_closes_it_and_runs_on_both_key_paths() {
        for press in both_key_paths(AppShortcut::FocusMode) {
            let mut app = app_with_open_header_menu();

            let _ = update_input(&mut app, press);

            assert!(!app.sidebar.has_header_menu_open());
            assert!(app.sidebar.vm().is_focus_mode);
        }
    }

    #[test]
    fn a_mode_binding_from_a_terminal_switches_while_the_modes_menu_is_open() {
        let mut app = app_with_open_modes_menu();

        let _ =
            update_input(&mut app, terminal_binding(AppShortcut::PinnedMode1));

        assert_eq!(app.modes.active(), Mode::Containers);
        assert!(!app.modes.is_menu_open());
    }

    /// Text typed into a terminal that still had keyboard focus under an
    /// open menu or dialog never reaches its shell.
    #[test]
    fn typed_terminal_input_under_an_overlay_is_dropped() {
        let typed = || {
            AppEvent::Terminal(TerminalIntent::Widget(
                otty_ui_term::Event::Write {
                    id: 0,
                    data: b"a".to_vec(),
                },
            ))
        };
        for mut app in [app_with_open_pane_menu(), {
            let mut app = app();
            let _ = open_shell_tab(&mut app);
            open_settings(&mut app);
            app
        }] {
            let task = update_input(&mut app, typed());

            assert_eq!(task.units(), 0);
        }
    }

    /// The Split item of the New session menu closes the menu and splits
    /// the focused pane.
    #[test]
    fn split_in_the_header_menu_closes_it_and_splits_the_pane() {
        let mut app = app();
        let tab_id = open_shell_tab(&mut app);
        let (_, pane) = app.terminal.focused_pane().expect("the pane");
        let _ = update_input(
            &mut app,
            AppEvent::Sidebar(SidebarIntent::HeaderMenuOpen(
                HeaderMenu::Sessions,
            )),
        );

        let _ = update_input(
            &mut app,
            AppEvent::Terminal(TerminalIntent::SplitPane {
                tab_id,
                pane,
                axis: iced::widget::pane_grid::Axis::Vertical,
            }),
        );

        assert!(!app.sidebar.has_header_menu_open());
        assert_eq!(app.terminal.pane_count(), Some(2));
    }

    /// A press on a pane under the pane menu does nothing: the menu stays.
    #[test]
    fn a_pane_press_under_the_pane_menu_keeps_it() {
        let mut app = app_with_open_pane_menu();
        let (tab_id, pane) = app.terminal.focused_pane().expect("the pane");

        let _ = update_input(
            &mut app,
            AppEvent::Terminal(TerminalIntent::PaneClicked { tab_id, pane }),
        );

        assert!(app.terminal.has_context_menu());
    }

    /// A shortcut under the pane menu closes it and runs.
    #[test]
    fn a_shortcut_under_the_pane_menu_closes_it_and_runs() {
        let mut app = app_with_open_pane_menu();

        press(&mut app, AppShortcut::FocusMode);

        assert!(!app.terminal.has_context_menu());
        assert!(app.sidebar.vm().is_focus_mode);
    }

    fn quick_launch_intent(intent: QuickLaunchIntent) -> AppEvent {
        AppEvent::QuickLaunch(intent)
    }

    /// Scenario 13 of the old app: a palette dismiss or a mode switch asks
    /// for the active pane while the SSH prompt is open. The typed secret
    /// must stay in the prompt, so the pane does not take the keyboard.
    #[test]
    fn focus_active_under_the_secret_prompt_moves_no_focus() {
        let mut app = app();
        open_terminal_tab(&mut app);
        let focus_active = || AppEvent::Terminal(TerminalIntent::FocusActive);
        let focused_without_the_prompt =
            runs_widget_operation(update_internal(&mut app, focus_active()));
        open_secret_prompt(&mut app);

        let task = update_internal(&mut app, focus_active());

        assert!(focused_without_the_prompt);
        assert!(!runs_widget_operation(task));
    }

    /// Scenario 13 of the old app: a shortcut under the SSH prompt does
    /// nothing, so no terminal behind the prompt takes the typed secret.
    #[test]
    fn a_shortcut_does_nothing_while_the_secret_prompt_is_open() {
        let mut app = app();
        open_secret_prompt(&mut app);

        press(&mut app, AppShortcut::FocusMode);

        assert!(app.quick_launch.has_secret_prompt());
        assert!(!app.sidebar.vm().is_focus_mode);
    }

    #[test]
    fn a_panel_button_does_nothing_while_the_secret_prompt_is_open() {
        let mut app = app();
        open_secret_prompt(&mut app);
        let panels = app.sidebar.vm();

        let _ = update_input(
            &mut app,
            AppEvent::Chrome(ChromeIntent::ToggleLeftPanel),
        );

        assert!(app.quick_launch.has_secret_prompt());
        assert_eq!(app.sidebar.vm().is_left_open, panels.is_left_open);
    }

    /// Scenario 14 of the old app: an SSH prompt that comes up over an
    /// open menu takes what the user types, and the menu stays under it.
    #[test]
    fn the_secret_prompt_takes_typing_under_an_open_menu() {
        let mut app = app_with_open_header_menu();
        open_secret_prompt(&mut app);

        let typed = update_input(
            &mut app,
            quick_launch_intent(QuickLaunchIntent::SecretPromptChanged(
                String::from("pw"),
            )),
        );
        let _check = update_input(
            &mut app,
            quick_launch_intent(QuickLaunchIntent::SecretPromptSubmit),
        );

        // The typed password was taken: Connect closes the prompt and runs
        // the launch again, which is checked in the background.
        assert!(collect_outputs(typed).is_empty());
        assert!(!app.quick_launch.has_secret_prompt());
        assert!(app.quick_launch.tick_timer().is_some());
        assert!(app.sidebar.has_header_menu_open());
    }

    #[test]
    fn the_menu_under_a_cancelled_secret_prompt_is_still_open() {
        let mut app = app_with_open_header_menu();
        open_secret_prompt(&mut app);

        let _ = update_input(
            &mut app,
            quick_launch_intent(QuickLaunchIntent::SecretPromptCancel),
        );

        assert!(!app.quick_launch.has_secret_prompt());
        assert_eq!(top_overlay(&app), Some(Overlay::HeaderMenu));
    }

    #[test]
    fn an_app_shortcut_during_an_inline_edit_cancels_it() {
        let mut app = app();
        start_inline_edit(&mut app);

        press(&mut app, AppShortcut::FilesPanel);

        assert!(!app.quick_launch.has_inline_edit());
    }

    /// A shortcut that can not run now still cancels the edit, as any
    /// other shortcut does.
    #[test]
    fn a_shortcut_that_can_not_run_during_an_inline_edit_still_cancels_it() {
        let mut app = app();
        start_inline_edit(&mut app);

        press(&mut app, AppShortcut::CloseSession);

        assert!(!app.quick_launch.has_inline_edit());
    }

    #[test]
    fn a_plain_key_during_an_inline_edit_keeps_it() {
        let mut app = app();
        start_inline_edit(&mut app);

        let _ = update_input(
            &mut app,
            AppEvent::Keyboard(key_press(
                iced::keyboard::key::Physical::Code(
                    iced::keyboard::key::Code::KeyA,
                ),
                "a",
                iced::keyboard::Modifiers::empty(),
            )),
        );

        assert!(app.quick_launch.has_inline_edit());
    }

    #[test]
    fn hovering_a_files_row_or_moving_over_the_title_bar_keeps_an_inline_edit()
    {
        let mut app = app();
        start_inline_edit(&mut app);

        let _ = update_input(
            &mut app,
            AppEvent::Explorer(ExplorerIntent::NodeHovered {
                path: Some(vec![String::from("src")]),
            }),
        );
        let _ = update_input(
            &mut app,
            AppEvent::Chrome(ChromeIntent::MoveOnTitleBar),
        );

        assert!(app.quick_launch.has_inline_edit());
    }

    /// What follows from input that already passed the checks, such as an
    /// I/O answer, leaves the edit alone.
    #[test]
    fn an_internal_event_leaves_an_inline_edit_alone() {
        let mut app = app();
        start_inline_edit(&mut app);

        let _ = update_internal(&mut app, AppEvent::HostNameRead(None));
        let _ = update_internal(
            &mut app,
            AppEvent::Command(AppCommand::ToggleFilesPanel),
        );

        assert!(app.quick_launch.has_inline_edit());
    }

    /// Typing in the name field and Enter pass the inline edit check and
    /// create the folder.
    #[test]
    fn typing_in_the_name_field_and_enter_create_the_folder() {
        let mut app = app();
        start_inline_edit(&mut app);

        let _ = update_input(
            &mut app,
            quick_launch_intent(QuickLaunchIntent::InlineEditChanged(
                String::from("servers"),
            )),
        );
        let _ = update_input(
            &mut app,
            quick_launch_intent(QuickLaunchIntent::InlineEditSubmit),
        );

        // The new folder waits for the next tick to be saved.
        assert!(!app.quick_launch.has_inline_edit());
        assert!(app.quick_launch.tick_timer().is_some());
    }

    /// A shortcut under the context menu of the launches closes it and
    /// runs, as under any menu.
    #[test]
    fn a_shortcut_under_the_launch_menu_closes_it_and_runs() {
        let mut app = app_with_open_launch_menu();

        press(&mut app, AppShortcut::FocusMode);

        assert!(!app.quick_launch.has_context_menu());
        assert!(app.sidebar.vm().is_focus_mode);
    }

    // The containers feature follows the right panel.

    #[test]
    fn closing_and_opening_the_right_panel_tells_the_containers_feature() {
        let mut app = app();
        assert!(app.sidebar.vm().is_right_open);
        assert!(!app.containers.is_right_panel_hidden());

        let _ = update_input(
            &mut app,
            AppEvent::Sidebar(SidebarIntent::ToggleRight),
        );
        let after_close = app.containers.is_right_panel_hidden();
        let _ = update_input(
            &mut app,
            AppEvent::Sidebar(SidebarIntent::ToggleRight),
        );

        assert!(after_close);
        assert!(!app.containers.is_right_panel_hidden());
    }

    #[test]
    fn focus_mode_hides_the_right_panel_from_the_containers_feature() {
        let mut app = app();

        let _ = update_internal(
            &mut app,
            AppEvent::Sidebar(SidebarIntent::ToggleFocusMode),
        );

        assert!(app.containers.is_right_panel_hidden());
    }

    #[test]
    fn an_update_that_leaves_the_panel_alone_tells_containers_nothing() {
        let mut app = app();

        let task = update_internal(
            &mut app,
            AppEvent::HostNameRead(Some(String::from("build-box"))),
        );

        assert!(!app.containers.is_right_panel_hidden());
        assert_eq!(task.units(), 0);
    }
}
