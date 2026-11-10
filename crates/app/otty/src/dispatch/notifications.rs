use std::time::Instant;

use iced::Task;
use otty_app_notifications::{
    NotificationsCtx, NotificationsIntent, NotificationsOutcome,
};
use otty_app_slots::Feature;

use crate::app::App;
use crate::message::AppEvent;

/// Give a notifications intent to the feature, with the time read from the
/// clock, and read what it reports.
pub(crate) fn deliver(
    app: &mut App,
    intent: NotificationsIntent,
) -> Task<AppEvent> {
    let ctx = NotificationsCtx {
        now: Instant::now(),
    };

    let outcomes = app.notifications.reduce(intent, ctx);

    let tasks: Vec<Task<AppEvent>> = outcomes.into_iter().map(read).collect();

    Task::batch(tasks)
}

fn read(outcome: NotificationsOutcome) -> Task<AppEvent> {
    match outcome {
        // The open list takes the keyboard from the terminal, so typed keys
        // do not reach the shell and Esc reaches the frame, which closes it.
        NotificationsOutcome::ListOpened => super::take_keyboard(),
        // In Terminal mode the terminal feature (issue 11) gives the active
        // pane its keys back; in Containers mode nothing takes them.
        NotificationsOutcome::ListClosed => Task::none(),
    }
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;

    use super::*;
    use crate::testing::task_outputs::focuses_nothing;
    use crate::testing::window::app;
    use crate::view::view;

    #[test]
    fn a_pushed_error_shows_in_the_window_as_a_toast() {
        let mut app = app();

        let _none = deliver(
            &mut app,
            NotificationsIntent::Push(String::from("disk full")),
        );

        let mut ui = simulator(view(&app));
        assert!(ui.find("disk full").is_ok());
        assert_eq!(app.notifications.unseen_count(), 1);
    }

    /// The open list takes the keyboard from the terminal, as any menu
    /// does, so typed keys reach no shell and Esc reaches the frame.
    #[test]
    fn an_opened_list_takes_the_keyboard_focus() {
        let mut app = app();

        let task = deliver(&mut app, NotificationsIntent::OpenList);

        assert!(app.notifications.is_list_open());
        assert!(focuses_nothing(task));
    }

    /// Until the terminal feature moves in, nothing takes the keyboard
    /// after the list closes.
    #[test]
    fn a_closed_list_moves_no_focus_yet() {
        let mut app = app();
        let _open = deliver(&mut app, NotificationsIntent::OpenList);

        let task = deliver(&mut app, NotificationsIntent::CloseList);

        assert!(!app.notifications.is_list_open());
        assert_eq!(task.units(), 0);
    }
}
