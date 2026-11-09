use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Most notifications the list keeps; a new one drops the oldest.
const LIST_LIMIT: usize = 100;
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

    /// Return when it came.
    pub(super) fn at(&self) -> Instant {
        self.at
    }
}

/// The last notifications, newest first, which of them show as toasts,
/// how many came since the list was last opened, and when the open list
/// was opened.
#[derive(Debug, Default)]
pub(super) struct NotificationsState {
    items: VecDeque<Notification>,
    next_id: u64,
    unseen: usize,
    list_opened_at: Option<Instant>,
}

impl NotificationsState {
    /// Return the notifications the list keeps, newest first.
    pub(super) fn items(&self) -> impl Iterator<Item = &Notification> {
        self.items.iter()
    }

    /// Return the notifications that show as toasts, newest first.
    pub(super) fn toasts(&self) -> impl Iterator<Item = &Notification> {
        self.items.iter().filter(|item| item.is_toast)
    }

    /// Return whether a toast shows, so the toast timer has to run.
    pub(super) fn has_toasts(&self) -> bool {
        self.toasts().next().is_some()
    }

    /// Return how many notifications came since the list was last opened.
    pub(super) fn unseen(&self) -> usize {
        self.unseen
    }

    /// Return when the open list was opened, or `None` while it is closed.
    pub(super) fn list_opened_at(&self) -> Option<Instant> {
        self.list_opened_at
    }

    /// Return whether the list is open.
    pub(super) fn is_list_open(&self) -> bool {
        self.list_opened_at.is_some()
    }

    /// Add a notification that came at `now`; the list keeps the last 100.
    /// It shows as a toast on top of the others and counts as unseen; with
    /// more than three, the oldest toast goes away. While the list is open
    /// the user already looks at it: no toast, nothing unseen.
    pub(super) fn push(&mut self, text: String, now: Instant) {
        let id = self.next_id;
        self.next_id += 1;
        let is_toast = !self.is_list_open();

        self.items.push_front(Notification {
            id,
            text,
            at: now,
            is_toast,
        });
        self.items.truncate(LIST_LIMIT);
        if is_toast {
            self.unseen = self.unseen.saturating_add(1);
        }
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

    /// Open the list at `now`: every notification counts as seen and the
    /// toasts go away. Returns `false` when the list is already open.
    pub(super) fn open_list(&mut self, now: Instant) -> bool {
        if self.is_list_open() {
            return false;
        }

        self.list_opened_at = Some(now);
        self.unseen = 0;
        for item in self.items.iter_mut() {
            item.is_toast = false;
        }
        true
    }

    /// Close the list. Returns `false` when it is already closed.
    pub(super) fn close_list(&mut self) -> bool {
        self.list_opened_at.take().is_some()
    }

    /// Forget every notification.
    pub(super) fn clear(&mut self) {
        self.items.clear();
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

    fn listed(state: &NotificationsState) -> Vec<&str> {
        state.items().map(Notification::text).collect()
    }

    #[test]
    fn the_list_keeps_every_notification_newest_first_toast_or_not() {
        let mut state = NotificationsState::default();
        let now = Instant::now();
        for text in ["first", "second", "third", "fourth"] {
            state.push(String::from(text), now);
        }
        state.expire_toasts(now + Duration::from_secs(5));

        assert_eq!(listed(&state), vec!["fourth", "third", "second", "first"]);
    }

    #[test]
    fn the_list_keeps_the_last_100_and_drops_the_oldest() {
        let mut state = NotificationsState::default();
        let now = Instant::now();

        for index in 0..=LIST_LIMIT {
            state.push(format!("error {index}"), now);
        }

        let listed = listed(&state);
        assert_eq!(listed.len(), LIST_LIMIT);
        assert_eq!(listed[0], "error 100");
        assert_eq!(listed[LIST_LIMIT - 1], "error 1");
    }

    /// Owner, Q4: the badge counts what came since the list was last
    /// opened, and opening the list marks everything seen.
    #[test]
    fn unseen_counts_the_notifications_since_the_list_was_last_opened() {
        let mut state = NotificationsState::default();
        let now = Instant::now();
        state.push(String::from("first"), now);
        state.push(String::from("second"), now);
        let before = state.unseen();

        let _ = state.open_list(now);
        let opened = state.unseen();
        let _ = state.close_list();
        state.push(String::from("third"), now);

        assert_eq!(before, 2);
        assert_eq!(opened, 0);
        assert_eq!(state.unseen(), 1);
    }

    #[test]
    fn opening_the_list_hides_the_toasts_and_keeps_the_notifications() {
        let mut state = NotificationsState::default();
        let now = Instant::now();
        state.push(String::from("disk full"), now);

        let opened = state.open_list(now + Duration::from_secs(1));

        assert!(opened);
        assert!(state.is_list_open());
        assert_eq!(state.list_opened_at(), Some(now + Duration::from_secs(1)));
        assert!(!state.has_toasts());
        assert_eq!(listed(&state), vec!["disk full"]);
    }

    #[test]
    fn opening_an_open_list_or_closing_a_closed_one_changes_nothing() {
        let mut state = NotificationsState::default();
        let now = Instant::now();

        let closed_while_closed = state.close_list();
        let first = state.open_list(now);
        let again = state.open_list(now + Duration::from_secs(9));
        let closed = state.close_list();

        assert!(!closed_while_closed);
        assert!(first);
        assert!(!again);
        assert!(closed);
        assert!(!state.is_list_open());
        assert_eq!(state.list_opened_at(), None);
    }

    /// The user already looks at the list: a new error joins it without a
    /// toast and without counting as unseen.
    #[test]
    fn an_error_while_the_list_is_open_joins_it_without_a_toast() {
        let mut state = NotificationsState::default();
        let now = Instant::now();
        let _ = state.open_list(now);

        state.push(String::from("disk full"), now);

        assert!(!state.has_toasts());
        assert_eq!(state.unseen(), 0);
        assert_eq!(listed(&state), vec!["disk full"]);
    }

    #[test]
    fn clear_forgets_every_notification_and_keeps_the_list_open() {
        let mut state = NotificationsState::default();
        let now = Instant::now();
        state.push(String::from("first"), now);
        state.push(String::from("second"), now);
        let _ = state.open_list(now);

        state.clear();

        assert!(listed(&state).is_empty());
        assert!(state.is_list_open());
    }

    #[test]
    fn dismissing_an_unknown_toast_changes_nothing() {
        let mut state = NotificationsState::default();
        state.push(String::from("disk full"), Instant::now());

        state.dismiss_toast(999);

        assert_eq!(texts(&state), vec!["disk full"]);
    }
}
