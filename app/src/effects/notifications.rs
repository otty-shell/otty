//! Runs the effects of the notifications widget: moves the keyboard focus
//! when the list opens or closes. Other effects files tell the user about an
//! error with [`push`].

use iced::Task;

use crate::event::AppEvent;
use crate::focus;
use crate::widgets::notifications::event::{
    NotificationsEffect, NotificationsEvent, NotificationsIntent,
};
use crate::widgets::terminal_workspace::event::{
    TerminalWorkspaceEvent, TerminalWorkspaceIntent,
};

/// Run a notifications effect: move the keyboard focus.
pub(crate) fn run(effect: NotificationsEffect) -> Task<AppEvent> {
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
pub(crate) fn push(text: String) -> Task<AppEvent> {
    Task::done(AppEvent::Notifications(NotificationsEvent::Intent(
        NotificationsIntent::Push(text),
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
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
        let task = run(NotificationsEffect::ListOpened);

        assert!(focuses_nothing(task));
    }

    #[test]
    fn a_closed_list_gives_the_active_pane_its_keys_back() {
        let events = collect_outputs(run(NotificationsEffect::ListClosed));

        assert!(matches!(
            events.as_slice(),
            [AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
                TerminalWorkspaceIntent::FocusActive
            ))]
        ));
    }
}
