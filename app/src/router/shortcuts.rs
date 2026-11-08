//! Runs app shortcuts, whichever route the key press took.
//!
//! A press reaches here either from a terminal binding (the terminal had
//! keyboard focus) or from the app keyboard subscription (no widget
//! captured the key). Only one of the two ever sees a given press; see
//! [`crate::shortcuts`].

use iced::{Task, keyboard};

use super::AppEvent;
use super::tabs::close_active_tab;
use crate::app::App;
use crate::shortcuts::{AppShortcut, Platform, shortcut_for_key_press};
use crate::widgets::modes::{ModesEvent, ModesIntent};
use crate::widgets::palette::{PaletteEvent, PaletteIntent};
use crate::widgets::sidebar::{SidebarEvent, SidebarIntent};
use crate::widgets::tabs::{TabsEvent, TabsIntent};

/// Run the app shortcut a key press from the keyboard subscription stands
/// for, if any.
pub(crate) fn handle_key_event(
    app: &App,
    event: &keyboard::Event,
) -> Task<AppEvent> {
    match shortcut_for_key_press(event, Platform::current()) {
        Some(shortcut) => handle(app, shortcut),
        None => Task::none(),
    }
}

/// Run `shortcut`.
pub(crate) fn handle(app: &App, shortcut: AppShortcut) -> Task<AppEvent> {
    match shortcut {
        AppShortcut::Settings => tabs_intent(TabsIntent::OpenSettingsTab),
        AppShortcut::NewTerminal => tabs_intent(TabsIntent::OpenTerminalTab),
        AppShortcut::CloseSession => close_active_tab(app),
        // Split pane acts on the pane that has keyboard focus, so only the
        // terminal binding runs it (in the terminal workspace reducer). With
        // no terminal focused there is no pane to split.
        AppShortcut::SplitPane => Task::none(),
        AppShortcut::LeftPanel => sidebar_intent(SidebarIntent::ToggleLeft),
        AppShortcut::FilesPanel => sidebar_intent(SidebarIntent::ToggleRight),
        AppShortcut::FocusMode => {
            sidebar_intent(SidebarIntent::ToggleFocusMode)
        },
        AppShortcut::CommandPalette => Task::done(AppEvent::Palette(
            PaletteEvent::Intent(PaletteIntent::Open),
        )),
        AppShortcut::TerminalMode
        | AppShortcut::PinnedMode1
        | AppShortcut::PinnedMode2
        | AppShortcut::PinnedMode3
        | AppShortcut::PinnedMode4 => select_mode(shortcut),
        AppShortcut::AllModesMenu => modes_intent(ModesIntent::ToggleMenu),
    }
}

/// Switch to the mode at the switcher position of a mode shortcut; the
/// modes widget knows which mode sits there.
fn select_mode(shortcut: AppShortcut) -> Task<AppEvent> {
    match shortcut.mode_position() {
        Some(position) => modes_intent(ModesIntent::SelectAt { position }),
        None => Task::none(),
    }
}

fn tabs_intent(intent: TabsIntent) -> Task<AppEvent> {
    Task::done(AppEvent::Tabs(TabsEvent::Intent(intent)))
}

fn sidebar_intent(intent: SidebarIntent) -> Task<AppEvent> {
    Task::done(AppEvent::Sidebar(SidebarEvent::Intent(intent)))
}

fn modes_intent(intent: ModesIntent) -> Task<AppEvent> {
    Task::done(AppEvent::Modes(ModesEvent::Intent(intent)))
}

#[cfg(test)]
mod tests {
    use iced::keyboard::key::{NativeCode, Physical};
    use iced::keyboard::{Event, Key, Location, Modifiers};

    use super::*;
    use crate::domain::mode::Mode;
    use crate::testing::task_outputs::collect_outputs;
    use crate::widgets::modes::ModesWidget;
    use crate::widgets::tabs::TabsCtx;

