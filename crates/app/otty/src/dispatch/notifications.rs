use std::time::Instant;

use iced::Task;
use otty_app_notifications::{
    NotificationsCtx, NotificationsIntent, NotificationsOutcome,
};
use otty_app_slots::Component;
use otty_app_terminal::TerminalIntent;

use super::terminal;
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

    let tasks: Vec<Task<AppEvent>> = outcomes
        .into_iter()
        .map(|outcome| read(app, outcome))
        .collect();

    Task::batch(tasks)
}

fn read(app: &mut App, outcome: NotificationsOutcome) -> Task<AppEvent> {
    match outcome {
        // The open list takes the keyboard from the terminal, so typed keys
        // do not reach the shell and Esc reaches the frame, which closes it.
        NotificationsOutcome::ListOpened => super::take_keyboard(),
        // The terminal decides: in Terminal mode the active pane gets its
        // keys back; in Containers mode nothing takes them.
        NotificationsOutcome::ListClosed => {
            terminal::deliver(app, TerminalIntent::FocusActive)
        },
    }
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;

    use super::*;
    use crate::dispatch;
    use crate::testing::task_outputs::{
        focuses_nothing, runs_widget_operation,
    };
    use crate::testing::window::{app, open_shell_tab};
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

    /// Without a terminal tab no pane takes the keyboard after the list
    /// closes.
    #[test]
    fn a_closed_list_without_a_terminal_tab_moves_no_focus() {
        let mut app = app();
        let _open = deliver(&mut app, NotificationsIntent::OpenList);

        let task = deliver(&mut app, NotificationsIntent::CloseList);

        assert!(!app.notifications.is_list_open());
        assert_eq!(task.units(), 0);
    }

    /// In Terminal mode the active pane gets its keys back when the list
    /// closes; while the list is open, no pane takes them from it.
    #[test]
    fn closing_the_list_gives_the_pane_its_keys_in_terminal_mode() {
        let mut app = app();
        let _tab = open_shell_tab(&mut app);
        let _open = deliver(&mut app, NotificationsIntent::OpenList);
        let under_the_list = runs_widget_operation(dispatch::dispatch(
            &mut app,
            AppEvent::Terminal(TerminalIntent::FocusActive),
        ));

        let task = deliver(&mut app, NotificationsIntent::CloseList);

        assert!(!under_the_list);
        assert!(runs_widget_operation(task));
    }
}
