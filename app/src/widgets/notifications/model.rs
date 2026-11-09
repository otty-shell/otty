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
    fn a_text_of_300_characters_stays_whole() {
        let exact = "a".repeat(TOAST_TEXT_LIMIT);
        let state = state_with(&[&exact]);

        let vm = ToastStackViewModel::from_state(&state);

        assert_eq!(vm.toasts[0].text, exact);
    }
}
