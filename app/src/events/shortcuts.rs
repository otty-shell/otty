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
    }
}

fn tabs_intent(intent: TabsIntent) -> Task<AppEvent> {
    Task::done(AppEvent::Tabs(TabsEvent::Intent(intent)))
}

fn sidebar_intent(intent: SidebarIntent) -> Task<AppEvent> {
    Task::done(AppEvent::Sidebar(SidebarEvent::Intent(intent)))
}

#[cfg(test)]
mod tests {
    use iced::keyboard::key::{NativeCode, Physical};
    use iced::keyboard::{Event, Key, Location, Modifiers};

    use super::*;
    use crate::testing::task_outputs::collect_outputs;
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
