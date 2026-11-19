//! Spend: how many tokens a profile used, by day, by project and by model.
//! Tokens only, no money.

use std::path::{Path, PathBuf};

use jiff::civil::Date;

use crate::account::AccountIdentity;
use crate::error::SourceError;
use crate::local_time::{LocalNow, is_in_last_days, is_in_month_of};
use crate::profile::Profile;
use crate::provider::Provider;
use crate::token_count::TokenCount;

/// How many days back the tokens are counted, today included.
pub(crate) const COUNTED_DAYS: i32 = 30;

/// The prompt limits the tokens of long prompts are counted over: the
/// limits of the long prompt prices the LiteLLM price table has
/// (`input_cost_per_token_above_200k_tokens` and the like), in tokens. A
/// price with another limit is not used (see [`ModelTokens`]): its model
/// is priced at its standard prices for every request. `litellm.rs` reads
/// those limits from the file, so a new limit there must be added here.
pub(crate) const PROMPT_LIMITS: [u64; 7] =
    [32_000, 100_000, 128_000, 200_000, 256_000, 272_000, 512_000];

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

    /// Every day counted, oldest first and today last: the last 30 days
    /// and the days of the current month before them.
    pub(crate) fn days(&self) -> &[DaySpend] {
        &self.days
    }

    /// The tokens of each project in the last 30 days, the most first.
    pub fn projects(&self) -> &[ProjectTokens] {
        &self.projects
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
        Self { days, projects }
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

    /// All the tokens of this day.
    pub fn tokens(&self) -> TokenCount {
        total_tokens(&self.models)
    }

    /// The tokens spent on `date`, by model.
    pub fn new(date: Date, models: Vec<ModelTokens>) -> Self {
        Self { date, models }
    }
}

/// The tokens one model spent, and in how many requests, through one
/// provider and, when the agent writes it, one account (spec Decision 4).
///
/// Some models cost more for a request with a long prompt, one over a
/// limit their prices name. The counter does not know the prices, so it
/// keeps, for each of the [`PROMPT_LIMITS`], the part of the tokens spent
/// in requests whose prompt was over it; the prices pick their limit from
/// these with [`long_prompts_over`](Self::long_prompts_over).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelTokens {
    model: Option<String>,
    provider: Option<Provider>,
    account: Option<AccountIdentity>,
    tokens: TokenCount,
    requests: u64,
    long_prompts: [TokenCount; PROMPT_LIMITS.len()],
}

impl ModelTokens {
    /// The model id as the agent wrote it, such as `claude-opus-4-5` or
    /// `gpt-5-codex`. `None` when the files did not name the model.
    pub fn model(&self) -> Option<&str> {
        self.model.as_deref()
    }

    /// The provider that served the model: the one the agent wrote, else
    /// the one of the profile's login or server. `None` when it is not
    /// known, such as a local model or a server OTTY does not know.
    pub fn provider(&self) -> Option<Provider> {
        self.provider
    }

    /// The account that paid, when the agent writes it, as Codex does for
    /// each session. `None` when the agent does not tell.
    pub fn account(&self) -> Option<&AccountIdentity> {
        self.account.as_ref()
    }

    /// The tokens.
    pub fn tokens(&self) -> TokenCount {
        self.tokens
    }

    /// How many requests (answers of the model) spent the tokens.
    pub fn requests(&self) -> u64 {
        self.requests
    }

    /// The part of [`tokens`](Self::tokens) spent in requests whose prompt
    /// was over `limit` tokens; `None` when `limit` is not one of the
    /// [`PROMPT_LIMITS`] the tokens are counted over.
    ///
    /// ```
    /// use otty_agents::{ModelTokens, TokenCount};
    ///
    /// let tokens = ModelTokens::new(Some("m"), TokenCount::new(9, 0, 0), 1);
    ///
    /// assert_eq!(
    ///     tokens.long_prompts_over(272_000),
    ///     Some(TokenCount::default())
    /// );
    /// assert_eq!(tokens.long_prompts_over(1_000), None);
    /// ```
    pub fn long_prompts_over(&self, limit: u64) -> Option<TokenCount> {
        let index =
            PROMPT_LIMITS.iter().position(|counted| *counted == limit)?;

        self.long_prompts.get(index).copied()
    }

