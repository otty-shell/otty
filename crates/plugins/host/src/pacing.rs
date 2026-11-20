//! The pace of the slow readings of limits: a reading that ran a program
//! or a hidden terminal, such as `codex app-server` or Claude Code's
//! `/usage` screen, runs at most once in the five minutes of the limits
//! read, per part and profile. Within that time the answer of the last
//! reading is given again, for any plugin.

use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use otty_plugin_manifest::PartRef;

use crate::operations::v0_1::otty::host::types::{Limits, ReadError};

/// How long the answer of a reading is given again instead of starting
/// the program anew. The app reads the limits every five minutes; a
/// reading starts a little after the timer, by as much as the reads
/// before it took, so the gap is a little shorter than five minutes. A
/// timer read may then start a new reading each time, while a Refresh or
/// a return to Agents mode in between gets the remembered answer.
pub(crate) const PACE_GAP: Duration = Duration::from_secs(4 * 60 + 30);

/// What a reading of limits gave: the limits, or why the part could not
/// read them.
pub(crate) type LimitsAnswer = Result<Limits, ReadError>;

/// The last slow reading of each part and profile of one plugin.
#[derive(Default)]
pub(crate) struct LimitPacing {
    readings: Mutex<HashMap<PaceKey, Reading>>,
}

impl LimitPacing {
    /// The answer of the reading of `key` that started less than the gap
    /// before `now`; `None` when a new reading may start.
    pub(crate) fn recent(
        &self,
        key: &PaceKey,
        now: Instant,
    ) -> Option<LimitsAnswer> {
        let readings =
            self.readings.lock().unwrap_or_else(PoisonError::into_inner);

        let reading = readings.get(key)?;
        let is_recent =
            now.saturating_duration_since(reading.started_at) < PACE_GAP;

        is_recent.then(|| reading.answer.clone())
    }

    /// Remembers `answer` of the slow reading of `key` that started at
    /// `started_at`.
    pub(crate) fn remember(
        &self,
        key: PaceKey,
        started_at: Instant,
        answer: LimitsAnswer,
    ) {
        let mut readings =
            self.readings.lock().unwrap_or_else(PoisonError::into_inner);

        readings.insert(key, Reading { started_at, answer });
    }
}

/// Which reading is paced: the part and the profile's folder.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct PaceKey {
    part: PartRef,
    folder: String,
}

impl PaceKey {
    /// The reading of the profile in `folder` by `part`.
    pub(crate) fn new(part: &PartRef, folder: &str) -> Self {
        Self {
            part: part.clone(),
            folder: folder.to_string(),
        }
    }
}

/// One slow reading: when it started and what it answered.
struct Reading {
    started_at: Instant,
    answer: LimitsAnswer,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operations::v0_1::otty::host::types::LimitWindow;

    fn limits(percent_used: f64) -> Limits {
        Limits {
            windows: vec![LimitWindow {
                length_minutes: Some(300),
                model: None,
                percent_used,
                resets_at_ms: None,
                reset_text: None,
            }],
            credits: None,
            plan: None,
            recorded_at_ms: None,
        }
    }

    fn key(folder: &str) -> PaceKey {
        PaceKey::new(&PartRef::agent("test-agent"), folder)
    }

    fn percent(answer: Option<LimitsAnswer>) -> Option<f64> {
        answer?.ok().map(|limits| limits.windows[0].percent_used)
    }

    #[test]
    fn pacing_skips_a_second_probe_within_five_minutes() {
        let pacing = LimitPacing::default();
        let started_at = Instant::now();

        pacing.remember(key("/home/u/.agent"), started_at, Ok(limits(29.0)));

        let soon = started_at + Duration::from_secs(60);
        assert_eq!(
            percent(pacing.recent(&key("/home/u/.agent"), soon)),
            Some(29.0)
        );
    }

    #[test]
    fn a_failed_reading_is_given_again_within_the_gap_too() {
        let pacing = LimitPacing::default();
        let started_at = Instant::now();

        pacing.remember(
            key("/home/u/.agent"),
            started_at,
            Err(ReadError::Failed("not logged in".to_string())),
        );

        let soon = started_at + Duration::from_secs(60);
        let answer = pacing.recent(&key("/home/u/.agent"), soon);
        assert!(matches!(answer, Some(Err(ReadError::Failed(_)))));
    }

    #[test]
    fn a_new_reading_may_start_after_the_gap() {
        let pacing = LimitPacing::default();
        let started_at = Instant::now();

        pacing.remember(key("/home/u/.agent"), started_at, Ok(limits(29.0)));

        let later = started_at + PACE_GAP;
        assert!(pacing.recent(&key("/home/u/.agent"), later).is_none());
    }

    #[test]
    fn each_profile_and_part_has_its_own_reading() {
        let pacing = LimitPacing::default();
        let started_at = Instant::now();

        pacing.remember(key("/home/u/.agent"), started_at, Ok(limits(29.0)));

        let other_profile = key("/home/u/.agent-work");
        let other_part =
            PaceKey::new(&PartRef::agent("other-agent"), "/home/u/.agent");
        assert!(pacing.recent(&other_profile, started_at).is_none());
        assert!(pacing.recent(&other_part, started_at).is_none());
    }
}
