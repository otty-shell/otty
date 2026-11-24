//! The numbers of the usage block (spec 2026-11-19, Decision 6): the
//! tokens of a period of one profile or of several, one point per hour of
//! today or per day, in one project or in all, added up by project, by
//! profile or by model, the tokens and their money side by side. The money
//! of every profile counts, on a subscription too, as its agent part
//! priced each answer:
//! the cost the agent wrote, else the tokens at API prices (spec
//! 2026-11-23-accounts-apart-from-profiles, Decision 3). The block only
//! adds it up.

use std::path::Path;

use otty_agents::{DaySpend, HourSpend, MoneyTotal, ProjectModelTokens, Spend};
use otty_app_domain::folder_path::folder_name;

use super::super::usage::{UsagePeriod, UsageTab};
use super::UNKNOWN;

/// Days in the period of 7 days.
const SEVEN_DAYS: usize = 7;
/// Days in the period of 30 days.
const THIRTY_DAYS: usize = 30;

/// One row of the table of the usage block: a project, a profile or a
/// model, with its tokens and their money at each point of the period.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UsageGroup {
    /// The name of the row, such as "otty" or "example-large".
    pub(crate) name: String,
    /// Its tokens at each point, oldest first.
    pub(crate) tokens: Vec<u64>,
    /// The money of those tokens at each point, oldest first, as their
    /// agent parts priced them.
    pub(crate) money: Vec<MoneyTotal>,
}

impl UsageGroup {
    /// Its tokens over the whole period.
    pub(crate) fn total_tokens(&self) -> u64 {
        self.tokens.iter().sum()
    }

    /// The money of its tokens over the whole period.
    pub(crate) fn total_money(&self) -> MoneyTotal {
        self.money
            .iter()
            .fold(MoneyTotal::default(), |sum, money| sum + *money)
    }

    /// A row of `count` points with nothing spent yet.
    fn empty(name: String, count: usize) -> Self {
        Self {
            name,
            tokens: vec![0; count],
            money: vec![MoneyTotal::default(); count],
        }
    }

    /// Add the tokens and the money of `part` at the point `index`.
    fn add(&mut self, index: usize, part: &ProjectModelTokens) {
        if let Some(slot) = self.tokens.get_mut(index) {
            *slot += part.tokens().tokens().total();
        }
        if let Some(slot) = self.money.get_mut(index) {
            *slot = *slot + part.tokens().money();
        }
    }
}

/// The tokens of one profile that a usage block adds up: the profile's
/// own count, or those of one of its sessions.
#[derive(Debug, Clone, Copy)]
pub(crate) struct UsageSource<'a> {
    /// The profile name, the row of By profile.
    pub(crate) name: &'a str,
    /// The tokens by day and today by the hour.
    pub(crate) spend: &'a Spend,
}

/// What a usage block is counted from.
#[derive(Debug, Clone, Copy)]
pub(crate) struct UsageInput<'a> {
    /// The tokens of each profile.
    pub(crate) sources: &'a [UsageSource<'a>],
    /// The project the tokens are counted in; every project with none.
    pub(crate) project: Option<&'a Path>,
    /// The home folder, which a project in it is named after.
    pub(crate) home: &'a Path,
}

/// The tokens of each point of `period` in `spend`, oldest first: each
/// hour of today, or each of the last 7 or 30 days.
pub(crate) fn period_points(
    spend: &Spend,
    period: UsagePeriod,
) -> Vec<&[ProjectModelTokens]> {
    let days = match period {
        UsagePeriod::Today => {
            return spend
                .today_by_hour()
                .iter()
                .map(HourSpend::parts)
                .collect();
        },
        UsagePeriod::SevenDays => SEVEN_DAYS,
        UsagePeriod::ThirtyDays => THIRTY_DAYS,
    };

    let counted = spend.last_30_days_by_day();
    let first = counted.len().saturating_sub(days);
    counted[first..].iter().map(DaySpend::parts).collect()
}

/// How many points `period` has over every source of `input`: as many as
/// the source counted over the most days. Each source ends today, so a
/// source with fewer points lines up at the end.
pub(crate) fn point_count(
    input: &UsageInput<'_>,
    period: UsagePeriod,
) -> usize {
    input
        .sources
        .iter()
        .map(|source| period_points(source.spend, period).len())
        .max()
        .unwrap_or(0)
}

