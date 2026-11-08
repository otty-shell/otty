//! Key presses in view tests, delivered the way the running app gets them.
//!
//! iced gives a key press to the widgets of the window first. The app
//! keyboard subscription (`iced::keyboard::listen` in `app.rs`) gets the
//! key only if no widget captured it. The tests build no subscription, so
//! [`input_from_focused_terminal`] hands the key to the app itself, on the
//! same condition.

use iced::advanced::widget::operation;
use iced::event::Status;
use iced::keyboard::key::{NativeCode, Physical};
use iced::keyboard::{self, Key, Location};
use iced_test::runtime::user_interface::{Cache, UserInterface};

use crate::app::view::view;
use crate::app::{App, Message};
use crate::router::AppEvent;
use crate::shortcuts::{AppShortcut, Platform};
use crate::testing::task_outputs::collect_outputs;
use crate::testing::window::headless_renderer;

/// The press of `shortcut`'s keys on the current platform, as the keyboard
/// reports it.
pub(crate) fn shortcut_press(shortcut: AppShortcut) -> keyboard::Event {
    let keys = shortcut.keys(Platform::current());
    let physical_key = keys
        .code()
        .map(Physical::Code)
        .unwrap_or(Physical::Unidentified(NativeCode::Unidentified));

    keyboard::Event::KeyPressed {
        key: Key::Character(keys.key().into()),
        modified_key: Key::Character(keys.key().into()),
        physical_key,
        location: Location::Standard,
        modifiers: keys.modifiers(),
        text: None,
        repeat: false,
    }
}

/// Return the input the app gets when `key` is pressed in its window while
/// the focused pane of the active tab has the keyboard focus: what the
/// widgets sent, then the key itself if no widget captured it.
pub(crate) fn input_from_focused_terminal(
    app: &App,
    key: &keyboard::Event,
) -> Vec<AppEvent> {
    let terminal = focused_terminal(app);
    let mut renderer = headless_renderer();
    let mut ui = UserInterface::build(
        view(app),
        app.window_size,
        Cache::default(),
        &mut renderer,
    );
    ui.operate(&renderer, &mut operation::focusable::focus(terminal));

    let mut input = Vec::new();
    let (_, statuses) = ui.update(
        &[iced::Event::Keyboard(key.clone())],
        iced::mouse::Cursor::Unavailable,
        &mut renderer,
        &mut iced::advanced::clipboard::Null,
        &mut input,
    );

    // The running app also gets the key from `iced::keyboard::listen`, which
    // delivers only the keys no widget captured. The test runs no
    // subscription, so it hands the key over itself, on the same condition.
    if statuses.first() == Some(&Status::Ignored) {
        input.push(AppEvent::Keyboard(key.clone()));
    }

    input
}

/// Press `key` in the focused terminal of `app` (see
/// [`input_from_focused_terminal`]) and run in `app` what the press leads
/// to, as the iced runtime does.
pub(crate) fn press_in_focused_terminal(app: &mut App, key: &keyboard::Event) {
    let input = input_from_focused_terminal(app, key);

    for event in input {
        let follow_ups = collect_outputs(app.update(Message::Input(event)));

        for follow_up in follow_ups {
            let _ = app.update(follow_up);
        }
    }
}

/// The widget id of the focused pane of the active tab.
fn focused_terminal(app: &App) -> iced::widget::Id {
    let tab_id = app.widgets.tabs.active_tab_id().expect("an active tab");

    app.widgets
        .terminal_workspace
        .tab(tab_id)
        .and_then(|tab| tab.focused_terminal_entry())
        .map(|entry| entry.terminal().widget_id().clone())
        .expect("the active tab should have a focused pane")
}
