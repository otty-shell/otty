//! What input from the user does while a menu is open, which overlay lies on
//! top of the window, and what decides whether a command can run now. The
//! input checks only see input: what follows from it, and what arrives on
//! its own, skips them (see `update_internal` in `update.rs`), so it can not
//! get lost.
//!
//! The overlays today are the header menus, the All modes menu, the command
//! palette and the notifications list. Each feature issue adds its own:
//! Settings (09), the context menus and the inline edit of the terminal and
//! quick launch (11, 12), the SSH secret prompt (12) and the Containers
//! dialogs and row menu (14). The dialogs bring the dialog check of
//! `update.rs` with them.

use otty_app_domain::commands::CommandCtx;
use otty_app_domain::mode::Mode;
use otty_app_explorer::ExplorerIntent;
use otty_app_notifications::NotificationsIntent;
use otty_app_palette::PaletteIntent;
use otty_app_slots::OverlayKind;

use crate::app::App;
use crate::chrome::ChromeIntent;
use crate::message::AppEvent;
use crate::modes::ModesIntent;
use crate::sidebar::SidebarIntent;

/// An overlay: a menu that lies over the window and takes the user's input
/// before the window under it. [`top_overlay`] finds the one on top.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Overlay {
    /// The notifications list under the bell, over every mode.
    NotificationsList,
    /// The command palette, over every mode.
    Palette,
    /// The All modes menu.
    ModesMenu,
    /// The New session or Launches menu of a section header.
    HeaderMenu,
}

impl Overlay {
    /// Return whether the overlay is a menu or a dialog. A click outside a
    /// menu closes it, and a shortcut closes it and runs.
    pub(crate) fn kind(self) -> OverlayKind {
        match self {
            Overlay::NotificationsList
            | Overlay::Palette
            | Overlay::ModesMenu
            | Overlay::HeaderMenu => OverlayKind::Menu,
        }
    }

    /// Return what Esc sends while this overlay is on top: the event that
    /// closes it.
    pub(crate) fn escape_event(self) -> AppEvent {
        match self {
            Overlay::NotificationsList => {
                AppEvent::Notifications(NotificationsIntent::CloseList)
            },
            Overlay::Palette => AppEvent::Palette(PaletteIntent::Dismiss),
            Overlay::ModesMenu => AppEvent::Modes(ModesIntent::CloseMenu),
            Overlay::HeaderMenu => {
                AppEvent::Sidebar(SidebarIntent::HeaderMenuDismiss)
            },
        }
    }
}

/// Determines how the event loop should treat input from the user when a
/// menu is on top.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MenuGuard {
    /// Let the event pass through to normal dispatch.
    Allow,
    /// Silently drop the event without closing the menu.
    Ignore,
    /// Close all menus; the event itself does nothing else.
    Dismiss,
    /// Close all menus, then dispatch the event.
    DismissAndRun,
}

/// Classify input from the user when a menu is on top.
pub(crate) fn context_menu_guard(event: &AppEvent) -> MenuGuard {
    use MenuGuard::*;

    match event {
        AppEvent::Sidebar(intent) => match intent {
            SidebarIntent::HeaderMenuDismiss
            | SidebarIntent::WorkspaceCursorMoved { .. } => Allow,
            SidebarIntent::HeaderMenuOpen(_) => Ignore,
            _ => Dismiss,
        },
        // The first press on a panel button, the palette button or the bell
        // only closes the menu; opening settings closes it and goes on.
        AppEvent::Chrome(intent) => match intent {
            ChromeIntent::ToggleLeftPanel
            | ChromeIntent::ToggleFilesPanel
            | ChromeIntent::ToggleFocusMode
            | ChromeIntent::OpenCommandPalette
            | ChromeIntent::OpenNotifications => Dismiss,
            ChromeIntent::OpenSettings => DismissAndRun,
            _ => Allow,
        },
        // The All modes menu is one of the menus: its rows, pin toggles and
        // dismiss layer send these. Choosing a mode closes every menu.
        AppEvent::Modes(ModesIntent::Select(_)) => DismissAndRun,
        AppEvent::Modes(_) => Allow,
        // The palette is one of the menus: typing in it, its entries and its
        // dismiss layer send these.
        AppEvent::Palette(_) => Allow,
        // The pointer moving over the Files tree leaves the menus open; a
        // press on a row closes them.
        AppEvent::Explorer(ExplorerIntent::NodeHovered { .. }) => Allow,
        // The broom, the row crosses and the dismiss layer of the
        // notifications list. The cross of a toast under a menu is a press
        // outside it, so it only closes the menus.
        AppEvent::Notifications(
            NotificationsIntent::Clear
            | NotificationsIntent::Remove(_)
            | NotificationsIntent::CloseList,
        ) => Allow,
        AppEvent::ResizeWindow(_) => Allow,
        // A shortcut closes the menus and runs. Other keys no widget took do
        // nothing under a menu.
        AppEvent::Command(_) => DismissAndRun,
        AppEvent::Keyboard(_) => Ignore,
        _ => Dismiss,
    }
}

