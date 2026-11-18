//! The money a profile billed by an API key spent, estimated from its
//! tokens at list prices (spec Decision 9).

use std::ops::Add;

use jiff::civil::Date;

use crate::dollars::Dollars;
use crate::local_time::is_in_last_days;
use crate::price_table::PriceTable;
use crate::spend::{DaySpend, ModelTokens, Spend};

/// The money a profile spent, estimated from the tokens of its session
/// files at the list prices of a [`PriceTable`]. It is an estimate, not a
/// bill: it is as old as the table's prices, and knows nothing of
/// discounts or of the fast mode. The requests with a long prompt are
/// priced at the long prompt prices of their model.
///
/// A model with no price keeps its tokens and adds nothing to the money;
/// its requests are counted as having no price, so the screen can say the
/// money is at least the sum shown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoneyEstimate {
    days: Vec<DayMoney>,
    this_month: MoneyTotal,
}

impl MoneyEstimate {
    /// The money spent today.
    pub fn today(&self) -> MoneyTotal {
        self.last_days(1)
    }

    /// The money spent in the last 7 days, today included.
    pub fn last_7_days(&self) -> MoneyTotal {
        self.last_days(7)
    }

    /// The money spent in the current month.
    pub fn this_month(&self) -> MoneyTotal {
        self.this_month
    }

    /// The estimate of the money `spend` cost at the list `prices`.
    ///
    /// ```
    /// use otty_agents::{
    ///     DaySpend, ModelPrice, ModelTokens, MoneyEstimate, PriceTable, Spend,
    ///     TokenCount,
    /// };
    ///
    /// let today = DaySpend::new(
    ///     jiff::civil::date(2026, 11, 16),
    ///     vec![ModelTokens::new(
    ///         Some("gpt-5"),
    ///         TokenCount::new(1_000_000, 0, 0),
    ///         1,
    ///     )],
    /// );
    /// let spend = Spend::new(vec![today], Vec::new());
    /// let prices = PriceTable::new(vec![ModelPrice::new(
    ///     "gpt-5", 1_250_000, 125_000, 1_250_000, 10_000_000,
    /// )]);
    ///
    /// let money = MoneyEstimate::new(&spend, &prices);
    ///
    /// assert_eq!(money.today().cost().micro_dollars(), 1_250_000);
    /// ```
    pub fn new(spend: &Spend, prices: &PriceTable) -> Self {
        let days = spend
            .last_30_days_by_day()
            .iter()
            .map(|day| DayMoney::new(day, prices))
            .collect();

        let this_month = spend
            .this_month_by_model()
            .iter()
            .map(|model| model_total(model, prices))
            .fold(MoneyTotal::default(), |sum, model| sum + model);

        Self { days, this_month }
    }

    /// The money of the last `count` days, today included.
    fn last_days(&self, count: i32) -> MoneyTotal {
        let Some(today) = self.days.last().map(|day| day.date) else {
            return MoneyTotal::default();
        };

        self.days
            .iter()
            .filter(|day| is_in_last_days(day.date, today, count))
            .fold(MoneyTotal::default(), |sum, day| sum + day.total)
    }
}

/// The money of a stretch of time: what the priced requests cost, and how
/// many requests had no price.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MoneyTotal {
    cost: Dollars,
    unpriced_requests: u64,
}

impl MoneyTotal {
    /// What the requests with a price cost.
    pub fn cost(&self) -> Dollars {
        self.cost
    }

    /// How many requests had no price: their money is not in
    /// [`cost`](Self::cost).
    pub fn unpriced_requests(&self) -> u64 {
        self.unpriced_requests
    }

