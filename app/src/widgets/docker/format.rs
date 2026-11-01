//! Short texts for the Docker tables: ages, sizes, IDs and ports.

use otty_docker::PortMapping;

use crate::i18n::{self, Key};

/// Shown in a cell that has no value.
pub(crate) const NO_VALUE: &str = "—";
/// SI size units, as the Docker CLI uses them.
const SIZE_UNITS: [&str; 5] = ["B", "kB", "MB", "GB", "TB"];
/// Characters of an ID that `docker ps` and `docker images` show.
const SHORT_ID_LENGTH: usize = 12;
/// Prefix of image and digest IDs.
const DIGEST_PREFIX: &str = "sha256:";

const MINUTE: i64 = 60;
const HOUR: i64 = 60 * MINUTE;
const DAY: i64 = 24 * HOUR;

/// How long ago `created` was, at `now` (both Unix seconds): "2 min", "2 h",
/// "3 d", "2 w", "1 mo", "1 y". Unknown times show [`NO_VALUE`].
pub(crate) fn age_text(now: i64, created: Option<i64>) -> String {
    let Some(created) = created else {
        return NO_VALUE.to_string();
    };

    match age_bucket(now - created) {
        Some((key, count)) => i18n::with_count(key, count),
        None => i18n::t(Key::DockerAgeUnderMinute).to_string(),
    }
}

/// A size in bytes with three significant digits and SI units, as the
/// Docker CLI shows it: "13.3 kB", "192 MB", "1.12 GB".
pub(crate) fn size_text(bytes: u64) -> String {
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1000.0 && unit < SIZE_UNITS.len() - 1 {
        value /= 1000.0;
        unit += 1;
    }

    let mut number = three_digits(value);
    // 999.6 kB rounds to "1000"; that is 1 MB.
    if number == "1000" && unit < SIZE_UNITS.len() - 1 {
        unit += 1;
        number = three_digits(value / 1000.0);
    }

    let unit = SIZE_UNITS[unit];
    format!("{number} {unit}")
}

/// The first 12 characters of an ID, without the `sha256:` prefix.
pub(crate) fn short_id(id: &str) -> &str {
    let id = id.strip_prefix(DIGEST_PREFIX).unwrap_or(id);

    id.get(..SHORT_ID_LENGTH).unwrap_or(id)
}

