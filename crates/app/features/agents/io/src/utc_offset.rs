//! The user's offset from UTC, which tells where the days of the tokens
//! spent start and in which time zone the dashboard writes its times.
//!
//! The workspace has no crate that knows the time zones, so the offset is
//! asked from the system `date` program once for each count, and once for
//! each read of the profiles, which brings it to the feature before any
//! limits. With no answer the days are the days of UTC and the times are
//! written in UTC.

use std::process::Command;

/// The user's offset from UTC in seconds, east of UTC positive, as `date
/// +%z` tells it now; `None` when it can not be read.
pub(crate) fn local_utc_offset() -> Option<i32> {
    let output = Command::new("date").arg("+%z").output();

    let offset = match output {
        Ok(output) if output.status.success() => {
            parse_utc_offset(&String::from_utf8_lossy(&output.stdout))
        },
        _ => None,
    };

    if offset.is_none() {
        log::warn!("the offset from UTC could not be read; days are in UTC");
    }

    offset
}

/// The offset written as `+hhmm` or `-hhmm`, such as `+0300`, in seconds.
fn parse_utc_offset(text: &str) -> Option<i32> {
    let text = text.trim();
    let sign = match text.as_bytes().first() {
        Some(b'+') => 1,
        Some(b'-') => -1,
        _ => return None,
    };
    let digits = text.get(1..)?;
    if digits.len() != 4 || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }

    let hours: i32 = digits.get(..2)?.parse().ok()?;
    let minutes: i32 = digits.get(2..)?.parse().ok()?;

    Some(sign * (hours * 3600 + minutes * 60))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_offset_east_or_west_of_utc_is_read() {
        assert_eq!(parse_utc_offset("+0300\n"), Some(10_800));
        assert_eq!(parse_utc_offset("-0430"), Some(-16_200));
        assert_eq!(parse_utc_offset("+0000"), Some(0));
    }

    #[test]
    fn text_that_is_not_an_offset_is_not_read() {
        for text in ["", "UTC", "0300", "+03:00", "+3", "+03a0", "+０３００"]
        {
            assert_eq!(parse_utc_offset(text), None, "{text}");
        }
    }
}
