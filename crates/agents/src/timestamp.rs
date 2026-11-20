//! Reads the times the plugin parts tell: counts of milliseconds since
//! 1970.

use std::time::{Duration, SystemTime};

/// The time `millis` milliseconds after 1970; `None` for a count that is
/// negative, not finite or too far to be a time.
pub(crate) fn time_from_millis(millis: f64) -> Option<SystemTime> {
    let since_epoch = Duration::try_from_secs_f64(millis / 1000.0).ok()?;

    SystemTime::UNIX_EPOCH.checked_add(since_epoch)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::at;

    #[test]
    fn milliseconds_since_1970_are_read() {
        assert_eq!(
            time_from_millis(1_794_823_200_000.0),
            Some(at(1_794_823_200))
        );
        assert_eq!(time_from_millis(-1.0), None);
        assert_eq!(time_from_millis(f64::NAN), None);
    }
}