/// Return the overlay on top of the window, if one is open.
///
/// The order is fixed, top first: the SSH prompt and Settings (their
/// feature issues add them here), the notifications list, the palette, the
/// All modes menu, the header menus, then the overlays of the active mode
/// (added by the terminal, quick launch and containers issues). Those of
/// the other mode are not on screen, so they do not count.
pub(crate) fn top_overlay(app: &App) -> Option<Overlay> {
    if app.notifications.is_list_open() {
        return Some(Overlay::NotificationsList);
    }
    if app.palette.is_open() {
        return Some(Overlay::Palette);
    }
    if app.modes.is_menu_open() {
        return Some(Overlay::ModesMenu);
    }
    if app.sidebar.has_header_menu_open() {
        return Some(Overlay::HeaderMenu);
    }

    None
}

/// Gather what decides whether a command can run now: the mode on screen,
/// the active tab and its focused pane. The input filter checks shortcuts
/// with it, and the palette lists and runs only the commands it allows (see
/// `AppCommand::is_available`).
pub(crate) fn command_ctx(app: &App) -> CommandCtx {
    CommandCtx {
        is_terminal_mode: app.modes.active() == Mode::Terminal,
        // No tab is open until the tabs feature (issue 10) reports one.
        has_active_tab: false,
        // No pane has focus until the terminal feature (issue 11) reports
        // one.
        has_focused_pane: false,
    }
}

/// Return `true` when the event is a press of the Esc key that no widget
/// took. The frame closes the overlay on top with it. A held key repeats its
/// press; the repeats do not count, so one press closes one overlay.
pub(crate) fn is_escape_press(event: &AppEvent) -> bool {
    use iced::keyboard::key::Named;
    use iced::keyboard::{Event, Key};

    matches!(
        event,
        AppEvent::Keyboard(Event::KeyPressed {
            key: Key::Named(Named::Escape),
            repeat: false,
            ..
        })
    )
}

#[cfg(test)]
mod tests {
    use iced::keyboard::key::{Code, Named, NativeCode, Physical};
    use iced::keyboard::{Event, Key, Location, Modifiers};
    use otty_app_domain::commands::AppCommand;
    use otty_app_explorer::ExplorerIntent;
    use otty_app_slots::Feature;
    use otty_ui_kit::components::command_palette::PaletteAction;

    use super::*;
    use crate::sidebar::types::HeaderMenu;
    use crate::testing::keyboard::{escape_press, key_press};
    use crate::testing::window::{app, open_notification_list, open_palette};

    /// Opens one overlay on an app.
    type Opener = fn(&mut App);

    fn open_modes_menu(app: &mut App) {
        let _ = app.modes.reduce(ModesIntent::ToggleMenu, ());
    }

    fn open_header_menu(app: &mut App) {
        let _ = app
            .sidebar
            .reduce(SidebarIntent::HeaderMenuOpen(HeaderMenu::Sessions), ());
    }

    /// The openers of the overlays, with the overlay each one opens.
    const OVERLAYS: [(Opener, Overlay); 4] = [
        (open_notification_list, Overlay::NotificationsList),
        (open_palette, Overlay::Palette),
        (open_modes_menu, Overlay::ModesMenu),
        (open_header_menu, Overlay::HeaderMenu),
    ];

    #[test]
    fn each_overlay_of_the_frame_is_a_menu() {
        for (_, overlay) in OVERLAYS {
            assert_eq!(overlay.kind(), OverlayKind::Menu, "{overlay:?}");
        }
    }

    #[test]
    fn nothing_is_on_top_of_a_plain_window() {
        let app = app();

        assert_eq!(top_overlay(&app), None);
    }

    #[test]
    fn each_open_overlay_is_found() {
        for (open, overlay) in OVERLAYS {
            let mut app = app();
            open(&mut app);

            assert_eq!(top_overlay(&app), Some(overlay));
        }
    }

    #[test]
    fn the_modes_menu_lies_over_a_header_menu() {
        let mut app = app();
        open_header_menu(&mut app);
        open_modes_menu(&mut app);

        assert_eq!(top_overlay(&app), Some(Overlay::ModesMenu));
    }

    /// The list lies over the All modes menu and the header menus, and
    /// counts in Containers mode too.
    #[test]
    fn the_notification_list_lies_over_the_other_menus() {
        let mut app = app();
        let _ = app.modes.reduce(ModesIntent::Select(Mode::Containers), ());
        open_header_menu(&mut app);
        open_modes_menu(&mut app);

        open_notification_list(&mut app);

        assert_eq!(top_overlay(&app), Some(Overlay::NotificationsList));
    }

