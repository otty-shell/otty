//! Spend: how many tokens a profile used, by day, by project and by model,
//! each model with the money its agent part gave its answers.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use jiff::civil::Date;

use crate::api_prices::ApiPrices;
use crate::error::SourceError;
use crate::local_time::{LocalNow, is_in_last_days, is_in_month_of};
use crate::money_estimate::MoneyTotal;
use crate::profile::Profile;
use crate::provider::ProviderRef;
use crate::token_count::TokenCount;

/// How many days back the tokens are counted, today included.
pub(crate) const COUNTED_DAYS: i32 = 30;

/// The tokens a profile spent: for each day of the last 30 days (and of the
/// current month, which starts 30 days ago on its 31st day), each split by
/// model, and for each project in the last 30 days.
///
/// The days are in the user's time zone, the one of the [`LocalNow`] the
/// count was given.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Spend {
    days: Vec<DaySpend>,
    projects: Vec<ProjectTokens>,
    hours: Vec<HourSpend>,
}

impl Spend {
    /// The tokens spent today.
    pub fn today(&self) -> TokenCount {
        self.last_days(1)
    }

    /// The tokens spent in the last 7 days, today included.
    pub fn last_7_days(&self) -> TokenCount {
        self.last_days(7)
    }

    /// The tokens spent in the last 30 days, today included.
    pub fn last_30_days(&self) -> TokenCount {
        self.last_days(COUNTED_DAYS)
    }

    /// Each of the last 30 days, oldest first and today last, for the bars
    /// of the dashboard. A day with nothing spent is there with no models.
    pub fn last_30_days_by_day(&self) -> &[DaySpend] {
        let Some(today) = self.today_date() else {
            return &[];
        };

        let first = self
            .days
            .iter()
            .position(|day| is_in_last_days(day.date, today, COUNTED_DAYS))
            .unwrap_or(self.days.len());

        &self.days[first..]
    }

    /// The tokens of each project in the last 30 days, the most first.
    pub fn projects(&self) -> &[ProjectTokens] {
        &self.projects
    }

    /// Each hour of today, from 0:00 to 23:00, each split by project and
    /// model, for the bars of today. Empty when the counter did not split
    /// today by the hour.
    pub fn today_by_hour(&self) -> &[HourSpend] {
        &self.hours
    }

    /// The tokens of each model in the current month, the most first.
    pub fn this_month_by_model(&self) -> Vec<ModelTokens> {
        let Some(today) = self.today_date() else {
            return Vec::new();
        };

        models_of(
            self.days
                .iter()
                .filter(|day| is_in_month_of(day.date, today)),
        )
    }

    /// The tokens of each model in the last 30 days, today included, the
    /// most first: the same days as [`last_30_days`](Self::last_30_days).
    pub fn last_30_days_by_model(&self) -> Vec<ModelTokens> {
        models_of(self.last_30_days_by_day().iter())
    }

    /// Spend from its parts: `days` one entry for each day up to today,
    /// oldest first and today last, and `projects` the most first.
    ///
    /// ```
    /// use otty_agents::{DaySpend, ModelTokens, Spend, TokenCount};
    ///
    /// let today = DaySpend::new(
    ///     jiff::civil::date(2026, 11, 16),
    ///     vec![ModelTokens::new(
    ///         Some("gpt-5"),
    ///         TokenCount::new(10, 0, 2),
    ///         1,
    ///     )],
    /// );
    /// let spend = Spend::new(vec![today], Vec::new());
    ///
    /// assert_eq!(spend.today().output(), 2);
    /// ```
    pub fn new(days: Vec<DaySpend>, projects: Vec<ProjectTokens>) -> Self {
        Self {
            days,
            projects,
            hours: Vec::new(),
        }
    }

    /// The same spend with the tokens of each hour of today, from 0:00 to
    /// 23:00.
    pub fn with_hours(mut self, hours: Vec<HourSpend>) -> Self {
        self.hours = hours;
        self
    }