    /// What the tokens of `models` cost at the list `prices`, the same way
    /// as [`MoneyEstimate`] prices them: a model with no price adds its
    /// requests as having no price.
    ///
    /// ```
    /// use otty_agents::{
    ///     ModelPrice, ModelTokens, MoneyTotal, PriceTable, TokenCount,
    /// };
    ///
    /// let prices = PriceTable::new(vec![ModelPrice::new(
    ///     "gpt-5", 1_250_000, 125_000, 1_250_000, 10_000_000,
    /// )]);
    /// let models = [
    ///     ModelTokens::new(Some("gpt-5"), TokenCount::new(1_000_000, 0, 0), 1),
    ///     ModelTokens::new(Some("mystery"), TokenCount::new(5, 0, 0), 2),
    /// ];
    ///
    /// let total = MoneyTotal::of_models(&models, &prices);
    ///
    /// assert_eq!(total.cost().micro_dollars(), 1_250_000);
    /// assert_eq!(total.unpriced_requests(), 2);
    /// ```
    pub fn of_models(models: &[ModelTokens], prices: &PriceTable) -> Self {
        models
            .iter()
            .map(|model| model_total(model, prices))
            .fold(Self::default(), |sum, model| sum + model)
    }
}

impl Add for MoneyTotal {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            cost: self.cost + other.cost,
            unpriced_requests: self
                .unpriced_requests
                .saturating_add(other.unpriced_requests),
        }
    }
}

/// The money of one day, kept to add up today and the last 7 days.
#[derive(Debug, Clone, PartialEq, Eq)]
struct DayMoney {
    date: Date,
    total: MoneyTotal,
}

impl DayMoney {
    /// What the tokens of `day` cost at `prices`.
    fn new(day: &DaySpend, prices: &PriceTable) -> Self {
        let total = day
            .models()
            .iter()
            .map(|model| model_total(model, prices))
            .fold(MoneyTotal::default(), |sum, model| sum + model);

        Self {
            date: day.date(),
            total,
        }
    }
}

