//! The user's time zone, which tells where the days of the tokens spent
//! start and in which time zone the dashboard writes its times.
//!
//! It is asked from the system once for each count, and once for each read
//! of the profiles, which brings it to the feature before any limits. With
//! no answer the days are the days of UTC and the times are written in UTC.

use jiff::tz::TimeZone;

/// The user's time zone as the system tells it now: the `TZ` variable, or
/// the zone the system is set to; `None` when it can not be read.
pub(crate) fn user_time_zone() -> Option<TimeZone> {
    match TimeZone::try_system() {
        Ok(time_zone) => Some(time_zone),
        Err(error) => {
            log::warn!(
                "the time zone could not be read, days are in UTC: {error}"
            );
            None
        },
    }
}
