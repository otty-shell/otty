//! What input from the user does while a menu is open, which overlay lies on
//! top of the window, and what decides whether a command can run now. The
//! input checks only see input: what follows from it, and what arrives on
//! its own, skips them (see `update_internal` in `update.rs`), so it can not
//! get lost.
//!
//! The frame's overlays today are the header menus and the All modes menu.
//! Each feature issue adds its own: the notifications list (06), the
//! palette (08), Settings (09), the context menus and the inline edit of
//! the terminal and quick launch (11, 12), the SSH secret prompt (12) and
//! the Containers dialogs and row menu (14). The dialogs bring the dialog
//! check of `update.rs` with them.

use otty_app_domain::commands::CommandCtx;
use otty_app_domain::mode::Mode;
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
            Overlay::ModesMenu | Overlay::HeaderMenu => OverlayKind::Menu,
        }
    }

    /// Return what Esc sends while this overlay is on top: the event that
    /// closes it.
    pub(crate) fn escape_event(self) -> AppEvent {
        match self {
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
/// The order is fixed, top first: the SSH prompt, Settings, the
/// notifications list and the palette (their feature issues add them here),
/// the All modes menu, the header menus, then the overlays of the active
/// mode (added by the terminal, quick launch and containers issues). Those
/// of the other mode are not on screen, so they do not count.
pub(crate) fn top_overlay(app: &App) -> Option<Overlay> {
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
/// with it (see `AppCommand::is_available`).
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
    use otty_app_slots::Feature;

    use super::*;
    use crate::sidebar::types::HeaderMenu;
    use crate::testing::keyboard::{escape_press, key_press};
    use crate::testing::window::app;

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
    const OVERLAYS: [(Opener, Overlay); 2] = [
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