/// What the tokens of `model` cost at `prices`: their cost, or their
/// requests as having no price. A model with no tokens costs nothing, even
/// with no price: there is nothing to price.
fn model_total(model: &ModelTokens, prices: &PriceTable) -> MoneyTotal {
    if model.tokens().is_zero() {
        return MoneyTotal::default();
    }

    match model.model().and_then(|name| prices.price_of(name)) {
        Some(price) => MoneyTotal {
            cost: price.cost(model),
            unpriced_requests: 0,
        },
        None => MoneyTotal {
            cost: Dollars::default(),
            unpriced_requests: model.requests(),
        },
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::local_time::LocalNow;
    use crate::price_table::ModelPrice;
    use crate::spend_files::{CountedAnswer, FileSpend};
    use crate::spend_tally::tally;
    use crate::testing::{NOW, at, hours_east, utc_now};
    use crate::token_count::TokenCount;

    /// 2026-11-01T10:00:00Z.
    const FIRST_OF_NOVEMBER: u64 = 1_793_527_200;
    const DAY: u64 = 24 * 3600;

    /// A dollar in micro-dollars.
    const DOLLAR: u64 = 1_000_000;

    /// Made-up prices: `m` costs $1 per million input tokens and $10 per
    /// million output tokens.
    fn prices() -> PriceTable {
        PriceTable::new(vec![
            ModelPrice::new("m", DOLLAR, DOLLAR / 10, 2 * DOLLAR, 10 * DOLLAR),
            ModelPrice::new("claude-opus-4-5", 5 * DOLLAR, 0, 0, 0),
        ])
    }

    fn million_in(model: Option<&str>, requests: u64) -> ModelTokens {
        ModelTokens::new(model, TokenCount::new(1_000_000, 0, 0), requests)
    }

    fn day(month: i8, day: i8, models: Vec<ModelTokens>) -> DaySpend {
        DaySpend::new(jiff::civil::date(2026, month, day), models)
    }

    fn dollars(count: u64) -> Dollars {
        Dollars::from_micro_dollars(count * DOLLAR)
    }

    fn priced(count: u64) -> MoneyTotal {
        MoneyTotal {
            cost: dollars(count),
            unpriced_requests: 0,
        }
    }

    /// The tokens `answers` spent, each at its time, counted at `now` in a
    /// time zone `offset_hours` east of UTC.
    fn tallied(answers: &[(u64, u64)], now: u64, offset_hours: i8) -> Spend {
        let answers = answers
            .iter()
            .map(|(time, input)| {
                CountedAnswer::new(
                    at(*time),
                    Some("m"),
                    TokenCount::new(*input, 0, 0),
                )
            })
            .collect();
        let files = [FileSpend::new(Some(PathBuf::from("/a")), answers)];

        tally(
            files.iter(),
            &LocalNow::new(at(now), hours_east(offset_hours)),
        )
    }

    #[test]
    fn models_are_priced_and_a_model_with_no_price_is_counted_apart() {
        let models = [
            million_in(Some("m"), 2),
            million_in(Some("claude-opus-4-5"), 1),
            million_in(None, 3),
            million_in(Some("mystery"), 4),
            ModelTokens::new(Some("mystery"), TokenCount::default(), 9),
        ];

        let total = MoneyTotal::of_models(&models, &prices());

        assert_eq!(
            total,
            MoneyTotal {
                cost: dollars(6),
                unpriced_requests: 7,
            }
        );
    }

    #[test]
    fn money_of_today_7_days_and_the_month() {
        let spend = Spend::new(
            vec![
                day(10, 31, vec![million_in(Some("m"), 1)]),
                day(11, 1, vec![million_in(Some("m"), 1)]),
                day(11, 9, vec![million_in(Some("m"), 1)]),
                day(11, 10, vec![million_in(Some("m"), 1)]),
                day(11, 16, vec![million_in(Some("m"), 2)]),
            ],
            Vec::new(),
        );

        let money = MoneyEstimate::new(&spend, &prices());

        assert_eq!(money.today(), priced(1));
        assert_eq!(money.last_7_days(), priced(2));
        assert_eq!(money.this_month(), priced(4));
    }

    #[test]
    fn a_dated_snapshot_and_cache_reads_are_priced() {
        let tokens = TokenCount::new(1_000_000, 1_000_000, 0);
        let spend = Spend::new(
            vec![day(
                11,
                16,
                vec![
                    ModelTokens::new(Some("m"), tokens, 1),
                    million_in(Some("claude-opus-4-5-20251101"), 1),
                ],
            )],
            Vec::new(),
        );

        let money = MoneyEstimate::new(&spend, &prices());

        // `m`: $1 of input and $0.10 of cache reads; Opus: $5 of input.
        assert_eq!(
            money.today().cost(),
            Dollars::from_micro_dollars(6_100_000)
        );
    }

    #[test]
    fn a_model_with_no_price_keeps_its_tokens_and_adds_nothing() {
        let spend = Spend::new(
            vec![
                day(
                    11,
                    15,
                    vec![million_in(Some("mystery"), 3), million_in(None, 1)],
                ),
                day(
                    11,
                    16,
                    vec![
                        million_in(Some("mystery"), 2),
                        million_in(Some("m"), 1),
                    ],
                ),
            ],
            Vec::new(),
        );

        let money = MoneyEstimate::new(&spend, &prices());

        let today = MoneyTotal {
            cost: dollars(1),
            unpriced_requests: 2,
        };
        assert_eq!(money.today(), today);
        assert_eq!(money.this_month().cost(), dollars(1));
        assert_eq!(money.this_month().unpriced_requests(), 6);
    }

    #[test]
    fn a_model_with_no_tokens_costs_nothing_even_with_no_price() {
        let spend = Spend::new(
            vec![day(
                11,
                16,
                vec![ModelTokens::new(
                    Some("<synthetic>"),
                    TokenCount::default(),
                    4,
                )],
            )],
            Vec::new(),
        );

        let money = MoneyEstimate::new(&spend, &prices());

        assert_eq!(money.today(), MoneyTotal::default());
        assert_eq!(money.this_month(), MoneyTotal::default());
    }

    #[test]
    fn the_day_and_the_month_follow_the_users_midnight() {
        // 22:30 UTC on October 31: still October in UTC, already
        // November 1 in Moscow (UTC+3).
        let late_on_halloween = FIRST_OF_NOVEMBER - 11 * 3600 - 1800;
        let answers = [(late_on_halloween, 1_000_000)];

        let in_utc = MoneyEstimate::new(
            &tallied(&answers, FIRST_OF_NOVEMBER, 0),
            &prices(),
        );
        let in_moscow = MoneyEstimate::new(
            &tallied(&answers, FIRST_OF_NOVEMBER, 3),
            &prices(),
        );

        assert_eq!(in_utc.today(), MoneyTotal::default());
        assert_eq!(in_utc.this_month(), MoneyTotal::default());
        assert_eq!(in_utc.last_7_days(), priced(1));
        assert_eq!(in_moscow.today(), priced(1));
        assert_eq!(in_moscow.this_month(), priced(1));
    }

    #[test]
    fn on_the_31st_the_month_reaches_past_the_30_days() {
        // 2026-12-31T12:00:00Z.
        let new_years_eve = 1_798_718_400;
        let first_of_december = new_years_eve - 30 * DAY;
        let answers =
            [(first_of_december, 1_000_000), (new_years_eve, 2_000_000)];

        let money =
            MoneyEstimate::new(&tallied(&answers, new_years_eve, 0), &prices());

        assert_eq!(money.days.len(), 30);
        assert_eq!(money.today(), priced(2));
        assert_eq!(money.this_month(), priced(3));
    }

    /// Made-up prices of models that cost more for a long prompt, as the
    /// LiteLLM table has them: Haiku over 100,000 tokens, Sol over
    /// 272,000.
    fn long_prompt_prices() -> PriceTable {
        PriceTable::new(vec![
            ModelPrice::new(
                "claude-haiku-5-5",
                100_000,
                10_000,
                125_000,
                500_000,
            )
            .with_long_prompt(100_000, 500_000, 50_000, 625_000, 2_500_000),
            ModelPrice::new(
                "gpt-6.1-sol",
                2_000_000,
                100_000,
                2_500_000,
                10_000_000,
            )
            .with_long_prompt(
                272_000, 4_000_000, 200_000, 5_000_000, 15_000_000,
            ),
        ])
    }

    #[test]
    fn a_request_just_over_the_prompt_limit_is_priced_as_a_long_prompt() {
        let cost = |model: &str, first: TokenCount, second: TokenCount| {
            let files = [FileSpend::new(
                None,
                vec![
                    CountedAnswer::new(at(NOW), Some(model), first),
                    CountedAnswer::new(at(NOW), Some(model), second),
                ],
            )];
            let spend = tally(files.iter(), &utc_now());
            let money = MoneyEstimate::new(&spend, &long_prompt_prices());
            money.this_month().cost().micro_dollars()
        };

        // Haiku: 100,000 at $0.10; then 50,000 at $0.50 and 50,001 cached
        // at $0.05, per million.
        assert_eq!(
            cost(
                "claude-haiku-5-5",
                TokenCount::new(100_000, 0, 0),
                TokenCount::new(50_000, 50_001, 0)
            ),
            10_000 + 27_500
        );
        // GPT-6.1 Sol: 272,000 at $2; then 272,001 at $4, per million.
        assert_eq!(
            cost(
                "gpt-6.1-sol",
                TokenCount::new(272_000, 0, 0),
                TokenCount::new(272_001, 0, 0)
            ),
            544_000 + 1_088_004
        );
    }

    #[test]
    fn no_spend_costs_nothing() {
        let money = MoneyEstimate::new(&Spend::default(), &prices());

        assert_eq!(money.today(), MoneyTotal::default());
        assert_eq!(money.last_7_days(), MoneyTotal::default());
        assert_eq!(money.this_month(), MoneyTotal::default());
    }
}
