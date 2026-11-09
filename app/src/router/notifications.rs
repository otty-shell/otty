//! Routes the notifications widget: reads the clock for its reducer and
//! moves the keyboard focus when the list opens or closes. Other routes
//! tell the user about an error with [`push`].

use std::time::Instant;

use iced::Task;

use super::AppEvent;
use crate::app::App;
use crate::focus;
use crate::widgets::notifications::NotificationsCtx;
use crate::widgets::notifications::event::{
    NotificationsEffect, NotificationsEvent, NotificationsIntent,
};
use crate::widgets::terminal_workspace::event::{
    TerminalWorkspaceEvent, TerminalWorkspaceIntent,
};

/// Reduce the intents of the notifications widget and run its effects.
pub(crate) fn route(
    app: &mut App,
    event: NotificationsEvent,
) -> Task<AppEvent> {
    match event {
        NotificationsEvent::Intent(intent) => {
            let ctx = NotificationsCtx {
                now: Instant::now(),
            };

            app.widgets
                .notifications
                .reduce(intent, &ctx)
                .map(AppEvent::Notifications)
        },
        NotificationsEvent::Effect(effect) => handle_effect(effect),
    }
}

/// Tell the user about an error: a toast at the top of the window. `text`
/// is already in the user's language, such as one built by
/// `i18n::with_error`.
///
/// # Example
///
/// ```ignore
/// Err(err) => notifications::push(i18n::with_error(
///     Key::TplPinnedModesSaveFailed,
///     &format!("{err}"),
/// )),
/// ```
pub(super) fn push(text: String) -> Task<AppEvent> {
    Task::done(AppEvent::Notifications(NotificationsEvent::Intent(
        NotificationsIntent::Push(text),
    )))
}

fn handle_effect(effect: NotificationsEffect) -> Task<AppEvent> {
    match effect {
        // The open list takes the keyboard from the terminal, so typed keys
        // do not reach the shell and Esc reaches the app, which closes it.
        NotificationsEffect::ListOpened => focus::nothing(),
        // In Terminal mode the active pane gets the keyboard back; in
        // Containers mode no pane is on screen to take it.
        NotificationsEffect::ListClosed => Task::done(
            AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
                TerminalWorkspaceIntent::FocusActive,
            )),
        ),
    }
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;

    use super::*;
    use crate::app::view::view;
    use crate::testing::task_outputs::{collect_outputs, focuses_nothing};

    #[test]
    fn push_sends_the_text_to_the_notifications_widget() {
        let events = collect_outputs(push(String::from("disk full")));

        assert!(matches!(
            events.as_slice(),
            [AppEvent::Notifications(NotificationsEvent::Intent(
                NotificationsIntent::Push(text)
            ))] if text == "disk full"
        ));
    }

    /// The open list takes the keyboard from the terminal, as any menu
    /// does, so typed keys reach no shell and Esc reaches the app.
    #[test]
    fn an_opened_list_takes_the_keyboard_focus() {
        let (mut app, _) = App::new();

        let task = route(
            &mut app,
            NotificationsEvent::Effect(NotificationsEffect::ListOpened),
        );

        assert!(focuses_nothing(task));
    }

    #[test]
    fn a_closed_list_gives_the_active_pane_its_keys_back() {
        let (mut app, _) = App::new();

        let events = collect_outputs(route(
            &mut app,
            NotificationsEvent::Effect(NotificationsEffect::ListClosed),
        ));

        assert!(matches!(
            events.as_slice(),
            [AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
                TerminalWorkspaceIntent::FocusActive
            ))]
        ));
    }

    #[test]
    fn a_pushed_error_shows_in_the_window_as_a_toast() {
        let (mut app, _) = App::new();

        for event in collect_outputs(push(String::from("disk full"))) {
            let _ = crate::router::route(&mut app, event);
        }

        let mut ui = simulator(view(&app));
        assert!(ui.find("disk full").is_ok());
    }
}
