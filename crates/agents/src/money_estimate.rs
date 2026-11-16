//! The money a profile billed by an API key spent, estimated from its
//! tokens at list prices (spec Decision 9).

use std::ops::Add;

use crate::dollars::Dollars;
use crate::local_time::CalendarDate;
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
    prices_taken_on: CalendarDate,
    days: Vec<DayMoney>,
    models: Vec<ModelMoney>,
}

impl MoneyEstimate {
    /// The date the prices of the estimate were taken.
    pub fn prices_taken_on(&self) -> CalendarDate {
        self.prices_taken_on
    }

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
        self.models
            .iter()
            .fold(MoneyTotal::default(), |sum, model| sum + model.total())
    }

    /// Each of the last 30 days, oldest first and today last, for the bars
    /// of the dashboard.
    pub fn last_30_days_by_day(&self) -> &[DayMoney] {
        &self.days
    }

    /// The money of each model in the current month, the most first; the
    /// models with no price last.
    pub fn this_month_by_model(&self) -> &[ModelMoney] {
        &self.models
    }

    /// The estimate of the money `spend` cost at the list `prices`.
    ///
    /// ```
    /// use otty_agents::{
    ///     CalendarDate, DaySpend, ModelPrice, ModelTokens, MoneyEstimate,
    ///     PriceTable, Spend, TokenCount,
    /// };
    ///
    /// let today = DaySpend::new(
    ///     CalendarDate::new(2026, 11, 16),
    ///     vec![ModelTokens::new(
    ///         Some("gpt-5"),
    ///         TokenCount::new(1_000_000, 0, 0),
    ///         1,
    ///     )],
    /// );
    /// let spend = Spend::new(vec![today], Vec::new());
    /// let prices = PriceTable::new(
    ///     CalendarDate::new(2026, 11, 16),
    ///     vec![ModelPrice::new(
    ///         "gpt-5", 1_250_000, 125_000, 1_250_000, 10_000_000,
    ///     )],
    /// );
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

        let mut models: Vec<ModelMoney> = spend
            .this_month_by_model()
            .iter()
            .map(|model| ModelMoney::new(model, prices))
            .collect();
        // A stable sort: the models of equal money, and those with no
        // price, keep the order of their tokens.
        models.sort_by_key(|model| std::cmp::Reverse(model.cost));

        Self {
            prices_taken_on: prices.taken_on(),
            days,
            models,
        }
    }

    /// The money of the last `count` days, today included.
    fn last_days(&self, count: i64) -> MoneyTotal {
        let Some(today) = self.days.last().map(|day| day.date.day_number())
        else {
            return MoneyTotal::default();
        };

        self.days
            .iter()
            .filter(|day| day.date.day_number() > today - count)
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

/// The money of one day.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DayMoney {
    date: CalendarDate,
    total: MoneyTotal,
}

impl DayMoney {
    /// The date of the day.
    pub fn date(&self) -> CalendarDate {
        self.date
    }

    /// The money of the day.
    pub fn total(&self) -> MoneyTotal {
        self.total
    }

    /// What the tokens of `day` cost at `prices`.
    fn new(day: &DaySpend, prices: &PriceTable) -> Self {
        let total = day
            .models()
            .iter()
            .map(|model| ModelMoney::new(model, prices).total())
            .fold(MoneyTotal::default(), |sum, model| sum + model);

        Self {
            date: day.date(),
            total,
        }
    }
}

/// The tokens of one model and what they cost.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelMoney {
    tokens: ModelTokens,
    cost: Option<Dollars>,
}

impl ModelMoney {
    /// The model, its tokens and its requests.
    pub fn tokens(&self) -> &ModelTokens {
        &self.tokens
    }

    /// What the tokens cost; `None` when the model has no price.
    pub fn cost(&self) -> Option<Dollars> {
        self.cost
    }

    /// What the tokens of `model` cost at `prices`. A model with no tokens
    /// costs nothing, even with no price: there is nothing to price.
    fn new(model: &ModelTokens, prices: &PriceTable) -> Self {
        let cost = if model.tokens().is_zero() {
            Some(Dollars::default())
        } else {
            model
                .model()
                .and_then(|name| prices.price_of(name))
                .map(|price| price.cost(model))
        };

        Self {
            tokens: model.clone(),
            cost,
        }
    }

