//! Runs app shortcuts, whichever route the key press took.
//!
//! A press reaches here either from a terminal binding (the terminal had
//! keyboard focus) or from the app keyboard subscription (no widget
//! captured the key). Only one of the two ever sees a given press; see
//! [`crate::shortcuts`].

use iced::{Task, keyboard};

use super::AppEvent;
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
        AppShortcut::CloseSession => match app.widgets.tabs.active_tab_id() {
            Some(tab_id) => tabs_intent(TabsIntent::CloseTab { tab_id }),
            None => Task::none(),
        },
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
        | AppShortcut::PinnedMode4 => select_mode(app, shortcut),
        AppShortcut::AllModesMenu => modes_intent(ModesIntent::ToggleMenu),
    }
}

/// Switch to the mode at the switcher position of a mode shortcut:
/// Terminal, then the pinned modes. With fewer pinned modes than the
/// position, nothing happens; a temporary tab is never a pinned mode.
fn select_mode(app: &App, shortcut: AppShortcut) -> Task<AppEvent> {
    let pinned_rows = app.widgets.modes.vm().pinned_rows;
    let mode = shortcut
        .mode_position()
        .and_then(|position| pinned_rows.get(position))
        .map(|row| row.mode);

    match mode {
        Some(mode) => modes_intent(ModesIntent::Select(mode)),
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
    use crate::testing::task_outputs::collect_outputs;
    use crate::widgets::modes::ModesWidget;
    use crate::widgets::modes::types::Mode;
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

    /// App on first start: Terminal active, Docker pinned, without reading
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

    fn selected_mode(events: &[AppEvent]) -> Option<Mode> {
        match events {
            [
                AppEvent::Modes(ModesEvent::Intent(ModesIntent::Select(mode))),
            ] => Some(*mode),
            _ => None,
        }
    }

    #[test]
    fn terminal_mode_shortcut_switches_to_terminal() {
        let app = first_start_app();

        let events = collect_outputs(handle(&app, AppShortcut::TerminalMode));

        assert_eq!(selected_mode(&events), Some(Mode::Terminal));
    }

    #[test]
    fn pinned_mode_1_shortcut_switches_to_the_first_pinned_mode() {
        let app = first_start_app();

        let events = collect_outputs(handle(&app, AppShortcut::PinnedMode1));

        assert_eq!(selected_mode(&events), Some(Mode::Docker));
    }

    #[test]
    fn pinned_mode_shortcut_past_the_pinned_modes_does_nothing() {
        let app = first_start_app();

        for shortcut in [
            AppShortcut::PinnedMode2,
            AppShortcut::PinnedMode3,
            AppShortcut::PinnedMode4,
        ] {
            let events = collect_outputs(handle(&app, shortcut));

            assert!(events.is_empty(), "{shortcut:?}");
        }
    }

    #[test]
    fn pinned_mode_shortcut_without_pinned_modes_does_nothing() {
        let mut app = first_start_app();
        let _ = app.widgets.modes.reduce(ModesIntent::Unpin(Mode::Docker));

        let events = collect_outputs(handle(&app, AppShortcut::PinnedMode1));

        assert!(events.is_empty());
    }

    #[test]
    fn pinned_mode_shortcut_skips_the_temporary_tab() {
        // Docker unpinned but active: its tab is temporary, not pinned
        // mode 1.
        let mut app = first_start_app();
        let _ = app.widgets.modes.reduce(ModesIntent::Unpin(Mode::Docker));
        let _ = app.widgets.modes.reduce(ModesIntent::Select(Mode::Docker));

        let events = collect_outputs(handle(&app, AppShortcut::PinnedMode1));

        assert!(events.is_empty());
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

        assert_eq!(selected_mode(&events), Some(Mode::Docker));
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
        let (mut app, _) = App::new();
        let _ = app.widgets.tabs.reduce(TabsIntent::OpenSettingsTab, &SH);
        let active = app
            .widgets
            .tabs
            .active_tab_id()
            .expect("settings tab should be active");

        let events = collect_outputs(handle(&app, AppShortcut::CloseSession));

        assert!(matches!(
            events.as_slice(),
            [AppEvent::Tabs(TabsEvent::Intent(TabsIntent::CloseTab { tab_id }))]
                if *tab_id == active
        ));
    }

    #[test]
    fn close_session_shortcut_without_tabs_does_nothing() {
        let (app, _) = App::new();

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
