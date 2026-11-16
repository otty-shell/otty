//! How long a command of a block took, as the text shown in the status line
//! of the block and in the sticky header.
//!
//! All times are milliseconds of one clock: the one the terminal stamps the
//! blocks with (`started_at` and `finished_at` of the block metadata).
//! Values are cut, not rounded, so the text never shows more time than has
//! passed and `59.99` seconds do not read as `60.0 s`.

const MS_PER_SECOND: i64 = 1_000;
const SECONDS_PER_MINUTE: i64 = 60;
const MINUTES_PER_HOUR: i64 = 60;
const MS_PER_MINUTE: i64 = MS_PER_SECOND * SECONDS_PER_MINUTE;

/// Return how long a finished command took: `0.2 s` and `48.2 s` under a
/// minute, `1 m 12 s` from a minute, `1 h 04 m` from an hour.
///
/// An end before the start (the system clock was set back) gives `0.0 s`.
pub(crate) fn finished_duration_text(
    started_at_ms: i64,
    finished_at_ms: i64,
) -> String {
    let elapsed_ms = elapsed_ms(started_at_ms, finished_at_ms);

    if elapsed_ms < MS_PER_MINUTE {
        let seconds = elapsed_ms / MS_PER_SECOND;
        let tenths = elapsed_ms % MS_PER_SECOND / 100;
        return format!("{seconds}.{tenths} s");
    }

    minutes_or_hours_text(elapsed_ms)
}

/// Return how long a running command has run so far: whole seconds (`12 s`)
/// under a minute, then `1 m 12 s` and `1 h 04 m` like a finished one.
///
/// A `now_ms` before the start (the system clock was set back) gives `0 s`.
pub(crate) fn running_duration_text(started_at_ms: i64, now_ms: i64) -> String {
    let elapsed_ms = elapsed_ms(started_at_ms, now_ms);

    if elapsed_ms < MS_PER_MINUTE {
        let seconds = elapsed_ms / MS_PER_SECOND;
        return format!("{seconds} s");
    }

    minutes_or_hours_text(elapsed_ms)
}

/// Time between two moments, never negative.
fn elapsed_ms(from_ms: i64, to_ms: i64) -> i64 {
    to_ms.saturating_sub(from_ms).max(0)
}

/// Text of a duration of a minute or more: minutes and seconds under an
/// hour, hours and minutes from an hour.
fn minutes_or_hours_text(elapsed_ms: i64) -> String {
    let total_seconds = elapsed_ms / MS_PER_SECOND;
    let total_minutes = total_seconds / SECONDS_PER_MINUTE;

    if total_minutes < MINUTES_PER_HOUR {
        let seconds = total_seconds % SECONDS_PER_MINUTE;
        return format!("{total_minutes} m {seconds:02} s");
    }

    let hours = total_minutes / MINUTES_PER_HOUR;
    let minutes = total_minutes % MINUTES_PER_HOUR;
    format!("{hours} h {minutes:02} m")
}

#[cfg(test)]
mod tests {
    use super::{finished_duration_text, running_duration_text};

    const SECOND: i64 = 1_000;
    const MINUTE: i64 = 60 * SECOND;
    const HOUR: i64 = 60 * MINUTE;

    /// A start time like the real ones, so the tests do not pass only for
    /// a start at zero.
    const START: i64 = 1_800_000_000_000;

    fn finished(elapsed_ms: i64) -> String {
        finished_duration_text(START, START + elapsed_ms)
    }

    fn running(elapsed_ms: i64) -> String {
        running_duration_text(START, START + elapsed_ms)
    }

    #[test]
    fn given_finished_under_a_minute_when_written_then_seconds_have_one_decimal()
     {
        assert_eq!(finished(0), "0.0 s");
        assert_eq!(finished(200), "0.2 s");
        assert_eq!(finished(48_200), "48.2 s");
    }

    #[test]
    fn given_finished_around_one_second_when_written_then_the_tenths_are_cut() {
        assert_eq!(finished(SECOND - 1), "0.9 s");
        assert_eq!(finished(SECOND), "1.0 s");
        assert_eq!(finished(SECOND + 99), "1.0 s");
        assert_eq!(finished(SECOND + 100), "1.1 s");
    }

    #[test]
    fn given_finished_around_a_minute_when_written_then_minutes_start_at_sixty_seconds()
     {
        assert_eq!(finished(MINUTE - 1), "59.9 s");
        assert_eq!(finished(MINUTE), "1 m 00 s");
        assert_eq!(finished(MINUTE + 12 * SECOND), "1 m 12 s");
        assert_eq!(finished(MINUTE + 12 * SECOND + 999), "1 m 12 s");
    }

    #[test]
    fn given_finished_around_an_hour_when_written_then_hours_start_at_sixty_minutes()
     {
        assert_eq!(finished(HOUR - 1), "59 m 59 s");
        assert_eq!(finished(HOUR), "1 h 00 m");
        assert_eq!(finished(HOUR + 4 * MINUTE), "1 h 04 m");
        assert_eq!(finished(HOUR + 4 * MINUTE + 59 * SECOND), "1 h 04 m");
        assert_eq!(finished(26 * HOUR + 30 * MINUTE), "26 h 30 m");
    }

    #[test]
    fn given_running_under_a_minute_when_written_then_seconds_are_whole() {
        assert_eq!(running(0), "0 s");
        assert_eq!(running(SECOND - 1), "0 s");
        assert_eq!(running(SECOND), "1 s");
        assert_eq!(running(12 * SECOND + 900), "12 s");
        assert_eq!(running(MINUTE - 1), "59 s");
    }

    #[test]
    fn given_running_from_a_minute_when_written_then_it_reads_like_a_finished_one()
     {
        assert_eq!(running(MINUTE), "1 m 00 s");
        assert_eq!(running(MINUTE + 12 * SECOND), "1 m 12 s");
        assert_eq!(running(HOUR - 1), "59 m 59 s");
        assert_eq!(running(HOUR), "1 h 00 m");
        assert_eq!(running(HOUR + 4 * MINUTE), "1 h 04 m");
    }

    #[test]
    fn given_end_before_start_when_written_then_the_duration_is_zero() {
        assert_eq!(finished(-5 * SECOND), "0.0 s");
        assert_eq!(running(-5 * SECOND), "0 s");
        assert_eq!(finished_duration_text(i64::MAX, i64::MIN), "0.0 s");
    }
}
