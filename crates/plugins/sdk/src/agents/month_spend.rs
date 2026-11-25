//! What the user spent through an account this calendar month, as a
//! provider part counts it from the agent's own session files (issue 22
//! of spec 2026-11-23-accounts-apart-from-profiles): the money of the
//! answers at API prices, kind by kind, each answer once. The month is the
//! user's own: it starts at midnight of its first day on their clock.

use std::collections::BTreeSet;

use crate::agents::{Answer, AnswerCost, Money, PricedCost};
use crate::clock;

/// Milliseconds in a day.
const DAY_MS: i64 = 86_400_000;

/// Milliseconds in a minute.
const MINUTE_MS: i64 = 60_000;

/// Millionths of a dollar in a dollar.
const MICRO_DOLLARS: f64 = 1_000_000.0;

/// Adds up the money of the answers of one month at API prices, kind by
/// kind: an answer of an earlier time is left out, and one identity
/// counts once, as a resumed session copies the earlier answers into its
/// new file. A cost the agent wrote itself has no kinds and is left out.
///
/// ```
/// use otty_plugin_sdk::agents::MonthSpend;
///
/// let month = MonthSpend::since(1_793_491_200_000);
///
/// assert_eq!(month.cost().map(|cost| cost.total), Some(0));
/// ```
#[derive(Debug, Clone)]
pub struct MonthSpend {
    since_ms: i64,
    seen: BTreeSet<String>,
    cost: PricedCost,
    priced: u64,
    unpriced: u64,
}

impl MonthSpend {
    /// When the month started, in milliseconds since the Unix epoch: a
    /// session file changed before it holds no answer of the month.
    pub fn since_ms(&self) -> i64 {
        self.since_ms
    }

    /// The money of the month's answers kind by kind; `None` when answers
    /// with tokens came and none had a price, such as before OTTY read its
    /// price list, so the money is not known. No answer is nothing spent.
    pub fn cost(&self) -> Option<PricedCost> {
        if self.priced == 0 && self.unpriced > 0 {
            return None;
        }

        Some(self.cost)
    }

    /// [`MonthSpend::cost`] as the money spent this month of an account:
    /// in US dollars, marked at API prices by its kinds.
    pub fn money(&self) -> Option<Money> {
        let cost = self.cost()?;

        Some(Money {
            amount: cost.total as f64 / MICRO_DOLLARS,
            currency: "USD".to_string(),
            by_kind: Some(cost),
        })
    }

    /// The month the user's clock is in now, with nothing spent yet.
    pub fn this_month() -> Self {
        Self::since(month_start_ms(clock::now_ms()))
    }

    /// A month that started at `since_ms`, with nothing spent yet.
    pub fn since(since_ms: i64) -> Self {
        Self {
            since_ms,
            seen: BTreeSet::new(),
            cost: PricedCost {
                total: 0,
                input: 0,
                cache_written: 0,
                cached: 0,
                output: 0,
            },
            priced: 0,
            unpriced: 0,
        }
    }

    /// Adds the money of `answer`, when it is of the month and its
    /// identity was not added before.
    pub fn add(&mut self, answer: &Answer) {
        if answer.at_ms < self.since_ms {
            return;
        }
        if let Some(identity) = &answer.identity
            && !self.seen.insert(identity.clone())
        {
            return;
        }

        match &answer.cost {
            Some(AnswerCost::AtApiPrices(cost)) => {
                self.cost = added(&self.cost, cost);
                self.priced += 1;
            },
            None if has_tokens(answer) => self.unpriced += 1,
            _ => {},
        }
    }
}

/// When the month of `now_ms` started on the user's clock: midnight of
/// its first day there. The offset is taken again at that time, which
/// summer time may give another one.
fn month_start_ms(now_ms: i64) -> i64 {
    let local_now = now_ms + offset_ms(now_ms);
    let day = local_now.div_euclid(DAY_MS);
    let local_start = (day - day_of_month(day) + 1) * DAY_MS;

    let near_start = local_start - offset_ms(now_ms);
    local_start - offset_ms(near_start)
}

/// How far the user's clock is ahead of UTC at `at_ms`, in milliseconds.
fn offset_ms(at_ms: i64) -> i64 {
    i64::from(clock::local_offset_minutes(at_ms)) * MINUTE_MS
}

/// The day of the month, from 1, of the day `days` after 1970-01-01, by
/// the civil calendar (the days-to-date rule of Howard Hinnant).
fn day_of_month(days: i64) -> i64 {
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted - era * 146_097;
    let year_of_era = (day_of_era - day_of_era / 1_460 + day_of_era / 36_524
        - day_of_era / 146_096)
        / 365;
    let day_of_year =
        day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_from_march = (5 * day_of_year + 2) / 153;

    day_of_year - (153 * month_from_march + 2) / 5 + 1
}

/// `sum` and `cost` added kind by kind.
fn added(sum: &PricedCost, cost: &PricedCost) -> PricedCost {
    PricedCost {
        total: sum.total.saturating_add(cost.total),
        input: sum.input.saturating_add(cost.input),
        cache_written: sum.cache_written.saturating_add(cost.cache_written),
        cached: sum.cached.saturating_add(cost.cached),
        output: sum.output.saturating_add(cost.output),
    }
}

