use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// How long a toast shows before it goes away on its own.
const TOAST_LIFETIME: Duration = Duration::from_secs(5);
/// Most toasts on screen at once; a new one hides the oldest.
const TOAST_LIMIT: usize = 3;

/// An error the user was told about.
#[derive(Debug)]
pub(super) struct Notification {
    id: u64,
    text: String,
    at: Instant,
    is_toast: bool,
}

impl Notification {
    /// Return the id the toast's cross sends.
    pub(super) fn id(&self) -> u64 {
        self.id
    }

    /// Return the text, in the language the user had when it came.
    pub(super) fn text(&self) -> &str {
        &self.text
    }
}

/// The notifications, newest first, and which of them show as toasts.
#[derive(Debug, Default)]
pub(super) struct NotificationsState {
    items: VecDeque<Notification>,
    next_id: u64,
}

impl NotificationsState {
    /// Return the notifications that show as toasts, newest first.
    pub(super) fn toasts(&self) -> impl Iterator<Item = &Notification> {
        self.items.iter().filter(|item| item.is_toast)
    }

    /// Return whether a toast shows, so the toast timer has to run.
    pub(super) fn has_toasts(&self) -> bool {
        self.toasts().next().is_some()
    }

    /// Add a notification that came at `now`. It shows as a toast on top
    /// of the others; with more than three, the oldest toast goes away.
    pub(super) fn push(&mut self, text: String, now: Instant) {
        let id = self.next_id;
        self.next_id += 1;

        self.items.push_front(Notification {
            id,
            text,
            at: now,
            is_toast: true,
        });
        for item in self
            .items
            .iter_mut()
            .filter(|item| item.is_toast)
            .skip(TOAST_LIMIT)
        {
            item.is_toast = false;
        }
    }

    /// Hide the toast `id`; its notification stays.
    pub(super) fn dismiss_toast(&mut self, id: u64) {
        if let Some(item) = self.items.iter_mut().find(|item| item.id == id) {
            item.is_toast = false;
        }
    }

    /// Hide the toasts that have shown for five seconds at `now`.
    pub(super) fn expire_toasts(&mut self, now: Instant) {
        for item in self.items.iter_mut() {
            if now.saturating_duration_since(item.at) >= TOAST_LIFETIME {
                item.is_toast = false;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(state: &NotificationsState) -> Vec<&str> {
        state.toasts().map(Notification::text).collect()
    }

    fn ids(state: &NotificationsState) -> Vec<u64> {
        state.toasts().map(Notification::id).collect()
    }

    #[test]
    fn a_new_state_shows_no_toast() {
        let state = NotificationsState::default();

        assert!(texts(&state).is_empty());
        assert!(!state.has_toasts());
    }

    #[test]
    fn a_pushed_error_shows_as_a_toast_on_top_of_the_older_ones() {
        let mut state = NotificationsState::default();
        let now = Instant::now();

        state.push(String::from("disk full"), now);
        state.push(String::from("busy"), now);

        assert_eq!(texts(&state), vec!["busy", "disk full"]);
        assert!(state.has_toasts());
    }

    #[test]
    fn each_notification_gets_its_own_id() {
        let mut state = NotificationsState::default();
        let now = Instant::now();

        state.push(String::from("a"), now);
        state.push(String::from("b"), now);

        let ids = ids(&state);
        assert_eq!(ids.len(), 2);
        assert_ne!(ids[0], ids[1]);
    }

    #[test]
    fn at_most_three_toasts_show_and_a_new_one_hides_the_oldest() {
        let mut state = NotificationsState::default();
        let now = Instant::now();

        for text in ["first", "second", "third", "fourth"] {
            state.push(String::from(text), now);
        }

        assert_eq!(texts(&state), vec!["fourth", "third", "second"]);
    }

    #[test]
    fn a_toast_goes_away_after_five_seconds() {
        let mut state = NotificationsState::default();
        let pushed = Instant::now();
        state.push(String::from("disk full"), pushed);

        state.expire_toasts(pushed + Duration::from_millis(4_900));
        let before = texts(&state).len();
        state.expire_toasts(pushed + Duration::from_secs(5));

        assert_eq!(before, 1);
        assert!(texts(&state).is_empty());
        assert!(!state.has_toasts());
    }

    #[test]
    fn each_toast_counts_its_own_five_seconds() {
        let mut state = NotificationsState::default();
        let first = Instant::now();
        state.push(String::from("older"), first);
        state.push(String::from("newer"), first + Duration::from_secs(3));

        state.expire_toasts(first + Duration::from_secs(5));

        assert_eq!(texts(&state), vec!["newer"]);
    }

    #[test]
    fn a_dismissed_toast_goes_away_and_the_others_stay() {
        let mut state = NotificationsState::default();
        let now = Instant::now();
        for text in ["first", "second", "third"] {
            state.push(String::from(text), now);
        }
        let second = ids(&state)[1];

        state.dismiss_toast(second);

        assert_eq!(texts(&state), vec!["third", "first"]);
    }

    /// A toast hidden by the cross does not come back when the oldest of
    /// three goes away: a new toast only hides older ones.
    #[test]
    fn a_dismissed_toast_stays_hidden_after_a_new_one() {
        let mut state = NotificationsState::default();
        let now = Instant::now();
        state.push(String::from("first"), now);
        let first = ids(&state)[0];

        state.dismiss_toast(first);
        state.push(String::from("second"), now);

        assert_eq!(texts(&state), vec!["second"]);
    }

    #[test]
    fn dismissing_an_unknown_toast_changes_nothing() {
        let mut state = NotificationsState::default();
        state.push(String::from("disk full"), Instant::now());

        state.dismiss_toast(999);

        assert_eq!(texts(&state), vec!["disk full"]);
    }
}