    /// The tokens of the last `count` days, today included.
    fn last_days(&self, count: i32) -> TokenCount {
        let Some(today) = self.today_date() else {
            return TokenCount::default();
        };

        self.days
            .iter()
            .filter(|day| is_in_last_days(day.date, today, count))
            .fold(TokenCount::default(), |sum, day| sum + day.tokens())
    }

    /// The date of today: the last of the days.
    fn today_date(&self) -> Option<Date> {
        self.days.last().map(|day| day.date)
    }
}

/// The tokens spent on one day, split by model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaySpend {
    date: Date,
    models: Vec<ModelTokens>,
    parts: Vec<ProjectModelTokens>,
}

impl DaySpend {
    /// The date of the day.
    pub fn date(&self) -> Date {
        self.date
    }

    /// The tokens of each model on this day, the most first.
    pub fn models(&self) -> &[ModelTokens] {
        &self.models
    }

    /// The tokens of each model in each project on this day, the most
    /// first: the same tokens as [`models`](Self::models), split by
    /// project too. Empty when the counter did not split them.
    pub fn parts(&self) -> &[ProjectModelTokens] {
        &self.parts
    }

    /// All the tokens of this day.
    pub fn tokens(&self) -> TokenCount {
        total_tokens(&self.models)
    }

    /// The tokens spent on `date`, by model.
    pub fn new(date: Date, models: Vec<ModelTokens>) -> Self {
        Self {
            date,
            models,
            parts: Vec::new(),
        }
    }

    /// The same day with its tokens split by project and model, the most
    /// first.
    pub fn with_parts(mut self, parts: Vec<ProjectModelTokens>) -> Self {
        self.parts = parts;
        self
    }
}

/// The tokens spent in one hour of today, split by project and model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HourSpend {
    hour: i8,
    parts: Vec<ProjectModelTokens>,
}

impl HourSpend {
    /// The hour of the day in the user's time zone, from 0 to 23.
    pub fn hour(&self) -> i8 {
        self.hour
    }

    /// The tokens of each model in each project in this hour, the most
    /// first.
    pub fn parts(&self) -> &[ProjectModelTokens] {
        &self.parts
    }

    /// All the tokens of this hour.
    pub fn tokens(&self) -> TokenCount {
        self.parts
            .iter()
            .fold(TokenCount::default(), |sum, part| sum + part.tokens.tokens)
    }

    /// The tokens spent in the hour `hour` of today, by project and model.
    ///
    /// ```
    /// use otty_agents::{HourSpend, ModelTokens, ProjectModelTokens, TokenCount};
    ///
    /// let tokens = ModelTokens::new(Some("gpt-5"), TokenCount::new(10, 0, 2), 1);
    /// let hour = HourSpend::new(9, vec![ProjectModelTokens::new(None, tokens)]);
    ///
    /// assert_eq!(hour.tokens().total(), 12);
    /// ```
    pub fn new(hour: i8, parts: Vec<ProjectModelTokens>) -> Self {
        Self { hour, parts }
    }
}

/// The tokens one model spent in one project: a part of a day or of an
/// hour, so that the usage can be split by project and by model at
/// once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectModelTokens {
    project: Option<PathBuf>,
    tokens: ModelTokens,
}

impl ProjectModelTokens {
    /// The project folder; `None` when the files did not name it.
    pub fn project(&self) -> Option<&Path> {
        self.project.as_deref()
    }

    /// The tokens of the model, with its provider.
    pub fn tokens(&self) -> &ModelTokens {
        &self.tokens
    }

    /// The `tokens` of one model spent in `project`.
    pub fn new(project: Option<PathBuf>, tokens: ModelTokens) -> Self {
        Self { project, tokens }
    }
}

/// The tokens one model spent, in how many requests, through one
/// provider, and their money as the agent part priced each answer: the
/// cost the agent wrote, else the tokens at API prices (spec
/// 2026-11-23-accounts-apart-from-profiles, Decision 3). Nothing ties
/// them to an account (Decision 1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelTokens {
    model: Option<String>,
    provider: Option<ProviderRef>,
    tokens: TokenCount,
    requests: u64,
    money: MoneyTotal,
}

