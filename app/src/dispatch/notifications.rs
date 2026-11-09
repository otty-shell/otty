use std::time::Instant;

use iced::Task;

use super::AppEvent;
use crate::app::App;
use crate::widgets::notifications::NotificationsCtx;
use crate::widgets::notifications::event::NotificationsIntent;

/// Deliver a notifications intent to the widget's reducer, with the time
/// read from the clock, so that the reducer stays plain.
pub(crate) fn deliver(
    app: &mut App,
    intent: NotificationsIntent,
) -> Task<AppEvent> {
    let ctx = NotificationsCtx {
        now: Instant::now(),
    };

    app.widgets
        .notifications
        .reduce(intent, &ctx)
        .map(AppEvent::Notifications)
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;

    use crate::app::App;
    use crate::app::view::view;
    use crate::dispatch::dispatch;
    use crate::effects::notifications::push;
    use crate::testing::task_outputs::collect_outputs;

    #[test]
    fn a_pushed_error_shows_in_the_window_as_a_toast() {
        let (mut app, _) = App::new();

        for event in collect_outputs(push(String::from("disk full"))) {
            let _ = dispatch(&mut app, event);
        }

        let mut ui = simulator(view(&app));
        assert!(ui.find("disk full").is_ok());
    }
}
