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
        AppShortcut::NewTerminal => tabs_intent(TabsIntent::OpenTerminalTab {
            title: app.shell_session.name().to_string(),
        }),
        AppShortcut::CloseSession => match app.widgets.tabs.active_tab_id() {
            Some(tab_id) => tabs_intent(TabsIntent::CloseTab { tab_id }),
            None => Task::none(),
        },
        // Split pane acts on the pane that has keyboard focus, so only the
        // terminal binding runs it (in the terminal workspace reducer). With
        // no terminal focused there is no pane to split.
        AppShortcut::SplitPane => Task::none(),
        // Panels and focus mode are wired by ticket 07, the command palette
        // by ticket 17.
        AppShortcut::CommandPalette
        | AppShortcut::LeftPanel
        | AppShortcut::FilesPanel
        | AppShortcut::FocusMode => Task::none(),
    }
}

fn tabs_intent(intent: TabsIntent) -> Task<AppEvent> {
    Task::done(AppEvent::Tabs(TabsEvent::Intent(intent)))
}

#[cfg(test)]
mod tests {
    use iced::keyboard::key::{NativeCode, Physical};
    use iced::keyboard::{Event, Key, Location, Modifiers};

    use super::*;
    use crate::task_outputs::collect_outputs;

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
    fn new_terminal_shortcut_opens_a_shell_tab_named_after_the_shell() {
        let (app, _) = App::new();
        let shell_name = app.shell_session.name().to_string();

        let events = collect_outputs(handle(&app, AppShortcut::NewTerminal));

        assert!(matches!(
            events.as_slice(),
            [AppEvent::Tabs(TabsEvent::Intent(TabsIntent::OpenTerminalTab { title }))]
                if *title == shell_name
        ));
    }

    #[test]
    fn close_session_shortcut_closes_the_active_tab() {
        let (mut app, _) = App::new();
        let _ = app.widgets.tabs.reduce(TabsIntent::OpenSettingsTab);
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
    fn shortcuts_without_an_action_yet_do_nothing() {
        let (app, _) = App::new();

        for shortcut in [
            AppShortcut::CommandPalette,
            AppShortcut::LeftPanel,
            AppShortcut::FilesPanel,
            AppShortcut::FocusMode,
            AppShortcut::SplitPane,
        ] {
            let events = collect_outputs(handle(&app, shortcut));

            assert!(events.is_empty(), "{shortcut:?} must do nothing yet");
        }
    }

    #[test]
    fn key_press_from_the_subscription_runs_its_shortcut() {
        let (app, _) = App::new();

        let events =
            collect_outputs(handle_key_event(&app, &new_terminal_press(false)));

        assert!(matches!(
            events.as_slice(),
            [AppEvent::Tabs(TabsEvent::Intent(
                TabsIntent::OpenTerminalTab { .. }
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
