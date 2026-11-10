//! What input from the user does while a menu or a dialog is open, which
//! overlay lies on top of the window, and what decides whether a command can
//! run now. The input checks only see input: what follows from it, and what
//! arrives on its own, skips them (see `update_internal` in `update.rs`), so
//! it can not get lost.
//!
//! The overlays today are the header menus, the All modes menu, the command
//! palette, the notifications list and the Settings dialog. Each feature
//! issue adds its own: the context menus and the inline edit of the terminal
//! and quick launch (11, 12), the SSH secret prompt (12) and the Containers
//! dialogs and row menu (14). A new dialog adds its answers to
//! [`dialog_guard`].

use otty_app_domain::commands::CommandCtx;
use otty_app_domain::mode::Mode;
use otty_app_explorer::ExplorerIntent;
use otty_app_notifications::NotificationsIntent;
use otty_app_palette::PaletteIntent;
use otty_app_settings::SettingsIntent;
use otty_app_slots::OverlayKind;

use crate::app::App;
use crate::chrome::ChromeIntent;
use crate::message::AppEvent;
use crate::modes::ModesIntent;
use crate::sidebar::SidebarIntent;

/// An overlay: a menu or a dialog that lies over the window and takes the
/// user's input before the window under it. [`top_overlay`] finds the one on
/// top.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Overlay {
    /// The Settings dialog, over every mode.
    Settings,
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
    /// menu closes it, and a shortcut closes it and runs. A dialog waits for
    /// its answer (see [`dialog_guard`]).
    pub(crate) fn kind(self) -> OverlayKind {
        match self {
            Overlay::Settings => OverlayKind::Dialog,
            Overlay::NotificationsList
            | Overlay::Palette
            | Overlay::ModesMenu
            | Overlay::HeaderMenu => OverlayKind::Menu,
        }
    }

    /// Return what Esc sends while this overlay is on top: the event that
    /// closes a menu, or Cancel of a dialog.
    pub(crate) fn escape_event(self) -> AppEvent {
        match self {
            // Closing Settings drops the edits that were not saved.
            Overlay::Settings => AppEvent::Settings(SettingsIntent::Close),
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
/// menu or a dialog is on top.
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

/// Classify input from the user while `dialog` is on top: it returns only
/// `Allow` or `Ignore`. A dialog waits for its answer, so only its own
/// answers, pointer moves and window controls pass. Shortcuts, app buttons,
/// presses outside it and other keys do nothing, and a menu under the dialog
/// stays open.
///
/// Settings is the one dialog that closes on a press on the dimmed area
/// around it: the press sends its Close, one of its own answers, so it
/// passes here like the close cross.
pub(crate) fn dialog_guard(dialog: Overlay, event: &AppEvent) -> MenuGuard {
    if is_dialog_answer(dialog, event) || passes_every_dialog(event) {
        return MenuGuard::Allow;
    }

    MenuGuard::Ignore
}

/// Return the overlay on top of the window, if one is open.
///
/// The order is fixed, top first: the SSH prompt (issue 12 adds it here),
/// Settings, the notifications list, the palette, the All modes menu, the
/// header menus, then the overlays of the active mode (added by the
/// terminal, quick launch and containers issues). Those of the other mode
/// are not on screen, so they do not count.
pub(crate) fn top_overlay(app: &App) -> Option<Overlay> {
    if app.settings.is_open() {
        return Some(Overlay::Settings);
    }
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

/// Whether `event` answers `dialog`: typing in it, its buttons and choices,
/// its close cross, Esc in its fields and, for Settings, a press on the
/// dimmed area around it.
fn is_dialog_answer(dialog: Overlay, event: &AppEvent) -> bool {
    match dialog {
        Overlay::Settings => matches!(
            event,
            AppEvent::Settings(
                SettingsIntent::ThemeSelected(_)
                    | SettingsIntent::LanguageChanged(_)
                    | SettingsIntent::ShellChanged(_)
                    | SettingsIntent::EditorChanged(_)
                    | SettingsIntent::EqualizePanesToggled(_)
                    | SettingsIntent::Save
                    | SettingsIntent::Reset
                    | SettingsIntent::Close
            )
        ),
        _ => false,
    }
}

/// Whether `event` passes under any dialog: a pointer move or a window
/// control. A hover over a Files row passes too, so the tree under the
/// dialog keeps its hover. The terminal feature (issue 11) adds the reports
/// of a terminal and the quick launch feature (issue 12) the pointer moves
/// over the Launches tree.
fn passes_every_dialog(event: &AppEvent) -> bool {
    matches!(
        event,
        AppEvent::Sidebar(SidebarIntent::WorkspaceCursorMoved { .. })
            | AppEvent::Explorer(ExplorerIntent::NodeHovered { .. })
            | AppEvent::Chrome(
                ChromeIntent::ToggleMaximize
                    | ChromeIntent::MinimizeWindow
                    | ChromeIntent::CloseWindow
                    | ChromeIntent::PressTitleBar
                    | ChromeIntent::MoveOnTitleBar
                    | ChromeIntent::ReleaseTitleBar
            )
            | AppEvent::ResizeWindow(_)
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
    use crate::testing::window::{
        app, open_notification_list, open_palette, open_settings,
    };

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
    const OVERLAYS: [(Opener, Overlay); 5] = [
        (open_settings, Overlay::Settings),
        (open_notification_list, Overlay::NotificationsList),
        (open_palette, Overlay::Palette),
        (open_modes_menu, Overlay::ModesMenu),
        (open_header_menu, Overlay::HeaderMenu),
    ];

    /// The dialogs: they wait for their answer.
    const DIALOGS: [Overlay; 1] = [Overlay::Settings];

    #[test]
    fn each_overlay_is_a_menu_or_a_dialog() {
        for (_, overlay) in OVERLAYS {
            let expected = if DIALOGS.contains(&overlay) {
                OverlayKind::Dialog
            } else {
                OverlayKind::Menu
            };

            assert_eq!(overlay.kind(), expected, "{overlay:?}");
        }
    }

    /// Settings lies over every menu, and counts in Containers mode too.
    #[test]
    fn settings_lies_over_every_menu_in_either_mode() {
        for mode in [Mode::Terminal, Mode::Containers] {
            let mut app = app();
            let _ = app.modes.reduce(ModesIntent::Select(mode), ());
            open_header_menu(&mut app);
            open_modes_menu(&mut app);
            open_palette(&mut app);
            open_notification_list(&mut app);

            open_settings(&mut app);

            assert_eq!(top_overlay(&app), Some(Overlay::Settings), "{mode:?}");
        }
    }

    fn settings(intent: SettingsIntent) -> AppEvent {
        AppEvent::Settings(intent)
    }

    /// Each dialog with what the user answers it: typing, its choices, its
    /// buttons, the close cross and Esc in a field.
    fn dialogs_with_answers() -> [(Overlay, Vec<AppEvent>); 1] {
        use otty_app_i18n::Locale;
        use otty_app_settings::LanguageSetting;
        use otty_app_theme::theme::BuiltinTheme;

        [(
            Overlay::Settings,
            vec![
                settings(SettingsIntent::ThemeSelected(
                    BuiltinTheme::OttyLight,
                )),
                settings(SettingsIntent::LanguageChanged(
                    LanguageSetting::Fixed(Locale::En),
                )),
                settings(SettingsIntent::ShellChanged(String::from(
                    "/bin/zsh",
                ))),
                settings(SettingsIntent::EditorChanged(String::from("nvim"))),
                settings(SettingsIntent::EqualizePanesToggled(false)),
                settings(SettingsIntent::Save),
                settings(SettingsIntent::Reset),
                settings(SettingsIntent::Close),
            ],
        )]
    }

    fn is_allowed(dialog: Overlay, event: &AppEvent) -> bool {
        dialog_guard(dialog, event) == MenuGuard::Allow
    }

    #[test]
    fn a_dialog_lets_its_own_answers_through() {
        for (dialog, answers) in dialogs_with_answers() {
            for answer in &answers {
                assert!(is_allowed(dialog, answer), "{dialog:?}: {answer:?}");
            }
        }
    }

    /// Only the file answers and the open come from elsewhere: they are no
    /// answers of the user, so input never sends them.
    #[test]
    fn settings_ignores_what_the_user_can_not_send_it() {
        let others = [
            settings(SettingsIntent::Open),
            settings(SettingsIntent::ReloadFailed(String::from("no access"))),
            settings(SettingsIntent::SaveFailed(String::from("disk full"))),
        ];

        for event in &others {
            assert!(!is_allowed(Overlay::Settings, event), "{event:?}");
        }
    }

    /// Scenario 8 of the old app: a shortcut, a header button, a press
    /// outside the dialog and a plain key do nothing while a dialog waits
    /// for its answer. So the gear and the Settings shortcut do nothing
    /// while Settings is open (owner, Q3), and the bell, the title bar
    /// search, the palette shortcut and a toast's cross neither.
    #[test]
    fn a_dialog_ignores_shortcuts_app_buttons_and_keys() {
        let events = [
            AppEvent::Command(AppCommand::ToggleFocusMode),
            AppEvent::Command(AppCommand::OpenPalette),
            AppEvent::Command(AppCommand::OpenSettings),
            AppEvent::Command(AppCommand::OpenNotifications),
            AppEvent::Command(AppCommand::SelectModeAt { position: 0 }),
            AppEvent::Chrome(ChromeIntent::ToggleLeftPanel),
            AppEvent::Chrome(ChromeIntent::ToggleFilesPanel),
            AppEvent::Chrome(ChromeIntent::ToggleFocusMode),
            AppEvent::Chrome(ChromeIntent::OpenSettings),
            AppEvent::Chrome(ChromeIntent::OpenCommandPalette),
            AppEvent::Chrome(ChromeIntent::OpenNotifications),
            AppEvent::Notifications(NotificationsIntent::DismissToast(1)),
            AppEvent::Modes(ModesIntent::Select(Mode::Terminal)),
            AppEvent::Modes(ModesIntent::ToggleMenu),
            AppEvent::Sidebar(SidebarIntent::ToggleRight),
            AppEvent::Sidebar(SidebarIntent::HeaderMenuOpen(
                HeaderMenu::Sessions,
            )),
            AppEvent::Palette(PaletteIntent::Open),
            AppEvent::Explorer(ExplorerIntent::NodePressed {
                path: vec![String::from("src")],
            }),
            AppEvent::Keyboard(key_press(
                Physical::Code(Code::KeyA),
                "a",
                Modifiers::empty(),
            )),
        ];

        for (dialog, _) in dialogs_with_answers() {
            for event in &events {
                assert!(!is_allowed(dialog, event), "{dialog:?}: {event:?}");
            }
        }
    }

    #[test]
    fn a_dialog_keeps_window_controls_and_pointer_moves() {
        let events = [
            AppEvent::Chrome(ChromeIntent::ToggleMaximize),
            AppEvent::Chrome(ChromeIntent::MinimizeWindow),
            AppEvent::Chrome(ChromeIntent::CloseWindow),
            AppEvent::Chrome(ChromeIntent::PressTitleBar),
            AppEvent::Chrome(ChromeIntent::MoveOnTitleBar),
            AppEvent::Chrome(ChromeIntent::ReleaseTitleBar),
            AppEvent::ResizeWindow(iced::window::Direction::East),
            AppEvent::Sidebar(SidebarIntent::WorkspaceCursorMoved {
                position: iced::Point::ORIGIN,
            }),
            // A row sends the path when the pointer enters it, and no path
            // when the pointer leaves it.
            AppEvent::Explorer(ExplorerIntent::NodeHovered {
                path: Some(vec![String::from("src")]),
            }),
            AppEvent::Explorer(ExplorerIntent::NodeHovered { path: None }),
        ];

        for (dialog, _) in dialogs_with_answers() {
            for event in &events {
                assert!(is_allowed(dialog, event), "{dialog:?}: {event:?}");
            }
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
            Overlay::Settings.escape_event(),
            AppEvent::Settings(SettingsIntent::Close)
        ));
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