impl ModelTokens {
    /// The model id as the agent wrote it, such as `example-large`. `None`
    /// when the files did not name the model.
    pub fn model(&self) -> Option<&str> {
        self.model.as_deref()
    }

    /// The provider that served the model: the one the agent wrote, else
    /// the one of the profile's server. `None` when it is not known, such
    /// as a local model or a server OTTY does not know.
    pub fn provider(&self) -> Option<&ProviderRef> {
        self.provider.as_ref()
    }

    /// The tokens.
    pub fn tokens(&self) -> TokenCount {
        self.tokens
    }

    /// How many requests (answers of the model) spent the tokens.
    pub fn requests(&self) -> u64 {
        self.requests
    }

    /// What the tokens cost, as the agent part priced each answer: exact
    /// when the agent wrote every cost, else at API prices, with the
    /// requests that had no price counted apart.
    pub fn money(&self) -> MoneyTotal {
        self.money
    }

    /// The tokens `model` spent in `requests` requests, with no money yet.
    ///
    /// ```
    /// use otty_agents::{ModelTokens, TokenCount};
    ///
    /// let tokens = ModelTokens::new(Some("gpt-5"), TokenCount::new(10, 0, 2), 1);
    ///
    /// assert_eq!(tokens.requests(), 1);
    /// ```
    pub fn new(model: Option<&str>, tokens: TokenCount, requests: u64) -> Self {
        Self {
            model: model.map(String::from),
            provider: None,
            tokens,
            requests,
            money: MoneyTotal::default(),
        }
    }

    /// The same tokens, served by `provider`, or by an unknown one.
    ///
    /// ```
    /// use otty_agents::{ModelTokens, ProviderRef, TokenCount};
    ///
    /// let tokens =
    ///     ModelTokens::new(Some("example-large"), TokenCount::new(10, 0, 2), 1)
    ///         .with_provider(Some(ProviderRef::of_id("example-cloud")));
    ///
    /// assert_eq!(
    ///     tokens.provider(),
    ///     Some(&ProviderRef::of_id("example-cloud"))
    /// );
    /// ```
    pub fn with_provider(mut self, provider: Option<ProviderRef>) -> Self {
        self.provider = provider;
        self
    }

    /// The same tokens, which cost `money` as their agent part priced
    /// them.
    ///
    /// ```
    /// use otty_agents::{Dollars, ModelTokens, MoneyTotal, TokenCount};
    ///
    /// let written = MoneyTotal::written(Dollars::from_micro_dollars(40));
    /// let tokens = ModelTokens::new(Some("gpt-5"), TokenCount::new(10, 0, 2), 1)
    ///     .with_money(written);
    ///
    /// assert_eq!(tokens.money(), written);
    /// ```
    pub fn with_money(mut self, money: MoneyTotal) -> Self {
        self.money = money;
        self
    }

    /// Counts one more request of this model that spent `tokens` and cost
    /// `money` as its part priced it.
    pub(crate) fn count_request(
        &mut self,
        tokens: TokenCount,
        money: MoneyTotal,
    ) {
        self.money = self.money + money;
        self.tokens += tokens;
        self.requests = self.requests.saturating_add(1);
    }

    /// Adds the tokens, requests and money of `other`, which must be of
    /// the same model and provider.
    fn add(&mut self, other: &ModelTokens) {
        self.money = self.money + other.money;
        self.tokens += other.tokens;
        self.requests = self.requests.saturating_add(other.requests);
    }
}

/// The tokens spent in one project folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectTokens {
    folder: PathBuf,
    tokens: TokenCount,
}

impl ProjectTokens {
    /// The project folder.
    pub fn folder(&self) -> &Path {
        &self.folder
    }

    /// The tokens.
    pub fn tokens(&self) -> TokenCount {
        self.tokens
    }

    /// The tokens spent in `folder`.
    pub fn new(folder: PathBuf, tokens: TokenCount) -> Self {
        Self { folder, tokens }
    }
}

