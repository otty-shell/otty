//! What a block of the dashboard draws of a kind that is read again and
//! again, by the rules of spec Decision 5: the last good answer stays,
//! greyed, when a later read fails.

use std::time::SystemTime;

/// What a block draws of one kind: nothing, why the reads failed while
/// none gave an answer, or the last good answer, greyed with why the
/// latest read failed when it did.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum KeptAnswer<'a, T, F: ?Sized> {
    /// Nothing to draw: no read ended yet.
    Nothing,
    /// No read gave an answer; the latest failed for this reason.
    OnlyFailure(&'a F),
    /// The last good answer and when it was read, with why the latest
    /// read failed when it did: the answer is then drawn greyed.
    Answer {
        value: &'a T,
        read_at: SystemTime,
        failure: Option<&'a F>,
    },
}

impl<'a, T, F: ?Sized> KeptAnswer<'a, T, F> {
    /// What to draw from the `last_good` answer with its time and the
    /// `failure` of the latest read.
    pub(super) fn new(
        last_good: Option<(&'a T, SystemTime)>,
        failure: Option<&'a F>,
    ) -> Self {
        match (last_good, failure) {
            (Some((value, read_at)), failure) => Self::Answer {
                value,
                read_at,
                failure,
            },
            (None, Some(failure)) => Self::OnlyFailure(failure),
            (None, None) => Self::Nothing,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::at;

    #[test]
    fn a_failure_after_an_answer_keeps_the_answer_with_the_failure() {
        let answer = 42;
        let failure = "down";

        let kept = KeptAnswer::new(Some((&answer, at(10))), Some(failure));
        let only = KeptAnswer::<u32, str>::new(None, Some(failure));
        let nothing = KeptAnswer::<u32, str>::new(None, None);

        assert_eq!(
            kept,
            KeptAnswer::Answer {
                value: &42,
                read_at: at(10),
                failure: Some("down"),
            }
        );
        assert_eq!(only, KeptAnswer::OnlyFailure("down"));
        assert_eq!(nothing, KeptAnswer::Nothing);
    }
}
