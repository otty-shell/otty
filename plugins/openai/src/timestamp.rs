//! The times Codex writes in its session records and its index of sessions.

use jiff::Timestamp;

/// The time written as RFC 3339 text, such as `2026-11-16T10:00:00.000Z`,
/// in milliseconds since the Unix epoch; `None` when the text is not one.
/// A text with no offset from UTC is not read: it does not tell which
/// moment it is.
pub(crate) fn millis_of(text: &str) -> Option<i64> {
    let time: Timestamp = text.parse().ok()?;

    Some(time.as_millisecond())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_time_with_its_offset_is_read() {
        assert_eq!(
            millis_of("2026-11-16T10:00:00.000Z"),
            Some(1_794_823_200_000)
        );
        assert_eq!(
            millis_of("2026-11-16T12:00:00.250+02:00"),
            Some(1_794_823_200_250)
        );
    }

    #[test]
    fn text_that_is_no_time_is_not_read() {
        for text in ["yesterday", "", "2026-11-16T10:00:00"] {
            assert_eq!(millis_of(text), None, "{text}");
        }
    }
}