/// Counts the tokens a profile spent from the agent's local files.
///
/// It is a trait because each agent writes its numbers its own way, and
/// counting them reads files.
#[cfg_attr(test, mockall::automock)]
pub trait SpendCounter: Send {
    /// The tokens the profile spent up to `now`, by the days of the user's
    /// time zone, each answer priced by the agent part at `prices` when
    /// the agent did not write its cost.
    fn spend(
        &self,
        profile: &Profile,
        now: &LocalNow,
        prices: &Arc<ApiPrices>,
    ) -> Result<Spend, SourceError>;
}

/// Sorts `models` the most tokens first; models with as many tokens by
/// their name, the unnamed one last, then by their provider.
pub(crate) fn sort_models(models: &mut [ModelTokens]) {
    models.sort_by(|a, b| {
        b.tokens
            .total()
            .cmp(&a.tokens.total())
            .then_with(|| a.model.is_none().cmp(&b.model.is_none()))
            .then_with(|| a.model.cmp(&b.model))
            .then_with(|| a.provider.cmp(&b.provider))
    });
}

/// The tokens of every model of `models` together.
pub(crate) fn total_tokens(models: &[ModelTokens]) -> TokenCount {
    models
        .iter()
        .fold(TokenCount::default(), |sum, model| sum + model.tokens)
}

/// The tokens of the same model through the same provider among
/// `models` added up, the most first.
pub(crate) fn merge_models<'a>(
    models: impl Iterator<Item = &'a ModelTokens>,
) -> Vec<ModelTokens> {
    let mut by_model: Vec<ModelTokens> = Vec::new();
    for model in models {
        let same = by_model.iter_mut().find(|kept| {
            kept.model == model.model && kept.provider == model.provider
        });

        match same {
            Some(kept) => kept.add(model),
            None => by_model.push(model.clone()),
        }
    }

    sort_models(&mut by_model);
    by_model
}

