//! The answer of the last `/usage` reading of each profile, so the slow
//! reading runs at most once in the five minutes of the limits read.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use crate::error::SourceError;
use crate::limits::Limits;

/// How long the answer of a reading is given again instead of starting
/// Claude Code anew. The app reads the limits every five minutes; the
/// reading itself starts a little after the timer, by as much as the first
/// source took, so the gap is a little shorter than five minutes. A timer
/// read may then start a new reading each time, while a Refresh or a
/// return to the mode in between gets the remembered answer.
const PROBE_GAP: Duration = Duration::from_secs(4 * 60 + 30);

/// The last reading of each profile, by its config folder: when it started
/// and what it answered.
#[derive(Debug, Default)]
pub(super) struct ProbeMemory {
    readings: Mutex<HashMap<PathBuf, Reading>>,
}

impl ProbeMemory {
    /// The answer of the reading of `config_dir` that started less than
    /// the gap before `now`; `None` when a new reading may start.
    pub(super) fn recent(
        &self,
        config_dir: &Path,
        now: Instant,
    ) -> Option<Result<Limits, SourceError>> {
        let readings =
            self.readings.lock().unwrap_or_else(PoisonError::into_inner);

        let reading = readings.get(config_dir)?;
        let is_recent =
            now.saturating_duration_since(reading.started_at) < PROBE_GAP;

        is_recent.then(|| reading.answer.clone())
    }

    /// Remembers `answer` of the reading of `config_dir` that started at
    /// `started_at`.
    pub(super) fn remember(
        &self,
        config_dir: &Path,
        started_at: Instant,
        answer: Result<Limits, SourceError>,
    ) {
        let mut readings =
            self.readings.lock().unwrap_or_else(PoisonError::into_inner);

        readings
            .insert(config_dir.to_path_buf(), Reading { started_at, answer });
    }
}

/// One reading: when it started and what it answered.
#[derive(Debug, Clone)]
struct Reading {
    started_at: Instant,
    answer: Result<Limits, SourceError>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::limits::{Billing, LimitWindow, UsageWindows, WindowId};

    fn limits() -> Limits {
        let window = LimitWindow::new(WindowId::hours(5), 29.0, None);

        Limits::new(
            Billing::UsageWindows(UsageWindows::new(vec![window], None)),
            None,
        )
    }

    #[test]
    fn a_reading_is_given_again_within_the_gap() {
        let memory = ProbeMemory::default();
        let config_dir = Path::new("/home/user/.claude-work");
        let started_at = Instant::now();

        memory.remember(config_dir, started_at, Ok(limits()));

        let soon = started_at + Duration::from_secs(60);
        assert_eq!(memory.recent(config_dir, soon), Some(Ok(limits())));
    }

    #[test]
    fn a_failed_reading_is_given_again_within_the_gap_too() {
        let memory = ProbeMemory::default();
        let config_dir = Path::new("/home/user/.claude-work");
        let started_at = Instant::now();

        memory.remember(config_dir, started_at, Err(SourceError::NotLoggedIn));

        let soon = started_at + Duration::from_secs(60);
        assert_eq!(
            memory.recent(config_dir, soon),
            Some(Err(SourceError::NotLoggedIn))
        );
    }

    #[test]
    fn a_new_reading_may_start_after_the_gap() {
        let memory = ProbeMemory::default();
        let config_dir = Path::new("/home/user/.claude-work");
        let started_at = Instant::now();

        memory.remember(config_dir, started_at, Ok(limits()));

        assert_eq!(memory.recent(config_dir, started_at + PROBE_GAP), None);
    }

    #[test]
    fn each_profile_has_its_own_reading() {
        let memory = ProbeMemory::default();
        let started_at = Instant::now();

        memory.remember(
            Path::new("/home/user/.claude-work"),
            started_at,
            Ok(limits()),
        );

        let other = memory.recent(Path::new("/home/user/.claude"), started_at);
        assert_eq!(other, None);
    }
}