/// The rows of `tab` over the points of `period` of every source of
/// `input`, in its project, each with its tokens and their money at each
/// point, the most tokens first; rows with as many by their name. A row
/// with nothing in the period is left out.
pub(crate) fn group_usage(
    input: &UsageInput<'_>,
    period: UsagePeriod,
    tab: UsageTab,
) -> Vec<UsageGroup> {
    let count = point_count(input, period);
    let mut groups: Vec<UsageGroup> = Vec::new();

    for source in input.sources {
        let points = period_points(source.spend, period);
        let offset = count - points.len();

        for (index, point) in points.iter().enumerate() {
            for part in point.iter() {
                if !is_in_project(part, input.project) || is_nothing(part) {
                    continue;
                }

                let name = row_name(part, tab, source, input.home);
                add_to(&mut groups, name, offset + index, part, count);
            }
        }
    }

    groups.sort_by(|a, b| {
        b.total_tokens()
            .cmp(&a.total_tokens())
            .then_with(|| a.name.cmp(&b.name))
    });
    groups
}

/// Adds the tokens and the money of `part` at the point `index` to the
/// row `name` of `groups`, a new row of `count` points when there is none
/// yet.
fn add_to(
    groups: &mut Vec<UsageGroup>,
    name: String,
    index: usize,
    part: &ProjectModelTokens,
    count: usize,
) {
    let position = match groups.iter().position(|group| group.name == name) {
        Some(found) => found,
        None => {
            groups.push(UsageGroup::empty(name, count));
            groups.len() - 1
        },
    };

    groups[position].add(index, part);
}

/// Whether `part` spent nothing: no tokens and no money.
fn is_nothing(part: &ProjectModelTokens) -> bool {
    let tokens = part.tokens();

    tokens.tokens().total() == 0 && tokens.money() == MoneyTotal::default()
}

/// Whether `part` was spent in `project`; every part is when there is no
/// project.
fn is_in_project(part: &ProjectModelTokens, project: Option<&Path>) -> bool {
    project.is_none_or(|project| part.project() == Some(project))
}

