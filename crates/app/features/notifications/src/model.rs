use std::time::Duration;

use otty_app_i18n::{self as i18n, Key};

use super::state::NotificationsState;

/// Most characters a toast shows; a longer text is cut and ends with "…".
/// The list shows the whole text.
const TOAST_TEXT_LIMIT: usize = 300;

/// What the toast stack shows: the toasts, newest first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ToastStackViewModel {
    pub(super) toasts: Vec<ToastStackItemViewModel>,
}

impl ToastStackViewModel {
    /// Build the view model from the widget state.
    pub(super) fn from_state(state: &NotificationsState) -> Self {
        let toasts = state
            .toasts()
            .map(|toast| ToastStackItemViewModel {
                id: toast.id(),
                text: toast_text(toast.text()),
            })
            .collect();

        Self { toasts }
    }
}

/// One toast: the id its cross sends and its text, cut when long.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ToastStackItemViewModel {
    pub(super) id: u64,
    pub(super) text: String,
}

/// What the open list shows: the notifications, newest first, and whether
/// the broom has anything to clear.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NotificationListViewModel<'a> {
    pub(super) rows: Vec<NotificationListRowViewModel<'a>>,
    pub(super) can_clear: bool,
}

impl<'a> NotificationListViewModel<'a> {
    /// Build the view model from the widget state, or `None` while the
    /// list is closed.
    pub(super) fn from_state(state: &'a NotificationsState) -> Option<Self> {
        let opened_at = state.list_opened_at()?;

        let rows: Vec<NotificationListRowViewModel<'a>> = state
            .items()
            .map(|item| NotificationListRowViewModel {
                id: item.id(),
                text: item.text(),
                age: age_text(opened_at.saturating_duration_since(item.at())),
            })
            .collect();

        Some(Self {
            can_clear: !rows.is_empty(),
            rows,
        })
    }
}

/// One row of the list: the id its cross removes, the whole text and how
/// long ago it came, counted to the moment the list opened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NotificationListRowViewModel<'a> {
    pub(super) id: u64,
    pub(super) text: &'a str,
    pub(super) age: String,
}

/// Return how long ago a notification came, in words: "just now" under a
/// minute, then whole minutes, then whole hours.
fn age_text(elapsed: Duration) -> String {
    let minutes = elapsed.as_secs() / 60;

    match minutes {
        0 => i18n::t(Key::NotificationJustNow).to_owned(),
        1..60 => i18n::with_count(Key::TplNotificationMinutesAgo, minutes),
        _ => i18n::with_count(Key::TplNotificationHoursAgo, minutes / 60),
    }
}

