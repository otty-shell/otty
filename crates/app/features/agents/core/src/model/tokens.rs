use otty_agents::TokenCount;
use otty_app_i18n::{self as i18n, Key};

use super::super::spend::ProfileSpend;

/// The tokens a profile spent as the details show them, as plain facts:
/// today and the last 30 days, each as tokens in, cached and out. The
/// dashboard of issue 16 draws the rest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TokensViewModel {
    /// The tokens of today, such as "12.3K in · 1.2M cached · 4.5K out".
    pub(crate) today: Option<String>,
    /// The tokens of the last 30 days, today included.
    pub(crate) last_30_days: Option<String>,
    /// Why the latest count failed; an earlier count may still show.
    pub(crate) failure: Option<String>,
}

impl TokensViewModel {
    /// The tokens of a profile; `None` while there is nothing to show:
    /// nothing was counted yet, or the agent can not count tokens.
    pub(crate) fn from_spend(spend: &ProfileSpend) -> Option<Self> {
        if !spend.has_counter() {
            return None;
        }

        let last_good = spend.last_good().map(|(spend, _)| spend);
        let failure = spend.failure().map(|error| {
            i18n::with_error(Key::TplAgentsTokensFailed, &error.to_string())
        });
        if last_good.is_none() && failure.is_none() {
            return None;
        }

        Some(Self {
            today: last_good.map(|spend| tokens_text(spend.today())),
            last_30_days: last_good
                .map(|spend| tokens_text(spend.last_30_days())),
            failure,
        })
    }
}

/// `count` as one line: tokens in, cached and out, each written short.
fn tokens_text(count: TokenCount) -> String {
    i18n::agents_tokens(
        &short_number(count.input()),
        &short_number(count.cached()),
        &short_number(count.output()),
    )
}

/// A number written short: as it is below a thousand, else in thousands,
/// millions or billions with one decimal below 100 of them, such as
/// "999", "12.3K", "456K" and "1.2M".
fn short_number(number: u64) -> String {
    const UNITS: [(u64, &str); 3] =
        [(1_000_000_000, "B"), (1_000_000, "M"), (1_000, "K")];

    let Some((size, unit)) =
        UNITS.into_iter().find(|(size, _)| number >= *size)
    else {
        return format!("{number}");
    };

    let tenths = number / (size / 10);
    let whole = tenths / 10;
    let decimal = tenths % 10;
    if whole >= 100 || decimal == 0 {
        format!("{whole}{unit}")
    } else {
        format!("{whole}.{decimal}{unit}")
    }
}

#[cfg(test)]
mod tests {
    use otty_agents::{
        CalendarDate, DaySpend, ModelTokens, SourceError, Spend,
    };

    use super::*;
    use crate::testing::at;

    fn spend(today: TokenCount, earlier: TokenCount) -> Spend {
        let day = |day: u8, tokens: TokenCount| {
            DaySpend::new(
                CalendarDate::new(2026, 11, day),
                vec![ModelTokens::new(Some("gpt-5"), tokens)],
            )
        };

        Spend::new(vec![day(1, earlier), day(16, today)], Vec::new())
    }

    #[test]
    fn numbers_are_written_short() {
        let cases = [
            (0, "0"),
            (999, "999"),
            (1_000, "1K"),
            (12_345, "12.3K"),
            (99_960, "99.9K"),
            (456_789, "456K"),
            (1_250_000, "1.2M"),
            (3_000_000_000, "3B"),
        ];

        for (number, text) in cases {
            assert_eq!(short_number(number), text, "{number}");
        }
    }

    #[test]
    fn today_and_the_30_days_show_in_cached_and_out() {
        let mut counted = ProfileSpend::default();
        counted.apply(
            Some(Ok(spend(
                TokenCount::new(12_345, 1_250_000, 456),
                TokenCount::new(1_000, 0, 44),
            ))),
            at(10),
        );

        let vm = TokensViewModel::from_spend(&counted);

        assert_eq!(
            vm,
            Some(TokensViewModel {
                today: Some(i18n::agents_tokens("12.3K", "1.2M", "456")),
                last_30_days: Some(i18n::agents_tokens("13.3K", "1.2M", "500")),
                failure: None,
            })
        );
    }

    #[test]
    fn a_failed_count_says_why_and_keeps_the_last_good_one() {
        let error = SourceError::NotAvailable(String::from("no folder"));
        let mut never_counted = ProfileSpend::default();
        never_counted.apply(Some(Err(error.clone())), at(10));
        let mut counted_before = ProfileSpend::default();
        counted_before.apply(
            Some(Ok(spend(TokenCount::new(1, 0, 0), TokenCount::default()))),
            at(10),
        );
        counted_before.apply(Some(Err(error.clone())), at(20));

        let never = TokensViewModel::from_spend(&never_counted);
        let before = TokensViewModel::from_spend(&counted_before);

        let failure = i18n::with_error(
            Key::TplAgentsTokensFailed,
            "not available: no folder",
        );
        assert_eq!(
            never,
            Some(TokensViewModel {
                today: None,
                last_30_days: None,
                failure: Some(failure.clone()),
            })
        );
        let before = before.expect("the last good count shows");
        assert_eq!(before.today, Some(i18n::agents_tokens("1", "0", "0")));
        assert_eq!(before.failure, Some(failure));
    }

    #[test]
    fn nothing_shows_before_a_count_or_without_a_counter() {
        let mut no_counter = ProfileSpend::default();
        no_counter.apply(None, at(10));

        assert_eq!(TokensViewModel::from_spend(&ProfileSpend::default()), None);
        assert_eq!(TokensViewModel::from_spend(&no_counter), None);
    }
}