    /// The palette lies over the All modes menu and the header menus, and
    /// under the notifications list, in either mode.
    #[test]
    fn the_palette_lies_between_the_menus_and_the_notification_list() {
        for mode in [Mode::Terminal, Mode::Containers] {
            let mut app = app();
            let _ = app.modes.reduce(ModesIntent::Select(mode), ());
            open_header_menu(&mut app);
            open_modes_menu(&mut app);
            open_palette(&mut app);
            let over_the_menus = top_overlay(&app);

            open_notification_list(&mut app);

            assert_eq!(over_the_menus, Some(Overlay::Palette), "{mode:?}");
            assert_eq!(
                top_overlay(&app),
                Some(Overlay::NotificationsList),
                "{mode:?}"
            );
        }
    }

    /// The modes menu belongs to both modes.
    #[test]
    fn the_modes_menu_is_found_in_containers_mode() {
        let mut app = app();
        let _ = app.modes.reduce(ModesIntent::Select(Mode::Containers), ());
        open_modes_menu(&mut app);

        assert_eq!(top_overlay(&app), Some(Overlay::ModesMenu));
    }

    #[test]
    fn the_command_context_reads_the_mode() {
        let mut app = app();
        let in_terminal = command_ctx(&app);
        let _ = app.modes.reduce(ModesIntent::Select(Mode::Containers), ());
        let in_containers = command_ctx(&app);

        assert_eq!(
            in_terminal,
            CommandCtx {
                is_terminal_mode: true,
                has_active_tab: false,
                has_focused_pane: false,
            }
        );
        assert!(!in_containers.is_terminal_mode);
    }

    #[test]
    fn a_pressed_escape_key_is_an_escape_press() {
        assert!(is_escape_press(&AppEvent::Keyboard(escape_press())));
    }

    /// A held Esc closes one overlay, not the next one under it too.
    #[test]
    fn a_held_esc_is_not_an_escape_press() {
        let held = AppEvent::Keyboard(Event::KeyPressed {
            key: Key::Named(Named::Escape),
            modified_key: Key::Named(Named::Escape),
            physical_key: Physical::Code(Code::Escape),
            location: Location::Standard,
            modifiers: Modifiers::empty(),
            text: None,
            repeat: true,
        });

        assert!(!is_escape_press(&held));
    }

    #[test]
    fn other_keys_and_a_released_escape_are_not_an_escape_press() {
        let letter = AppEvent::Keyboard(key_press(
            Physical::Unidentified(NativeCode::Unidentified),
            "a",
            Modifiers::empty(),
        ));
        let released = AppEvent::Keyboard(Event::KeyReleased {
            key: Key::Named(Named::Escape),
            modified_key: Key::Named(Named::Escape),
            physical_key: Physical::Code(Code::Escape),
            location: Location::Standard,
            modifiers: Modifiers::empty(),
        });

        assert!(!is_escape_press(&letter));
        assert!(!is_escape_press(&released));
        assert!(!is_escape_press(&AppEvent::HostNameRead(None)));
    }

    #[test]
    fn esc_sends_the_close_intent_of_each_overlay() {
        assert!(matches!(
            Overlay::ModesMenu.escape_event(),
            AppEvent::Modes(ModesIntent::CloseMenu)
        ));
        assert!(matches!(
            Overlay::HeaderMenu.escape_event(),
            AppEvent::Sidebar(SidebarIntent::HeaderMenuDismiss)
        ));
        assert!(matches!(
            Overlay::NotificationsList.escape_event(),
            AppEvent::Notifications(NotificationsIntent::CloseList)
        ));
        assert!(matches!(
            Overlay::Palette.escape_event(),
            AppEvent::Palette(PaletteIntent::Dismiss)
        ));
    }

    /// Typing, choosing an entry and the palette's own dismiss layer go to
    /// the open palette.
    #[test]
    fn the_palette_s_own_input_passes() {
        let intents = [
            PaletteIntent::Action(PaletteAction::QueryChanged(String::from(
                "spl",
            ))),
            PaletteIntent::Action(PaletteAction::MoveDown),
            PaletteIntent::Action(PaletteAction::RunActive),
            PaletteIntent::FocusInput,
            PaletteIntent::Dismiss,
        ];

        for intent in intents {
            let event = AppEvent::Palette(intent);

            assert_eq!(context_menu_guard(&event), MenuGuard::Allow);
        }
    }

    /// The broom, the cross of a row and the list's own dismiss layer go to
    /// the open list.
    #[test]
    fn the_notification_list_s_own_input_passes() {
        for intent in [
            NotificationsIntent::Clear,
            NotificationsIntent::Remove(1),
            NotificationsIntent::CloseList,
        ] {
            let event = AppEvent::Notifications(intent);

            assert_eq!(context_menu_guard(&event), MenuGuard::Allow);
        }
    }