/// Return `text` as a toast shows it: whole up to 300 characters, else its
/// first 300 characters and "…".
fn toast_text(text: &str) -> String {
    match text.char_indices().nth(TOAST_TEXT_LIMIT) {
        Some((cut, _)) => format!("{}…", &text[..cut]),
        None => text.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::*;

    fn state_with(texts: &[&str]) -> NotificationsState {
        let mut state = NotificationsState::default();
        for text in texts {
            state.push(String::from(*text), Instant::now());
        }

        state
    }

    #[test]
    fn the_toasts_show_newest_first_with_their_ids() {
        let state = state_with(&["disk full", "busy"]);
        let ids: Vec<u64> = state.toasts().map(|toast| toast.id()).collect();

        let vm = ToastStackViewModel::from_state(&state);

        assert_eq!(
            vm.toasts,
            vec![
                ToastStackItemViewModel {
                    id: ids[0],
                    text: String::from("busy"),
                },
                ToastStackItemViewModel {
                    id: ids[1],
                    text: String::from("disk full"),
                },
            ]
        );
    }

    #[test]
    fn no_toast_shows_nothing() {
        let vm =
            ToastStackViewModel::from_state(&NotificationsState::default());

        assert!(vm.toasts.is_empty());
    }

    /// A long error, such as a whole engine answer, does not fill the
    /// window: the toast shows its first 300 characters and "…". Letters
    /// that take several bytes are never cut in half.
    #[test]
    fn a_long_text_is_cut_at_300_characters_with_an_ellipsis() {
        let long = "ж".repeat(TOAST_TEXT_LIMIT + 20);
        let state = state_with(&[&long]);

        let vm = ToastStackViewModel::from_state(&state);

        let text = &vm.toasts[0].text;
        assert_eq!(text.chars().count(), TOAST_TEXT_LIMIT + 1);
        assert!(text.ends_with('…'), "{text}");
        assert!(text.starts_with(&"ж".repeat(TOAST_TEXT_LIMIT)));
    }

    #[test]
    fn the_list_has_no_view_model_while_it_is_closed() {
        let state = state_with(&["disk full"]);

        assert_eq!(NotificationListViewModel::from_state(&state), None);
    }

    /// Owner, Q3: the list shows relative times, counted to the moment it
    /// opened. Each row carries the id its cross removes.
    #[test]
    fn the_rows_show_newest_first_with_their_ids_and_age_when_the_list_opened()
    {
        let mut state = NotificationsState::default();
        let first = Instant::now();
        state.push(String::from("disk full"), first);
        state.push(String::from("busy"), first + Duration::from_secs(180));
        let _ = state.open_list(first + Duration::from_secs(210));
        let ids: Vec<u64> = state.items().map(|item| item.id()).collect();

        let vm = NotificationListViewModel::from_state(&state)
            .expect("the list is open");

        assert_eq!(
            vm.rows,
            vec![
                NotificationListRowViewModel {
                    id: ids[0],
                    text: "busy",
                    age: i18n::t(Key::NotificationJustNow).to_owned(),
                },
                NotificationListRowViewModel {
                    id: ids[1],
                    text: "disk full",
                    age: i18n::with_count(Key::TplNotificationMinutesAgo, 3),
                },
            ]
        );
        assert_ne!(ids[0], ids[1]);
        assert!(vm.can_clear);
    }

    /// The list shows the whole text, not the cut one of a toast.
    #[test]
    fn a_row_shows_the_whole_text() {
        let long = "a".repeat(TOAST_TEXT_LIMIT + 20);
        let mut state = state_with(&[&long]);
        let _ = state.open_list(Instant::now());

        let vm = NotificationListViewModel::from_state(&state)
            .expect("the list is open");

        assert_eq!(vm.rows[0].text, long);
    }

    #[test]
    fn an_empty_list_has_nothing_to_clear() {
        let mut state = NotificationsState::default();
        let _ = state.open_list(Instant::now());

        let vm = NotificationListViewModel::from_state(&state)
            .expect("the list is open");

        assert!(vm.rows.is_empty());
        assert!(!vm.can_clear);
    }

    #[test]
    fn ages_read_just_now_then_minutes_then_hours() {
        let minutes =
            |count| i18n::with_count(Key::TplNotificationMinutesAgo, count);
        let hours =
            |count| i18n::with_count(Key::TplNotificationHoursAgo, count);
        let just_now = i18n::t(Key::NotificationJustNow).to_owned();

        assert_eq!(age_text(Duration::ZERO), just_now);
        assert_eq!(age_text(Duration::from_secs(59)), just_now);
        assert_eq!(age_text(Duration::from_secs(60)), minutes(1));
        assert_eq!(age_text(Duration::from_secs(59 * 60 + 59)), minutes(59));
        assert_eq!(age_text(Duration::from_secs(60 * 60)), hours(1));
        assert_eq!(age_text(Duration::from_secs(25 * 60 * 60)), hours(25));
    }

    #[test]
    fn a_text_of_300_characters_stays_whole() {
        let exact = "a".repeat(TOAST_TEXT_LIMIT);
        let state = state_with(&[&exact]);

        let vm = ToastStackViewModel::from_state(&state);

        assert_eq!(vm.toasts[0].text, exact);
    }
}
