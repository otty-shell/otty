//! Spend: how many tokens a profile used, by day, by project and by model.
//! Tokens only, no money.

use std::path::{Path, PathBuf};

use crate::error::SourceError;
use crate::local_time::{CalendarDate, LocalNow};
use crate::profile::Profile;
use crate::token_count::TokenCount;

/// How many days back the tokens are counted, today included.
pub(crate) const COUNTED_DAYS: i64 = 30;

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
        let Some(today) = self.today_number() else {
            return &[];
        };

        let first = self
            .days
            .iter()
            .position(|day| day.date.day_number() > today - COUNTED_DAYS)
            .unwrap_or(self.days.len());

        &self.days[first..]
    }

    /// The tokens of each project in the last 30 days, the most first.
    pub fn projects(&self) -> &[ProjectTokens] {
        &self.projects
    }

    /// The tokens of each model in the current month, the most first.
    pub fn this_month_by_model(&self) -> Vec<ModelTokens> {
        let Some(today) = self.days.last().map(|day| day.date) else {
            return Vec::new();
        };
        let month_start = today.first_of_month();

        let mut by_model: Vec<ModelTokens> = Vec::new();
        for day in self.days.iter().filter(|day| day.date >= month_start) {
            for model in &day.models {
                match by_model.iter_mut().find(|kept| kept.model == model.model)
                {
                    Some(kept) => kept.tokens += model.tokens,
                    None => by_model.push(model.clone()),
                }
            }
        }

        sort_models(&mut by_model);
        by_model
    }

    /// Spend from its parts: `days` one entry for each day up to today,
    /// oldest first and today last, and `projects` the most first.
    ///
    /// ```
    /// use otty_agents::{CalendarDate, DaySpend, ModelTokens, Spend, TokenCount};
    ///
    /// let today = DaySpend::new(
    ///     CalendarDate::new(2026, 11, 16),
    ///     vec![ModelTokens::new(Some("gpt-5"), TokenCount::new(10, 0, 2))],
    /// );
    /// let spend = Spend::new(vec![today], Vec::new());
    ///
    /// assert_eq!(spend.today().output(), 2);
    /// ```
    pub fn new(days: Vec<DaySpend>, projects: Vec<ProjectTokens>) -> Self {
        Self { days, projects }
    }

    /// The tokens of the last `count` days, today included.
    fn last_days(&self, count: i64) -> TokenCount {
        let Some(today) = self.today_number() else {
            return TokenCount::default();
        };

        self.days
            .iter()
            .filter(|day| day.date.day_number() > today - count)
            .fold(TokenCount::default(), |sum, day| sum + day.tokens())
    }

    /// The number of today's day: the last of the days.
    fn today_number(&self) -> Option<i64> {
        self.days.last().map(|day| day.date.day_number())
    }
}

/// The tokens spent on one day, split by model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaySpend {
    date: CalendarDate,
    models: Vec<ModelTokens>,
}

impl DaySpend {
    /// The date of the day.
    pub fn date(&self) -> CalendarDate {
        self.date
    }

    /// The tokens of each model on this day, the most first.
    pub fn models(&self) -> &[ModelTokens] {
        &self.models
    }

    /// All the tokens of this day.
    pub fn tokens(&self) -> TokenCount {
        self.models
            .iter()
            .fold(TokenCount::default(), |sum, model| sum + model.tokens)
    }

    /// The tokens spent on `date`, by model.
    pub fn new(date: CalendarDate, models: Vec<ModelTokens>) -> Self {
        Self { date, models }
    }
}

/// The tokens one model spent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelTokens {
    model: Option<String>,
    tokens: TokenCount,
}

impl ModelTokens {
    /// The model id as the agent wrote it, such as `claude-opus-4-5` or
    /// `gpt-5-codex`. `None` when the files did not name the model.
    pub fn model(&self) -> Option<&str> {
        self.model.as_deref()
    }

    /// The tokens.
    pub fn tokens(&self) -> TokenCount {
        self.tokens
    }

    /// The tokens `model` spent.
    pub fn new(model: Option<&str>, tokens: TokenCount) -> Self {
        Self {
            model: model.map(String::from),
            tokens,
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
    /// A short name of the counter, shown with its numbers.
    fn name(&self) -> &'static str;

    /// The tokens the profile spent up to `now`, by the days of the user's
    /// time zone.
    fn spend(
        &self,
        profile: &Profile,
        now: LocalNow,
    ) -> Result<Spend, SourceError>;
}

/// Sorts `models` the most tokens first; models with as many tokens by
/// their name, the unnamed one last.
pub(crate) fn sort_models(models: &mut [ModelTokens]) {
    models.sort_by(|a, b| {
        b.tokens
            .total()
            .cmp(&a.tokens.total())
            .then_with(|| a.model.is_none().cmp(&b.model.is_none()))
            .then_with(|| a.model.cmp(&b.model))
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(input: u64) -> TokenCount {
        TokenCount::new(input, 0, 0)
    }

    fn day(month: u8, day: u8, models: &[(Option<&str>, u64)]) -> DaySpend {
        let models = models
            .iter()
            .map(|(model, input)| ModelTokens::new(*model, tokens(*input)))
            .collect();

        DaySpend::new(CalendarDate::new(2026, month, day), models)
    }

    /// One entry for each day from `first` to `last` of October 2026, each
    /// with `gpt-5` spending the day of the month as input tokens.
    fn october(first: u8, last: u8) -> Vec<DaySpend> {
        (first..=last)
            .map(|number| {
                day(10, number, &[(Some("gpt-5"), u64::from(number))])
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
        assert_eq!(bars[0].date(), CalendarDate::new(2026, 10, 2));
        assert_eq!(bars[29].date(), CalendarDate::new(2026, 10, 31));
    }

    #[test]
    fn the_month_counts_from_its_first_day_even_31_days_back() {
        let spend = Spend::new(october(1, 31), Vec::new());

        let by_model = spend.this_month_by_model();

        assert_eq!(
            by_model,
            [ModelTokens::new(Some("gpt-5"), tokens((1..=31).sum()))]
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
                ModelTokens::new(Some("o3"), tokens(7)),
                ModelTokens::new(None, tokens(7)),
                ModelTokens::new(Some("gpt-5"), tokens(6)),
            ]
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
    }
}