    /// The money of this model: its cost, or its requests as having no
    /// price.
    fn total(&self) -> MoneyTotal {
        match self.cost {
            Some(cost) => MoneyTotal {
                cost,
                unpriced_requests: 0,
            },
            None => MoneyTotal {
                cost: Dollars::default(),
                unpriced_requests: self.tokens.requests(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::bundled_prices::bundled_prices;
    use crate::local_time::LocalNow;
    use crate::price_table::ModelPrice;
    use crate::spend_files::{CountedAnswer, FileSpend};
    use crate::spend_tally::tally;
    use crate::testing::at;
    use crate::token_count::TokenCount;

    /// 2026-11-01T10:00:00Z.
    const FIRST_OF_NOVEMBER: u64 = 1_793_527_200;
    /// 2026-11-16T10:00:00Z.
    const NOW: u64 = 1_794_823_200;
    const DAY: u64 = 24 * 3600;

    /// A dollar in micro-dollars.
    const DOLLAR: u64 = 1_000_000;

    /// Made-up prices: `m` costs $1 per million input tokens and $10 per
    /// million output tokens.
    fn prices() -> PriceTable {
        PriceTable::new(
            CalendarDate::new(2026, 11, 2),
            vec![
                ModelPrice::new(
                    "m",
                    DOLLAR,
                    DOLLAR / 10,
                    2 * DOLLAR,
                    10 * DOLLAR,
                ),
                ModelPrice::new("claude-opus-4-5", 5 * DOLLAR, 0, 0, 0),
            ],
        )
    }

    fn million_in(model: Option<&str>, requests: u64) -> ModelTokens {
        ModelTokens::new(model, TokenCount::new(1_000_000, 0, 0), requests)
    }

    fn day(month: u8, day: u8, models: Vec<ModelTokens>) -> DaySpend {
        DaySpend::new(CalendarDate::new(2026, month, day), models)
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
    fn tallied(answers: &[(u64, u64)], now: u64, offset_hours: i32) -> Spend {
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

        tally(files.iter(), LocalNow::new(at(now), offset_hours * 3600))
    }

    #[test]
    fn money_of_today_7_days_the_month_and_each_day() {
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

        assert_eq!(money.prices_taken_on(), CalendarDate::new(2026, 11, 2));
        assert_eq!(money.today(), priced(1));
        assert_eq!(money.last_7_days(), priced(2));
        assert_eq!(money.this_month(), priced(4));
        let days: Vec<_> = money
            .last_30_days_by_day()
            .iter()
            .map(|day| (day.date().day(), day.total().cost()))
            .collect();
        assert_eq!(
            days,
            [
                (31, dollars(1)),
                (1, dollars(1)),
                (9, dollars(1)),
                (10, dollars(1)),
                (16, dollars(1)),
            ]
        );
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
        assert_eq!(
            money.this_month_by_model(),
            [
                ModelMoney {
                    tokens: million_in(Some("m"), 1),
                    cost: Some(dollars(1)),
                },
                ModelMoney {
                    tokens: ModelTokens::new(
                        Some("mystery"),
                        TokenCount::new(2_000_000, 0, 0),
                        5
                    ),
                    cost: None,
                },
                ModelMoney {
                    tokens: million_in(None, 1),
                    cost: None,
                },
            ]
        );
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
        assert_eq!(money.this_month_by_model()[0].cost(), Some(dollars(0)));
    }

    #[test]
    fn the_models_are_the_most_money_first() {
        let spend = Spend::new(
            vec![day(
                11,
                16,
                vec![
                    ModelTokens::new(
                        Some("m"),
                        TokenCount::new(2_000_000, 0, 0),
                        1,
                    ),
                    million_in(Some("mystery"), 1),
                    million_in(Some("claude-opus-4-5"), 1),
                ],
            )],
            Vec::new(),
        );

        let money = MoneyEstimate::new(&spend, &prices());

        let models: Vec<_> = money
            .this_month_by_model()
            .iter()
            .map(|model| model.tokens().model())
            .collect();
        assert_eq!(
            models,
            [Some("claude-opus-4-5"), Some("m"), Some("mystery")]
        );
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
    fn on_the_31st_the_month_reaches_past_the_30_bars() {
        // 2026-12-31T12:00:00Z.
        let new_years_eve = 1_798_718_400;
        let first_of_december = new_years_eve - 30 * DAY;
        let answers =
            [(first_of_december, 1_000_000), (new_years_eve, 2_000_000)];

        let money =
            MoneyEstimate::new(&tallied(&answers, new_years_eve, 0), &prices());

        let bars = money.last_30_days_by_day();
        assert_eq!(bars.len(), 30);
        assert_eq!(bars[0].date(), CalendarDate::new(2026, 12, 2));
        assert_eq!(bars[29].total(), priced(2));
        assert_eq!(money.this_month(), priced(3));
    }

    #[test]
    fn a_request_just_over_the_prompt_limit_is_priced_as_a_long_prompt() {
        let request =
            |model, tokens| CountedAnswer::new(at(NOW), Some(model), tokens);
        let files = [FileSpend::new(
            None,
            vec![
                request("claude-haiku-5-5", TokenCount::new(100_000, 0, 0)),
                request("claude-haiku-5-5", TokenCount::new(50_000, 50_001, 0)),
                request("gpt-6.1-sol", TokenCount::new(272_000, 0, 0)),
                request("gpt-6.1-sol", TokenCount::new(272_001, 0, 0)),
            ],
        )];
        let spend = tally(files.iter(), LocalNow::new(at(NOW), 0));

        let money = MoneyEstimate::new(&spend, &bundled_prices());

        let cost = |name: &str| {
            money
                .this_month_by_model()
                .iter()
                .find(|model| model.tokens().model() == Some(name))
                .and_then(ModelMoney::cost)
                .map(|cost| cost.micro_dollars())
        };
        // Haiku: 100,000 at $0.10; then 50,000 at $0.50 and 50,001 cached
        // at $0.05, per million.
        assert_eq!(cost("claude-haiku-5-5"), Some(10_000 + 27_500));
        // GPT-6.1 Sol: 272,000 at $2; then 272,001 at $4, per million.
        assert_eq!(cost("gpt-6.1-sol"), Some(544_000 + 1_088_004));
    }

    #[test]
    fn no_spend_costs_nothing() {
        let money = MoneyEstimate::new(&Spend::default(), &prices());

        assert_eq!(money.today(), MoneyTotal::default());
        assert_eq!(money.last_7_days(), MoneyTotal::default());
        assert_eq!(money.this_month(), MoneyTotal::default());
        assert!(money.last_30_days_by_day().is_empty());
        assert!(money.this_month_by_model().is_empty());
    }
}
