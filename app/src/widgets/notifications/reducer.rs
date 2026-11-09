#![forbid(clippy::disallowed_methods)]

use std::time::Instant;

use iced::Task;

use super::event::{NotificationsEvent, NotificationsIntent};
use super::state::NotificationsState;

/// What the notifications reducer reads from outside the widget: the
/// clock, which the route reads, so that the reducer stays plain.
pub(crate) struct NotificationsCtx {
    /// The time the intent is handled at.
    pub(crate) now: Instant,
}

/// Apply an intent to the state and return the effects it asks for.
pub(super) fn reduce(
    state: &mut NotificationsState,
    intent: NotificationsIntent,
    ctx: &NotificationsCtx,
) -> Task<NotificationsEvent> {
    match intent {
        NotificationsIntent::Push(text) => {
            state.push(text, ctx.now);
            Task::none()
        },
        NotificationsIntent::DismissToast(id) => {
            state.dismiss_toast(id);
            Task::none()
        },
        NotificationsIntent::Tick => {
            state.expire_toasts(ctx.now);
            Task::none()
        },
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::testing::task_outputs::collect_outputs;

    fn at(now: Instant) -> NotificationsCtx {
        NotificationsCtx { now }
    }

    fn toasts(state: &NotificationsState) -> Vec<&str> {
        state.toasts().map(|toast| toast.text()).collect()
    }

    #[test]
    fn a_pushed_error_shows_as_a_toast_and_asks_for_nothing() {
        let mut state = NotificationsState::default();

        let events = collect_outputs(reduce(
            &mut state,
            NotificationsIntent::Push(String::from("disk full")),
            &at(Instant::now()),
        ));

        assert!(events.is_empty());
        assert_eq!(toasts(&state), vec!["disk full"]);
    }

    /// The toast counts its five seconds from the time in the context,
    /// which the route reads from the clock.
    #[test]
    fn a_tick_five_seconds_after_the_push_hides_the_toast() {
        let mut state = NotificationsState::default();
        let pushed = Instant::now();
        let _ = reduce(
            &mut state,
            NotificationsIntent::Push(String::from("disk full")),
            &at(pushed),
        );

        let _ = reduce(
            &mut state,
            NotificationsIntent::Tick,
            &at(pushed + Duration::from_secs(4)),
        );
        let after_four = toasts(&state).len();
        let _ = reduce(
            &mut state,
            NotificationsIntent::Tick,
            &at(pushed + Duration::from_secs(5)),
        );

        assert_eq!(after_four, 1);
        assert!(toasts(&state).is_empty());
    }

    #[test]
    fn the_cross_of_a_toast_hides_it() {
        let mut state = NotificationsState::default();
        let now = Instant::now();
        let _ = reduce(
            &mut state,
            NotificationsIntent::Push(String::from("disk full")),
            &at(now),
        );
        let id = state.toasts().map(|toast| toast.id()).next();

        let _ = reduce(
            &mut state,
            NotificationsIntent::DismissToast(id.expect("a toast")),
            &at(now),
        );

        assert!(toasts(&state).is_empty());
    }
}
