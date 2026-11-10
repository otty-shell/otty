use super::event::{
    NotificationsCtx, NotificationsIntent, NotificationsOutcome,
};
use super::state::NotificationsState;

/// Apply an intent to the state and return what happened.
pub(super) fn reduce(
    state: &mut NotificationsState,
    intent: NotificationsIntent,
    ctx: NotificationsCtx,
) -> Vec<NotificationsOutcome> {
    match intent {
        NotificationsIntent::Push(text) => state.push(text, ctx.now),
        NotificationsIntent::DismissToast(id) => state.dismiss_toast(id),
        NotificationsIntent::Tick => state.expire_toasts(ctx.now),
        NotificationsIntent::OpenList => {
            if state.open_list(ctx.now) {
                return vec![NotificationsOutcome::ListOpened];
            }
        },
        NotificationsIntent::CloseList => {
            if state.close_list() {
                return vec![NotificationsOutcome::ListClosed];
            }
        },
        NotificationsIntent::Clear => state.clear(),
        NotificationsIntent::Remove(id) => state.remove(id),
    }

    Vec::new()
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::*;

    fn at(now: Instant) -> NotificationsCtx {
        NotificationsCtx { now }
    }

    fn toasts(state: &NotificationsState) -> Vec<&str> {
        state.toasts().map(|toast| toast.text()).collect()
    }

    #[test]
    fn a_pushed_error_shows_as_a_toast_and_asks_for_nothing() {
        let mut state = NotificationsState::default();

        let outcomes = reduce(
            &mut state,
            NotificationsIntent::Push(String::from("disk full")),
            at(Instant::now()),
        );

        assert!(outcomes.is_empty());
        assert_eq!(toasts(&state), vec!["disk full"]);
    }

    /// The toast counts its five seconds from the time in the context,
    /// which the binary reads from the clock.
    #[test]
    fn a_tick_five_seconds_after_the_push_hides_the_toast() {
        let mut state = NotificationsState::default();
        let pushed = Instant::now();
        let _ = reduce(
            &mut state,
            NotificationsIntent::Push(String::from("disk full")),
            at(pushed),
        );

        let at_four = reduce(
            &mut state,
            NotificationsIntent::Tick,
            at(pushed + Duration::from_secs(4)),
        );
        let after_four = toasts(&state).len();
        let at_five = reduce(
            &mut state,
            NotificationsIntent::Tick,
            at(pushed + Duration::from_secs(5)),
        );

        assert!(at_four.is_empty());
        assert!(at_five.is_empty());
        assert_eq!(after_four, 1);
        assert!(toasts(&state).is_empty());
    }

    #[test]
    fn opening_the_list_hides_the_toasts_and_reports_it_once() {
        let mut state = NotificationsState::default();
        let now = Instant::now();
        let _ = reduce(
            &mut state,
            NotificationsIntent::Push(String::from("disk full")),
            at(now),
        );

        let first = reduce(&mut state, NotificationsIntent::OpenList, at(now));
        let again = reduce(&mut state, NotificationsIntent::OpenList, at(now));

        assert_eq!(first, vec![NotificationsOutcome::ListOpened]);
        assert!(again.is_empty());
        assert!(state.is_list_open());
        assert!(toasts(&state).is_empty());
    }

    /// The list says it closed only when it was open, so the keyboard goes
    /// back to the pane once.
    #[test]
    fn closing_the_list_reports_it_only_when_it_was_open() {
        let mut state = NotificationsState::default();
        let now = Instant::now();
        let while_closed =
            reduce(&mut state, NotificationsIntent::CloseList, at(now));
        let _ = reduce(&mut state, NotificationsIntent::OpenList, at(now));

        let closed =
            reduce(&mut state, NotificationsIntent::CloseList, at(now));

        assert!(while_closed.is_empty());
        assert_eq!(closed, vec![NotificationsOutcome::ListClosed]);
        assert!(!state.is_list_open());
    }

    #[test]
    fn clear_empties_the_open_list() {
        let mut state = NotificationsState::default();
        let now = Instant::now();
        let _ = reduce(
            &mut state,
            NotificationsIntent::Push(String::from("disk full")),
            at(now),
        );
        let _ = reduce(&mut state, NotificationsIntent::OpenList, at(now));

        let outcomes = reduce(&mut state, NotificationsIntent::Clear, at(now));

        assert!(outcomes.is_empty());
        assert_eq!(state.items().count(), 0);
        assert!(state.is_list_open());
    }

    #[test]
    fn the_cross_of_a_row_removes_that_notification_and_keeps_the_list_open() {
        let mut state = NotificationsState::default();
        let now = Instant::now();
        for text in ["disk full", "busy"] {
            let _ = reduce(
                &mut state,
                NotificationsIntent::Push(String::from(text)),
                at(now),
            );
        }
        let _ = reduce(&mut state, NotificationsIntent::OpenList, at(now));
        let disk_full = state
            .items()
            .find(|item| item.text() == "disk full")
            .map(|item| item.id());

        let outcomes = reduce(
            &mut state,
            NotificationsIntent::Remove(disk_full.expect("a notification")),
            at(now),
        );

        let listed: Vec<&str> = state.items().map(|item| item.text()).collect();
        assert!(outcomes.is_empty());
        assert_eq!(listed, vec!["busy"]);
        assert!(state.is_list_open());
    }

    #[test]
    fn the_cross_of_a_toast_hides_it() {
        let mut state = NotificationsState::default();
        let now = Instant::now();
        let _ = reduce(
            &mut state,
            NotificationsIntent::Push(String::from("disk full")),
            at(now),
        );
        let id = state.toasts().map(|toast| toast.id()).next();

        let outcomes = reduce(
            &mut state,
            NotificationsIntent::DismissToast(id.expect("a toast")),
            at(now),
        );

        assert!(outcomes.is_empty());
        assert!(toasts(&state).is_empty());
    }
}