    /// Context the tests reduce tabs intents with.
    const SH: TabsCtx<'static> = TabsCtx { shell_name: "sh" };

    fn new_terminal_press(repeat: bool) -> Event {
        let keys = AppShortcut::NewTerminal.keys(Platform::current());

        Event::KeyPressed {
            key: Key::Character(keys.key().into()),
            modified_key: Key::Character(keys.key().into()),
            physical_key: Physical::Unidentified(NativeCode::Unidentified),
            location: Location::Standard,
            modifiers: keys.modifiers(),
            text: None,
            repeat,
        }
    }

    /// App on first start: Terminal active, Containers pinned, without reading
    /// `modes.json`.
    fn first_start_app() -> App {
        let (mut app, _) = App::new();
        app.widgets.modes = ModesWidget::new();
        app
    }

    /// The press of the physical key a mode shortcut has on this platform,
    /// reporting `character` (Shift may change it).
    fn mode_key_press(shortcut: AppShortcut, character: &str) -> Event {
        let keys = shortcut.keys(Platform::current());
        let code = keys.code().expect("mode digits are physical keys");

        Event::KeyPressed {
            key: Key::Character(character.into()),
            modified_key: Key::Character(character.into()),
            physical_key: Physical::Code(code),
            location: Location::Standard,
            modifiers: keys.modifiers(),
            text: None,
            repeat: false,
        }
    }

    fn selected_position(events: &[AppEvent]) -> Option<usize> {
        match events {
            [
                AppEvent::Modes(ModesEvent::Intent(ModesIntent::SelectAt {
                    position,
                })),
            ] => Some(*position),
            _ => None,
        }
    }

    #[test]
    fn terminal_mode_shortcut_asks_for_switcher_position_zero() {
        let app = first_start_app();

        let events = collect_outputs(handle(&app, AppShortcut::TerminalMode));

        assert_eq!(selected_position(&events), Some(0));
    }

    #[test]
    fn pinned_mode_1_shortcut_asks_for_switcher_position_one() {
        let app = first_start_app();

        let events = collect_outputs(handle(&app, AppShortcut::PinnedMode1));

        assert_eq!(selected_position(&events), Some(1));
    }

    #[test]
    fn pinned_mode_shortcuts_ask_for_their_switcher_positions() {
        // Which mode sits there, if any, is the modes widget's to say.
        let app = first_start_app();

        for (shortcut, position) in [
            (AppShortcut::PinnedMode2, 2),
            (AppShortcut::PinnedMode3, 3),
            (AppShortcut::PinnedMode4, 4),
        ] {
            let events = collect_outputs(handle(&app, shortcut));

            assert_eq!(
                selected_position(&events),
                Some(position),
                "{shortcut:?}"
            );
        }
    }

    #[test]
    fn all_modes_menu_shortcut_toggles_the_menu() {
        let app = first_start_app();

        let events = collect_outputs(handle(&app, AppShortcut::AllModesMenu));

        assert!(matches!(
            events.as_slice(),
            [AppEvent::Modes(ModesEvent::Intent(ModesIntent::ToggleMenu))]
        ));
    }

    #[test]
    fn mode_key_press_from_the_subscription_switches_mode() {
        let app = first_start_app();
        // With Shift held the US layout reports "@" for the 2 key.
        let event = mode_key_press(AppShortcut::PinnedMode1, "@");

        let events = collect_outputs(handle_key_event(&app, &event));

        assert_eq!(selected_position(&events), Some(1));
    }

    #[test]
    fn settings_shortcut_opens_the_settings_tab() {
        let (app, _) = App::new();

        let events = collect_outputs(handle(&app, AppShortcut::Settings));

        assert!(matches!(
            events.as_slice(),
            [AppEvent::Tabs(TabsEvent::Intent(
                TabsIntent::OpenSettingsTab
            ))]
        ));
    }