/// The tokens of each model over `days`, added up, the most first.
fn models_of<'a>(days: impl Iterator<Item = &'a DaySpend>) -> Vec<ModelTokens> {
    merge_models(days.flat_map(|day| day.models.iter()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dollars::Dollars;
    use crate::money_estimate::MoneyByKind;

    fn tokens(input: u64) -> TokenCount {
        TokenCount::new(input, 0, 0)
    }

    fn day(month: i8, day: i8, models: &[(Option<&str>, u64)]) -> DaySpend {
        let models = models
            .iter()
            .map(|(model, input)| ModelTokens::new(*model, tokens(*input), 1))
            .collect();

        DaySpend::new(jiff::civil::date(2026, month, day), models)
    }

    /// One entry for each day from `first` to `last` of October 2026, each
    /// with `gpt-5` spending the day of the month as input tokens.
    fn october(first: i8, last: i8) -> Vec<DaySpend> {
        (first..=last)
            .map(|number| {
                let input = u64::from(number.unsigned_abs());
                day(10, number, &[(Some("gpt-5"), input)])
            })
            .collect()
    }

    #[test]
    fn today_7_days_and_30_days_add_up_the_last_days() {
        let spend = Spend::new(october(1, 31), Vec::new());

        assert_eq!(spend.today(), tokens(31));
        assert_eq!(spend.last_7_days(), tokens((25..=31).sum()));
        assert_eq!(spend.last_30_days(), tokens((2..=31).sum()));
    }

    #[test]
    fn the_bars_are_the_last_30_days() {
        let spend = Spend::new(october(1, 31), Vec::new());

        let bars = spend.last_30_days_by_day();

        assert_eq!(bars.len(), 30);
        assert_eq!(bars[0].date(), jiff::civil::date(2026, 10, 2));
        assert_eq!(bars[29].date(), jiff::civil::date(2026, 10, 31));
    }

    #[test]
    fn the_month_counts_from_its_first_day_even_31_days_back() {
        let spend = Spend::new(october(1, 31), Vec::new());

        let by_model = spend.this_month_by_model();

        assert_eq!(
            by_model,
            [ModelTokens::new(Some("gpt-5"), tokens((1..=31).sum()), 31)]
        );
    }

    #[test]
    fn the_month_leaves_out_the_days_of_the_month_before() {
        let mut days = october(18, 31);
        days.extend([
            day(11, 1, &[(Some("gpt-5"), 5), (None, 7)]),
            day(11, 2, &[(Some("o3"), 7), (Some("gpt-5"), 1)]),
        ]);
        let spend = Spend::new(days, Vec::new());

        let by_model = spend.this_month_by_model();

        assert_eq!(
            by_model,
            [
                ModelTokens::new(Some("o3"), tokens(7), 1),
                ModelTokens::new(None, tokens(7), 1),
                ModelTokens::new(Some("gpt-5"), tokens(6), 2),
            ]
        );
    }

    /// Spec 2026-11-23-accounts-apart-from-profiles, Decision 3: the money
    /// of a model is the money its part gave each answer, added up.
    #[test]
    fn a_request_adds_its_money_to_the_model() {
        let micro = Dollars::from_micro_dollars;
        let mut model = ModelTokens::new(Some("m"), TokenCount::default(), 0);

        model.count_request(tokens(10), MoneyTotal::written(micro(2)));
        let listed = MoneyByKind::new(micro(3), micro(0), micro(0), micro(0));
        model.count_request(tokens(5), MoneyTotal::at_api_prices(listed, 1));
        model.count_request(tokens(1), MoneyTotal::unpriced(1));

        assert_eq!(model.tokens(), tokens(16));
        assert_eq!(model.requests(), 3);
        assert_eq!(model.money().cost(), micro(5));
        assert_eq!(model.money().unpriced_requests(), 1);
        assert!(model.money().is_estimate());
    }

    #[test]
    fn the_month_adds_up_the_money_of_a_model() {
        let micro = Dollars::from_micro_dollars;
        let costly = |cost| {
            ModelTokens::new(Some("m"), tokens(1), 1)
                .with_money(MoneyTotal::written(micro(cost)))
        };
        let days = vec![
            DaySpend::new(jiff::civil::date(2026, 11, 1), vec![costly(2)]),
            DaySpend::new(jiff::civil::date(2026, 11, 2), vec![costly(5)]),
        ];

        let by_model = Spend::new(days, Vec::new()).this_month_by_model();

        assert_eq!(by_model[0].money(), MoneyTotal::written(micro(7)));
        assert_eq!(by_model[0].requests(), 2);
    }

    #[test]
    fn the_models_of_30_days_reach_into_the_month_before() {
        let mut days = vec![day(10, 3, &[(Some("o3"), 100)])];
        days.extend(october(4, 31));
        days.extend([
            day(11, 1, &[(Some("gpt-5"), 5), (None, 7)]),
            day(11, 2, &[(Some("o3"), 7), (Some("gpt-5"), 1)]),
        ]);
        let spend = Spend::new(days, Vec::new());

        let by_model = spend.last_30_days_by_model();

        let october_tokens: u64 = (4..=31).sum();
        assert_eq!(
            by_model,
            [
                ModelTokens::new(Some("gpt-5"), tokens(october_tokens + 6), 30),
                ModelTokens::new(Some("o3"), tokens(7), 1),
                ModelTokens::new(None, tokens(7), 1),
            ],
            "October 3 is 31 days back and left out; October 4 is in"
        );
    }

    #[test]
    fn a_day_adds_up_its_models() {
        let today = day(11, 16, &[(Some("a"), 3), (Some("b"), 4)]);

        assert_eq!(today.tokens(), tokens(7));
    }

    #[test]
    fn empty_spend_has_no_tokens() {
        let spend = Spend::default();

        assert!(spend.today().is_zero());
        assert!(spend.last_30_days().is_zero());
        assert!(spend.last_30_days_by_day().is_empty());
        assert!(spend.this_month_by_model().is_empty());
        assert!(spend.last_30_days_by_model().is_empty());
    }
}