/// Published and exposed ports: "18080→80/tcp, 53/udp". Docker lists a
/// published port once for `0.0.0.0` and once for `::`; it is shown once.
pub(crate) fn ports_text(ports: &[PortMapping]) -> String {
    let mut shown: Vec<(Option<u16>, u16, &str)> = Vec::new();
    for port in ports {
        let key = (port.host_port(), port.container_port(), port.protocol());
        if !shown.contains(&key) {
            shown.push(key);
        }
    }

    if shown.is_empty() {
        return NO_VALUE.to_string();
    }

    shown
        .into_iter()
        .map(|(host, container, protocol)| match host {
            Some(host) => format!("{host}→{container}/{protocol}"),
            None => format!("{container}/{protocol}"),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// The template and number of an age of `seconds`; `None` for under a
/// minute. A time in the future (a clock skew) counts as just now.
fn age_bucket(seconds: i64) -> Option<(Key, u64)> {
    let whole = |unit: i64| u64::try_from(seconds / unit).unwrap_or(0);

    if seconds < MINUTE {
        return None;
    }
    if seconds < HOUR {
        return Some((Key::TplDockerAgeMinutes, whole(MINUTE)));
    }
    if seconds < DAY {
        return Some((Key::TplDockerAgeHours, whole(HOUR)));
    }

    let days = seconds / DAY;
    if days < 14 {
        return Some((Key::TplDockerAgeDays, whole(DAY)));
    }
    if days < 30 {
        return Some((Key::TplDockerAgeWeeks, whole(7 * DAY)));
    }
    if days < 365 {
        return Some((Key::TplDockerAgeMonths, whole(30 * DAY)));
    }

    Some((Key::TplDockerAgeYears, whole(365 * DAY)))
}

/// `value` with three significant digits and no trailing zeros: "6.69",
/// "34", "192".
fn three_digits(value: f64) -> String {
    let decimals = if value < 10.0 {
        2
    } else if value < 100.0 {
        1
    } else {
        0
    };

    let text = format!("{value:.decimals$}");
    if !text.contains('.') {
        return text;
    }

    text.trim_end_matches('0').trim_end_matches('.').to_string()
}

#[cfg(test)]
mod tests {
    use otty_docker::{EngineKind, fixtures};

    use super::*;

    #[test]
    fn an_age_under_a_minute_has_no_bucket() {
        assert_eq!(age_bucket(0), None);
        assert_eq!(age_bucket(59), None);
        // A creation time after now (a clock skew) counts as just now.
        assert_eq!(age_bucket(-30), None);
    }

    #[test]
    fn ages_go_from_minutes_to_years() {
        assert_eq!(age_bucket(2 * MINUTE), Some((Key::TplDockerAgeMinutes, 2)));
        assert_eq!(age_bucket(HOUR - 1), Some((Key::TplDockerAgeMinutes, 59)));
        assert_eq!(age_bucket(2 * HOUR), Some((Key::TplDockerAgeHours, 2)));
        assert_eq!(age_bucket(DAY - 1), Some((Key::TplDockerAgeHours, 23)));
        assert_eq!(age_bucket(3 * DAY), Some((Key::TplDockerAgeDays, 3)));
        assert_eq!(age_bucket(13 * DAY), Some((Key::TplDockerAgeDays, 13)));
        assert_eq!(age_bucket(14 * DAY), Some((Key::TplDockerAgeWeeks, 2)));
        assert_eq!(age_bucket(29 * DAY), Some((Key::TplDockerAgeWeeks, 4)));
        assert_eq!(age_bucket(30 * DAY), Some((Key::TplDockerAgeMonths, 1)));
        assert_eq!(age_bucket(364 * DAY), Some((Key::TplDockerAgeMonths, 12)));
        assert_eq!(age_bucket(365 * DAY), Some((Key::TplDockerAgeYears, 1)));
        assert_eq!(
            age_bucket(3 * 365 * DAY),
            Some((Key::TplDockerAgeYears, 3))
        );
    }

    #[test]
    fn an_age_text_fills_its_template() {
        let now = 1_800_000_000;

        assert_eq!(
            age_text(now, Some(now - 2 * HOUR)),
            i18n::with_count(Key::TplDockerAgeHours, 2)
        );
        assert_eq!(
            age_text(now, Some(now - 10)),
            i18n::t(Key::DockerAgeUnderMinute)
        );
        assert_eq!(age_text(now, None), NO_VALUE);
    }

    #[test]
    fn sizes_keep_three_significant_digits() {
        assert_eq!(size_text(0), "0 B");
        assert_eq!(size_text(999), "999 B");
        assert_eq!(size_text(13_312), "13.3 kB");
        assert_eq!(size_text(6_685_392), "6.69 MB");
        assert_eq!(size_text(34_000_000), "34 MB");
        assert_eq!(size_text(192_000_000), "192 MB");
        assert_eq!(size_text(1_120_000_000), "1.12 GB");
        assert_eq!(size_text(1_100_000_000), "1.1 GB");
        assert_eq!(size_text(2_500_000_000_000), "2.5 TB");
    }

    #[test]
    fn a_size_just_under_a_unit_rounds_up_into_it() {
        assert_eq!(size_text(999_999), "1 MB");
    }

    #[test]
    fn a_short_id_drops_the_digest_prefix() {
        assert_eq!(
            short_id("sha256:a7dbc319b14389d5ff16de8dd4b69e5b93ab40e4"),
            "a7dbc319b143"
        );
        assert_eq!(short_id("1f3a9c2e7b40aa"), "1f3a9c2e7b40");
        assert_eq!(short_id("abc"), "abc");
    }

    #[test]
    fn docker_port_pairs_for_both_address_families_show_once() {
        let resources = fixtures::resources(EngineKind::Docker);
        let running = &resources.containers()[5];

        assert_eq!(running.name(), "otty-fx-running");
        assert_eq!(running.ports().len(), 2);
        assert_eq!(ports_text(running.ports()), "18080→80/tcp");
    }

    #[test]
    fn a_container_without_ports_shows_no_value() {
        let resources = fixtures::resources(EngineKind::Podman);
        let created = &resources.containers()[3];

        assert_eq!(created.name(), "otty-fx-created");
        assert_eq!(ports_text(created.ports()), NO_VALUE);
    }
}