/// Whether `answer` has any tokens: an answer with none costs nothing,
/// priced or not.
fn has_tokens(answer: &Answer) -> bool {
    let tokens = &answer.tokens;

    tokens.input > 0 || tokens.cached > 0 || tokens.output > 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::TokenCount;
    use crate::test_host::TestHost;
    use crate::testing::MadeUpPlugin;

    /// The provider part of the made-up plugin.
    const PROVIDER: &str = "made-up-cloud";

    /// An hour in milliseconds.
    const HOUR: i64 = 3_600_000;

    /// 2026-11-20 at 12:00 UTC.
    const NOV_20_NOON: i64 = 1_795_176_000_000;

    /// 2026-11-01 at 00:00 UTC.
    const NOV_1: i64 = 1_793_491_200_000;

    /// 2026-11-16 at 12:00 UTC.
    const NOV_16_NOON: i64 = 1_794_830_400_000;

    /// One answer of `at_ms` with the identity `identity`, priced at API
    /// prices kind by kind, in millionths of a dollar.
    fn answer(identity: &str, at_ms: i64, cost: Option<AnswerCost>) -> Answer {
        Answer {
            identity: Some(identity.to_string()),
            session: None,
            at_ms,
            model: Some("m".to_string()),
            provider: None,
            project: None,
            tokens: TokenCount {
                input: 10,
                cached: 0,
                cache_written: 0,
                output: 5,
                reasoning: 0,
            },
            cost,
        }
    }

    fn priced(
        input: u64,
        cache_written: u64,
        cached: u64,
        output: u64,
    ) -> PricedCost {
        PricedCost {
            total: input + cache_written + cached + output,
            input,
            cache_written,
            cached,
            output,
        }
    }

    fn at_api_prices(cost: PricedCost) -> Option<AnswerCost> {
        Some(AnswerCost::AtApiPrices(cost))
    }

    /// The host of the made-up plugin, kept with its folder, its clock at
    /// `now_ms` and the user's time zone `offset_minutes` east of UTC.
    fn host_at(now_ms: i64, offset_minutes: i32) -> (MadeUpPlugin, TestHost) {
        let plugin = MadeUpPlugin::new();
        let host = TestHost::new(plugin.folder())
            .with_now_ms(now_ms)
            .with_local_offset_minutes(offset_minutes);

        (plugin, host)
    }

    /// Owner, 2026-11-25: "this month" is the user's calendar month.
    #[test]
    fn the_month_starts_at_midnight_of_its_first_day_where_the_user_is() {
        let leap_day = 1_709_200_800_000;
        let february = 1_706_745_600_000;
        let cases = [
            (NOV_20_NOON, 0, NOV_1),
            (NOV_1, 0, NOV_1),
            (NOV_1 - 1_000, 0, 1_790_812_800_000),
            (leap_day, 0, february),
            // In Moscow, three hours east, November starts on October 31 at
            // 21:00 UTC, and 22:00 UTC of that day is November already.
            (NOV_1 - 2 * HOUR, 180, NOV_1 - 3 * HOUR),
            // In New York, five hours west, 03:00 UTC of November 1 is
            // still October.
            (NOV_1 + 3 * HOUR, -300, 1_790_812_800_000 + 5 * HOUR),
        ];

        for (now_ms, offset, start) in cases {
            let (_plugin, host) = host_at(now_ms, offset);

            let month = host.as_provider(PROVIDER, MonthSpend::this_month);

            assert_eq!(month.since_ms(), start, "{now_ms} at {offset}");
        }
    }

    #[test]
    fn the_answers_of_the_month_add_up_kind_by_kind() {
        let mut month = MonthSpend::since(NOV_1);

        month.add(&answer("a", NOV_16_NOON, at_api_prices(priced(1, 2, 3, 4))));
        month.add(&answer("b", NOV_1, at_api_prices(priced(10, 20, 30, 40))));

        assert_eq!(month.cost(), Some(priced(11, 22, 33, 44)));
    }

    #[test]
    fn an_answer_of_another_month_is_left_out() {
        let mut month = MonthSpend::since(NOV_1);

        month.add(&answer("a", NOV_1 - 1, at_api_prices(priced(5, 0, 0, 0))));
        month.add(&answer("b", NOV_16_NOON, at_api_prices(priced(1, 0, 0, 0))));

        assert_eq!(month.cost(), Some(priced(1, 0, 0, 0)));
    }

    /// A resumed session copies the earlier answers into its new file:
    /// one identity counts once.
    #[test]
    fn an_answer_written_twice_counts_once() {
        let mut month = MonthSpend::since(NOV_1);
        let copy = answer("a", NOV_16_NOON, at_api_prices(priced(1, 0, 0, 2)));

        month.add(&copy);
        month.add(&copy);

        assert_eq!(month.cost(), Some(priced(1, 0, 0, 2)));
    }

    #[test]
    fn no_answer_this_month_is_nothing_spent() {
        let month = MonthSpend::since(NOV_1);

        assert_eq!(month.cost(), Some(priced(0, 0, 0, 0)));
    }

    /// With no price for any answer, such as before OTTY read its price
    /// list, the month's money is not known.
    #[test]
    fn with_no_price_for_any_answer_the_money_is_not_known() {
        let mut none = MonthSpend::since(NOV_1);
        none.add(&answer("a", NOV_16_NOON, None));

        let mut some = MonthSpend::since(NOV_1);
        some.add(&answer("a", NOV_16_NOON, None));
        some.add(&answer("b", NOV_16_NOON, at_api_prices(priced(1, 0, 0, 0))));

        assert_eq!(none.cost(), None);
        assert_eq!(some.cost(), Some(priced(1, 0, 0, 0)));
    }
}