    /// The cross of a toast under a menu is a click outside the menu: it
    /// only closes the menu.
    #[test]
    fn a_toast_cross_under_a_menu_only_closes_the_menus() {
        let event =
            AppEvent::Notifications(NotificationsIntent::DismissToast(1));

        assert_eq!(context_menu_guard(&event), MenuGuard::Dismiss);
    }

    #[test]
    fn given_menu_open_when_modes_events_arrive_then_they_are_allowed() {
        let events = [
            AppEvent::Modes(ModesIntent::CloseMenu),
            AppEvent::Modes(ModesIntent::Pin(Mode::Containers)),
        ];

        for event in events {
            assert_eq!(context_menu_guard(&event), MenuGuard::Allow);
        }
    }

    #[test]
    fn choosing_a_mode_or_opening_settings_closes_the_menus_and_goes_on() {
        let events = [
            AppEvent::Modes(ModesIntent::Select(Mode::Containers)),
            AppEvent::Chrome(ChromeIntent::OpenSettings),
        ];

        for event in events {
            assert_eq!(context_menu_guard(&event), MenuGuard::DismissAndRun);
        }
    }

    #[test]
    fn a_panel_or_palette_button_only_closes_the_menus() {
        let presses = [
            ChromeIntent::ToggleLeftPanel,
            ChromeIntent::ToggleFilesPanel,
            ChromeIntent::ToggleFocusMode,
            ChromeIntent::OpenCommandPalette,
            ChromeIntent::OpenNotifications,
        ];

        for press in presses {
            let event = AppEvent::Chrome(press);

            assert_eq!(context_menu_guard(&event), MenuGuard::Dismiss);
        }
    }

    #[test]
    fn a_window_button_leaves_the_menus_open() {
        for press in [
            ChromeIntent::MinimizeWindow,
            ChromeIntent::ToggleMaximize,
            ChromeIntent::CloseWindow,
        ] {
            let event = AppEvent::Chrome(press);

            assert_eq!(context_menu_guard(&event), MenuGuard::Allow);
        }
    }

    #[test]
    fn moving_over_the_title_bar_keeps_the_menus() {
        let event = AppEvent::Chrome(ChromeIntent::MoveOnTitleBar);

        assert_eq!(context_menu_guard(&event), MenuGuard::Allow);
    }

    #[test]
    fn given_menu_open_when_menu_open_intent_arrives_then_it_is_ignored() {
        let guard = context_menu_guard(&AppEvent::Sidebar(
            SidebarIntent::HeaderMenuOpen(HeaderMenu::Sessions),
        ));

        assert_eq!(guard, MenuGuard::Ignore);
    }

    #[test]
    fn given_menu_open_when_menu_dismiss_intent_arrives_then_it_is_allowed() {
        let guard = context_menu_guard(&AppEvent::Sidebar(
            SidebarIntent::HeaderMenuDismiss,
        ));

        assert_eq!(guard, MenuGuard::Allow);
    }

    #[test]
    fn the_right_panel_toggle_closes_the_menus() {
        let toggle = AppEvent::Sidebar(SidebarIntent::ToggleRight);

        assert_eq!(context_menu_guard(&toggle), MenuGuard::Dismiss);
    }

    #[test]
    fn hovering_a_files_row_keeps_the_menus() {
        // A row sends the path when the pointer enters it, and no path when
        // the pointer leaves it or the whole tree.
        let paths = [Some(vec![String::from("src")]), None];

        for path in paths {
            let event =
                AppEvent::Explorer(ExplorerIntent::NodeHovered { path });

            assert_eq!(context_menu_guard(&event), MenuGuard::Allow);
        }
    }

    #[test]
    fn pressing_a_files_row_still_closes_the_menus() {
        let event = AppEvent::Explorer(ExplorerIntent::NodePressed {
            path: vec![String::from("src")],
        });

        assert_eq!(context_menu_guard(&event), MenuGuard::Dismiss);
    }

    #[test]
    fn a_window_grip_leaves_the_menus_open() {
        let grip = AppEvent::ResizeWindow(iced::window::Direction::East);

        assert_eq!(context_menu_guard(&grip), MenuGuard::Allow);
    }

    #[test]
    fn a_command_under_an_open_menu_closes_the_menus_and_goes_on() {
        let event = AppEvent::Command(AppCommand::ToggleFocusMode);

        assert_eq!(context_menu_guard(&event), MenuGuard::DismissAndRun);
    }

    #[test]
    fn a_key_that_is_not_a_shortcut_does_nothing_under_a_menu() {
        let event = AppEvent::Keyboard(key_press(
            Physical::Code(Code::KeyA),
            "a",
            Modifiers::empty(),
        ));

        assert_eq!(context_menu_guard(&event), MenuGuard::Ignore);
    }
}
