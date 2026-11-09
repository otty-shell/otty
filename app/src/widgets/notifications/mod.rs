pub(crate) mod event;
mod model;
mod reducer;
mod state;
mod subscription;
pub(crate) mod view;

use event::{NotificationsEvent, NotificationsIntent};
use iced::{Subscription, Task};
use model::ToastStackViewModel;
pub(crate) use reducer::NotificationsCtx;
use state::NotificationsState;

/// Notifications widget: tells the user about errors with toasts at the
/// top of the window.
pub(crate) struct NotificationsWidget {
    state: NotificationsState,
}

impl NotificationsWidget {
    /// Create the widget with no notification.
    pub(crate) fn new() -> Self {
        Self {
            state: NotificationsState::default(),
        }
    }

    /// Apply an intent to the state and return the effects it asks for.
    pub(crate) fn reduce(
        &mut self,
        intent: NotificationsIntent,
        ctx: &NotificationsCtx,
    ) -> Task<NotificationsEvent> {
        reducer::reduce(&mut self.state, intent, ctx)
    }

    /// Return the toast timer, which runs only while a toast shows.
    pub(crate) fn subscription(&self) -> Subscription<NotificationsEvent> {
        subscription::subscription(self.state.has_toasts())
    }

    /// Return what the toast stack shows.
    pub(crate) fn toast_stack_vm(&self) -> ToastStackViewModel {
        ToastStackViewModel::from_state(&self.state)
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::*;

    #[test]
    fn without_a_toast_the_timer_does_not_run() {
        let widget = NotificationsWidget::new();

        assert_eq!(widget.subscription().units(), 0);
    }

    #[test]
    fn while_a_toast_shows_the_timer_runs_and_stops_after_it() {
        let mut widget = NotificationsWidget::new();
        let pushed = Instant::now();
        let _ = widget.reduce(
            NotificationsIntent::Push(String::from("disk full")),
            &NotificationsCtx { now: pushed },
        );
        let while_shown = widget.subscription().units();

        let _ = widget.reduce(
            NotificationsIntent::Tick,
            &NotificationsCtx {
                now: pushed + Duration::from_secs(5),
            },
        );

        assert_eq!(while_shown, 1);
        assert_eq!(widget.subscription().units(), 0);
    }
}