/// The name of the row of `tab` that `part` of `source` counts in: its
/// project folder, named as in `home`, its profile, or its model. A project
/// or a model the files did not name is "—".
fn row_name(
    part: &ProjectModelTokens,
    tab: UsageTab,
    source: &UsageSource<'_>,
    home: &Path,
) -> String {
    match tab {
        UsageTab::ByProject => part.project().map_or_else(
            || String::from(UNKNOWN),
            |folder| folder_name(folder, Some(home)),
        ),
        UsageTab::ByProfile => source.name.to_string(),
        _ => part.tokens().model().unwrap_or(UNKNOWN).to_string(),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use otty_agents::{Dollars, ModelTokens, ProviderRef, TokenCount};

    use super::*;

    /// `input` tokens of `model` in `project`, through Anthropic.
    fn part(
        project: Option<&str>,
        model: &str,
        input: u64,
    ) -> ProjectModelTokens {
        let tokens =
            ModelTokens::new(Some(model), TokenCount::new(input, 0, 0), 1)
                .with_provider(Some(ProviderRef::of_id("anthropic")));

        ProjectModelTokens::new(project.map(PathBuf::from), tokens)
    }

    /// The days 2026-11-01 to 2026-11-16, each with `day` tokens of opus in
    /// otty, and today with sonnet in docs too; today split by the hour.
    fn spend() -> Spend {
        let days = (1..=16)
            .map(|day| {
                let mut parts =
                    vec![part(Some("/home/user/otty"), "opus", day)];
                if day == 16 {
                    parts.push(part(Some("/home/user/docs"), "sonnet", 100));
                }
                DaySpend::new(
                    jiff::civil::date(2026, 11, day as i8),
                    Vec::new(),
                )
                .with_parts(parts)
            })
            .collect();
        let hours = (0..24)
            .map(|hour| {
                let parts = match hour {
                    9 => vec![part(Some("/home/user/otty"), "opus", 16)],
                    13 => vec![part(Some("/home/user/docs"), "sonnet", 100)],
                    _ => Vec::new(),
                };
                HourSpend::new(hour, parts)
            })
            .collect();

        Spend::new(days, Vec::new()).with_hours(hours)
    }

    /// The tokens of the profile `name` in `spend`.
    fn source<'a>(name: &'a str, spend: &'a Spend) -> UsageSource<'a> {
        UsageSource { name, spend }
    }

    /// Every project of `sources`.
    fn input<'a>(sources: &'a [UsageSource<'a>]) -> UsageInput<'a> {
        UsageInput {
            sources,
            project: None,
            home: Path::new("/home/user"),
        }
    }

    /// The days 2026-11-10 to 2026-11-16 of Codex, each with 1 000 tokens
    /// of gpt-5 in otty and 50 in docs, through OpenAI, priced by the
    /// Codex part at $1.25 a million input tokens.
    fn codex_spend() -> Spend {
        let days = (10..=16)
            .map(|day| {
                let gpt = |project: &str, input: u64| {
                    let money = MoneyTotal::at_api_prices(
                        Dollars::from_micro_dollars(input * 5 / 4),
                        1,
                    );
                    let tokens = ModelTokens::new(
                        Some("gpt-5"),
                        TokenCount::new(input, 0, 0),
                        1,
                    )
                    .with_provider(Some(ProviderRef::of_id("openai")))
                    .with_money(money);
                    ProjectModelTokens::new(
                        Some(PathBuf::from(project)),
                        tokens,
                    )
                };
                DaySpend::new(jiff::civil::date(2026, 11, day), Vec::new())
                    .with_parts(vec![
                        gpt("/home/user/otty", 1_000),
                        gpt("/home/user/docs", 50),
                    ])
            })
            .collect();

        Spend::new(days, Vec::new())
    }

    /// A row of `tokens` at each point with no money: the tokens of
    /// [`spend`] carry none.
    fn group(name: &str, tokens: &[u64]) -> UsageGroup {
        UsageGroup {
            name: String::from(name),
            tokens: tokens.to_vec(),
            money: vec![MoneyTotal::default(); tokens.len()],
        }
    }

    fn micro_dollars(money: MoneyTotal) -> u64 {
        money.cost().micro_dollars()
    }

    #[test]
    fn today_has_a_point_per_hour_and_the_days_one_per_day() {
        let spend = spend();

        let today = period_points(&spend, UsagePeriod::Today);
        let week = period_points(&spend, UsagePeriod::SevenDays);
        let month = period_points(&spend, UsagePeriod::ThirtyDays);

        assert_eq!(today.len(), 24);
        assert_eq!(today[9], [part(Some("/home/user/otty"), "opus", 16)]);
        assert_eq!(week.len(), 7);
        assert_eq!(week[0], [part(Some("/home/user/otty"), "opus", 10)]);
        assert_eq!(week[6].len(), 2, "today is the last point");
        assert_eq!(month.len(), 16, "only the days that were counted");
    }

    #[test]
    fn by_model_adds_up_each_model_at_each_point_the_largest_first() {
        let spend = spend();
        let sources = [source("Claude Code", &spend)];

        let groups = group_usage(
            &input(&sources),
            UsagePeriod::SevenDays,
            UsageTab::ByModel,
        );

        assert_eq!(
            groups,
            [
                group("sonnet", &[0, 0, 0, 0, 0, 0, 100]),
                group("opus", &[10, 11, 12, 13, 14, 15, 16]),
            ]
        );
        assert_eq!(groups[1].total_tokens(), 91);
    }

    #[test]
    fn by_project_names_each_folder_and_today_by_the_hour() {
        let spend = spend();
        let sources = [source("Claude Code", &spend)];

        let groups = group_usage(
            &input(&sources),
            UsagePeriod::Today,
            UsageTab::ByProject,
        );

        let names: Vec<&str> =
            groups.iter().map(|group| group.name.as_str()).collect();
        assert_eq!(names, ["docs", "otty"]);
        assert_eq!(groups[0].tokens[13], 100);
        assert_eq!(groups[1].tokens[9], 16);
        assert_eq!(groups[1].total_tokens(), 16);
    }

    #[test]
    fn tokens_with_no_project_or_no_model_are_named_by_a_dash() {
        let unnamed = ProjectModelTokens::new(
            None,
            ModelTokens::new(None, TokenCount::new(5, 0, 0), 1),
        );
        let spend = Spend::new(
            vec![
                DaySpend::new(jiff::civil::date(2026, 11, 16), Vec::new())
                    .with_parts(vec![unnamed]),
            ],
            Vec::new(),
        );
        let sources = [source("Claude Code", &spend)];

        let projects = group_usage(
            &input(&sources),
            UsagePeriod::SevenDays,
            UsageTab::ByProject,
        );
        let models = group_usage(
            &input(&sources),
            UsagePeriod::SevenDays,
            UsageTab::ByModel,
        );

        assert_eq!(projects, [group(UNKNOWN, &[5])]);
        assert_eq!(models, [group(UNKNOWN, &[5])]);
    }

    /// The project popup adds up each profile with a session in the
    /// project, and only what it spent there (spec 2026-11-19, Decision
    /// 6).
    #[test]
    fn by_profile_names_each_profile_and_counts_only_the_project() {
        let claude = spend();
        let codex = codex_spend();
        let sources = [source("Claude Code", &claude), source("Codex", &codex)];
        let otty = UsageInput {
            project: Some(Path::new("/home/user/otty")),
            ..input(&sources)
        };

        let groups =
            group_usage(&otty, UsagePeriod::SevenDays, UsageTab::ByProfile);

        let rows: Vec<(&str, &[u64])> = groups
            .iter()
            .map(|group| (group.name.as_str(), group.tokens.as_slice()))
            .collect();
        assert_eq!(
            rows,
            [
                ("Codex", [1_000; 7].as_slice()),
                ("Claude Code", [10, 11, 12, 13, 14, 15, 16].as_slice()),
            ]
        );
    }

    /// A profile counted over fewer days still has today as its last
    /// point.
    #[test]
    fn the_days_of_profiles_counted_over_fewer_days_line_up_at_today() {
        let claude = spend();
        let codex = codex_spend();
        let sources = [source("Claude Code", &claude), source("Codex", &codex)];

        let groups = group_usage(
            &input(&sources),
            UsagePeriod::ThirtyDays,
            UsageTab::ByProfile,
        );

        let codex = groups
            .iter()
            .find(|group| group.name == "Codex")
            .expect("Codex has a row");
        assert_eq!(codex.tokens.len(), 16);
        assert_eq!(codex.tokens[..9], [0; 9]);
        assert_eq!(codex.tokens[9..], [1_050; 7]);
        assert_eq!(codex.money[..9], [MoneyTotal::default(); 9]);
    }

    /// Issue 14 of spec 2026-11-23: each row has its tokens and, next to
    /// them, the money of every profile at API prices, a profile paid by
    /// a subscription too. A row with no money keeps its tokens.
    #[test]
    fn each_row_has_its_money_next_to_its_tokens() {
        let claude = spend();
        let codex = codex_spend();
        let sources = [source("Claude Code", &claude), source("Codex", &codex)];

        let week = group_usage(
            &input(&sources),
            UsagePeriod::SevenDays,
            UsageTab::ByProfile,
        );

        let names: Vec<&str> =
            week.iter().map(|group| group.name.as_str()).collect();
        assert_eq!(names, ["Codex", "Claude Code"]);
        assert_eq!(week[0].total_tokens(), 7_350);
        // 1 250 for 1 000 tokens in otty and 62 for 50 in docs, each day.
        assert_eq!(micro_dollars(week[0].money[6]), 1_312);
        assert_eq!(micro_dollars(week[0].total_money()), 9_184);
        assert!(week[0].total_money().is_estimate());
        assert_eq!(week[1].total_money(), MoneyTotal::default());
    }

    /// Decision 3: the money is what the agent part gave the tokens, in
    /// micro-dollars; the block multiplies nothing. A model with no price
    /// is a row of its tokens, with only unpriced requests for money.
    #[test]
    fn money_is_what_the_part_gave_in_micro_dollars() {
        let priced = MoneyTotal::at_api_prices(
            Dollars::from_micro_dollars(1_250_000),
            1,
        );
        let gpt = ProjectModelTokens::new(
            None,
            ModelTokens::new(
                Some("gpt-5"),
                TokenCount::new(1_000_000, 0, 0),
                1,
            )
            .with_money(priced),
        );
        let unpriced = ProjectModelTokens::new(
            None,
            ModelTokens::new(Some("mystery"), TokenCount::new(9, 0, 0), 1)
                .with_money(MoneyTotal::unpriced(1)),
        );
        let spend = Spend::new(
            vec![
                DaySpend::new(jiff::civil::date(2026, 11, 16), Vec::new())
                    .with_parts(vec![gpt, unpriced]),
            ],
            Vec::new(),
        );
        let sources = [source("Codex", &spend)];

        let groups = group_usage(
            &input(&sources),
            UsagePeriod::SevenDays,
            UsageTab::ByModel,
        );

        assert_eq!(
            groups,
            [
                UsageGroup {
                    name: String::from("gpt-5"),
                    tokens: vec![1_000_000],
                    money: vec![priced],
                },
                UsageGroup {
                    name: String::from("mystery"),
                    tokens: vec![9],
                    money: vec![MoneyTotal::unpriced(1)],
                },
            ]
        );
    }

    /// Money the agent wrote for every answer, such as OpenCode's, is
    /// exact: no mark.
    #[test]
    fn money_the_agent_wrote_is_not_at_api_prices() {
        let written = ProjectModelTokens::new(
            None,
            ModelTokens::new(Some("gpt-5"), TokenCount::new(10, 0, 0), 1)
                .with_money(MoneyTotal::written(Dollars::from_micro_dollars(
                    40,
                ))),
        );
        let spend = Spend::new(
            vec![
                DaySpend::new(jiff::civil::date(2026, 11, 16), Vec::new())
                    .with_parts(vec![written]),
            ],
            Vec::new(),
        );
        let sources = [source("OpenCode", &spend)];

        let groups = group_usage(
            &input(&sources),
            UsagePeriod::SevenDays,
            UsageTab::ByModel,
        );

        assert_eq!(micro_dollars(groups[0].total_money()), 40);
        assert!(!groups[0].total_money().is_estimate());
    }
}