    #[test]
    fn new_terminal_shortcut_opens_a_shell_tab() {
        let (app, _) = App::new();

        let events = collect_outputs(handle(&app, AppShortcut::NewTerminal));

        assert!(matches!(
            events.as_slice(),
            [AppEvent::Tabs(TabsEvent::Intent(
                TabsIntent::OpenTerminalTab
            ))]
        ));
    }

    #[test]
    fn close_session_shortcut_closes_the_active_tab() {
        let (app, _) = App::new();

        let events = collect_outputs(handle(&app, AppShortcut::CloseSession));

        assert!(matches!(
            events.as_slice(),
            [AppEvent::Tabs(TabsEvent::Intent(TabsIntent::CloseActive))]
        ));
    }

    #[test]
    fn close_session_shortcut_outside_terminal_mode_keeps_the_hidden_tab() {
        let mut app = first_start_app();
        let _ = app.widgets.tabs.reduce(TabsIntent::OpenSettingsTab, &SH);
        let _ = app
            .widgets
            .modes
            .reduce(ModesIntent::Select(Mode::Containers));

        let events = collect_outputs(handle(&app, AppShortcut::CloseSession));

        assert!(events.is_empty());
    }

    #[test]
    fn left_panel_shortcut_toggles_the_left_panel() {
        let (app, _) = App::new();

        let events = collect_outputs(handle(&app, AppShortcut::LeftPanel));

        assert!(matches!(
            events.as_slice(),
            [AppEvent::Sidebar(SidebarEvent::Intent(
                SidebarIntent::ToggleLeft
            ))]
        ));
    }

    #[test]
    fn files_panel_shortcut_toggles_the_right_panel() {
        let (app, _) = App::new();

        let events = collect_outputs(handle(&app, AppShortcut::FilesPanel));

        assert!(matches!(
            events.as_slice(),
            [AppEvent::Sidebar(SidebarEvent::Intent(
                SidebarIntent::ToggleRight
            ))]
        ));
    }

    #[test]
    fn focus_mode_shortcut_toggles_focus_mode() {
        let (app, _) = App::new();

        let events = collect_outputs(handle(&app, AppShortcut::FocusMode));

        assert!(matches!(
            events.as_slice(),
            [AppEvent::Sidebar(SidebarEvent::Intent(
                SidebarIntent::ToggleFocusMode
            ))]
        ));
    }

    #[test]
    fn command_palette_shortcut_opens_the_palette() {
        let (app, _) = App::new();

        let events = collect_outputs(handle(&app, AppShortcut::CommandPalette));

        assert!(matches!(
            events.as_slice(),
            [AppEvent::Palette(PaletteEvent::Intent(PaletteIntent::Open))]
        ));
    }

    #[test]
    fn split_pane_from_the_subscription_does_nothing() {
        let (app, _) = App::new();

        let events = collect_outputs(handle(&app, AppShortcut::SplitPane));

        assert!(events.is_empty());
    }

    #[test]
    fn key_press_from_the_subscription_runs_its_shortcut() {
        let (app, _) = App::new();

        let events =
            collect_outputs(handle_key_event(&app, &new_terminal_press(false)));

        assert!(matches!(
            events.as_slice(),
            [AppEvent::Tabs(TabsEvent::Intent(
                TabsIntent::OpenTerminalTab
            ))]
        ));
    }

    #[test]
    fn repeated_key_press_from_the_subscription_does_nothing() {
        let (app, _) = App::new();

        let events =
            collect_outputs(handle_key_event(&app, &new_terminal_press(true)));

        assert!(events.is_empty());
    }

    #[test]
    fn other_key_press_from_the_subscription_does_nothing() {
        let (app, _) = App::new();
        let event = Event::KeyPressed {
            key: Key::Character("b".into()),
            modified_key: Key::Character("b".into()),
            physical_key: Physical::Unidentified(NativeCode::Unidentified),
            location: Location::Standard,
            modifiers: Modifiers::CTRL,
            text: None,
            repeat: false,
        };

        let events = collect_outputs(handle_key_event(&app, &event));

        assert!(events.is_empty());
    }
}