    /// The tokens `model` spent in `requests` requests.
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
            account: None,
            tokens,
            requests,
            long_prompts: [TokenCount::default(); PROMPT_LIMITS.len()],
        }
    }

    /// The same tokens, served by `provider`, or by an unknown one.
    ///
    /// ```
    /// use otty_agents::{ModelTokens, Provider, TokenCount};
    ///
    /// let tokens =
    ///     ModelTokens::new(Some("deepseek-chat"), TokenCount::new(10, 0, 2), 1)
    ///         .with_provider(Some(Provider::DeepSeek));
    ///
    /// assert_eq!(tokens.provider(), Some(Provider::DeepSeek));
    /// assert_eq!(tokens.account(), None);
    /// ```
    pub fn with_provider(mut self, provider: Option<Provider>) -> Self {
        self.provider = provider;
        self
    }

    /// The same tokens, paid by `account`, or by an account not told.
    pub fn with_account(mut self, account: Option<AccountIdentity>) -> Self {
        self.account = account;
        self
    }

    /// The same tokens, with `long_prompts` of them spent in requests whose
    /// prompt was over `limit` tokens. A `limit` that is not one of the
    /// [`PROMPT_LIMITS`] changes nothing.
    ///
    /// ```
    /// use otty_agents::{ModelTokens, TokenCount};
    ///
    /// let tokens = ModelTokens::new(
    ///     Some("gpt-6.1-sol"),
    ///     TokenCount::new(500_000, 0, 9),
    ///     2,
    /// )
    /// .with_long_prompts(272_000, TokenCount::new(300_000, 0, 5));
    ///
    /// assert_eq!(
    ///     tokens.long_prompts_over(272_000).map(|long| long.input()),
    ///     Some(300_000)
    /// );
    /// ```
    pub fn with_long_prompts(
        mut self,
        limit: u64,
        long_prompts: TokenCount,
    ) -> Self {
        let index = PROMPT_LIMITS.iter().position(|counted| *counted == limit);
        if let Some(kept) =
            index.and_then(|index| self.long_prompts.get_mut(index))
        {
            *kept = long_prompts;
        }

        self
    }

    /// Counts one more request of this model that spent `tokens` with a
    /// prompt of `prompt` tokens.
    pub(crate) fn count_request(&mut self, tokens: TokenCount, prompt: u64) {
        self.tokens += tokens;
        self.requests = self.requests.saturating_add(1);

        for (limit, kept) in PROMPT_LIMITS.iter().zip(&mut self.long_prompts) {
            if prompt > *limit {
                *kept += tokens;
            }
        }
    }

    /// Adds the tokens and requests of `other`, which must be of the same
    /// model, provider and account.
    fn add(&mut self, other: &ModelTokens) {
        self.tokens += other.tokens;
        self.requests = self.requests.saturating_add(other.requests);
        for (kept, added) in
            self.long_prompts.iter_mut().zip(other.long_prompts)
        {
            *kept += added;
        }
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
    /// time zone.
    fn spend(
        &self,
        profile: &Profile,
        now: &LocalNow,
    ) -> Result<Spend, SourceError>;
}

/// Sorts `models` the most tokens first; models with as many tokens by
/// their name, the unnamed one last, then by their provider and account.
pub(crate) fn sort_models(models: &mut [ModelTokens]) {
    models.sort_by(|a, b| {
        b.tokens
            .total()
            .cmp(&a.tokens.total())
            .then_with(|| a.model.is_none().cmp(&b.model.is_none()))
            .then_with(|| a.model.cmp(&b.model))
            .then_with(|| a.provider.cmp(&b.provider))
            .then_with(|| a.account.cmp(&b.account))
    });
}

/// The tokens of every model of `models` together.
pub(crate) fn total_tokens(models: &[ModelTokens]) -> TokenCount {
    models
        .iter()
        .fold(TokenCount::default(), |sum, model| sum + model.tokens)
}

/// The tokens of the same model through the same provider and account
/// among `models` added up, the most first.
pub(crate) fn merge_models<'a>(
    models: impl Iterator<Item = &'a ModelTokens>,
) -> Vec<ModelTokens> {
    let mut by_model: Vec<ModelTokens> = Vec::new();
    for model in models {
        let same = by_model.iter_mut().find(|kept| {
            kept.model == model.model
                && kept.provider == model.provider
                && kept.account == model.account
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

    #[test]
    fn the_month_adds_up_the_long_prompts_of_a_model() {
        let long = |count| {
            ModelTokens::new(Some("m"), tokens(count), 1)
                .with_long_prompts(272_000, tokens(count))
        };
        let days = vec![
            DaySpend::new(jiff::civil::date(2026, 11, 1), vec![long(5)]),
            DaySpend::new(jiff::civil::date(2026, 11, 2), vec![long(7)]),
        ];

        let by_model = Spend::new(days, Vec::new()).this_month_by_model();

        assert_eq!(by_model[0].long_prompts_over(272_000), Some(tokens(12)));
        assert_eq!(by_model[0].requests(), 2);
    }

    #[test]
    fn a_request_counts_its_tokens_over_each_prompt_limit_it_passes() {
        let mut model = ModelTokens::new(Some("m"), TokenCount::default(), 0);

        model.count_request(tokens(10), 150_000);
        model.count_request(tokens(20), 300_000);
        model.count_request(tokens(40), 100_000);

        assert_eq!(model.tokens(), tokens(70));
        assert_eq!(model.long_prompts_over(32_000), Some(tokens(70)));
        assert_eq!(model.long_prompts_over(100_000), Some(tokens(30)));
        assert_eq!(model.long_prompts_over(200_000), Some(tokens(20)));
        assert_eq!(model.long_prompts_over(272_000), Some(tokens(20)));
        assert_eq!(model.long_prompts_over(512_000), Some(tokens(0)));
    }

    #[test]
    fn a_prompt_limit_that_is_not_counted_has_no_long_prompts() {
        let model = ModelTokens::new(Some("m"), tokens(5), 1)
            .with_long_prompts(150_000, tokens(5));

        assert_eq!(model.long_prompts_over(150_000), None);
        assert_eq!(model.long_prompts_over(128_000), Some(tokens(0)));
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
