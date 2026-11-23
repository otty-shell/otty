//! The numbers of the usage block (spec 2026-11-19, Decision 6): the
//! tokens of a period of one profile or of several, one point per hour of
//! today or per day, in one project or in all, added up by project, by
//! profile or by model, in tokens or in money at API prices. The money of
//! every profile counts, on a subscription too (spec
//! 2026-11-23-accounts-apart-from-profiles, Decision 3).

use std::path::Path;

use otty_agents::{
    DaySpend, HourSpend, ModelTokens, MoneyTotal, PriceTable,
    ProjectModelTokens, Registry, Spend,
};
use otty_app_domain::folder_path::folder_name;

use super::super::usage::{UsageMeasure, UsagePeriod, UsageTab};
use super::UNKNOWN;

/// Days in the period of 7 days.
const SEVEN_DAYS: usize = 7;
/// Days in the period of 30 days.
const THIRTY_DAYS: usize = 30;

/// One row of the table of the usage block: a project, a profile or a
/// model, with its value at each point of the period.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UsageGroup {
    /// The name of the row, such as "otty" or "example-large".
    pub(crate) name: String,
    /// Its value at each point, oldest first: tokens, or micro-dollars.
    pub(crate) values: Vec<u64>,
}

impl UsageGroup {
    /// Its value over the whole period.
    pub(crate) fn total(&self) -> u64 {
        self.values.iter().sum()
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
    /// The list prices the money is counted at; with none, the money is
    /// zero.
    pub(crate) prices: Option<&'a PriceTable>,
    /// The home folder, which a project in it is named after.
    pub(crate) home: &'a Path,
    /// The registry, which names who served the tokens to the price table.
    pub(crate) registry: &'a Registry,
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
/// `input`, in its project, each with its value at each point in
/// `measure`, the largest first; rows with as much by their name. A row
/// with nothing in the period is left out.
pub(crate) fn group_usage(
    input: &UsageInput<'_>,
    period: UsagePeriod,
    tab: UsageTab,
    measure: UsageMeasure,
) -> Vec<UsageGroup> {
    let count = point_count(input, period);
    let mut groups: Vec<UsageGroup> = Vec::new();

    for source in input.sources {
        let points = period_points(source.spend, period);
        let offset = count - points.len();

        for (index, point) in points.iter().enumerate() {
            for part in point.iter() {
                if !is_in_project(part, input.project) {
                    continue;
                }
                let value = value_of(
                    part.tokens(),
                    measure,
                    input.prices,
                    input.registry,
                );
                if value == 0 {
                    continue;
                }

                let name = row_name(part, tab, source, input.home);
                add_to(&mut groups, name, offset + index, value, count);
            }
        }
    }

    groups.sort_by(|a, b| {
        b.total().cmp(&a.total()).then_with(|| a.name.cmp(&b.name))
    });
    groups
}

/// Whether the money of `period` over `input` is at API prices: a source
/// has tokens there, in its project, whose cost its agent did not write.
/// An agent that writes it, such as OpenCode, has exact money.
pub(crate) fn money_is_at_api_prices(
    input: &UsageInput<'_>,
    period: UsagePeriod,
) -> bool {
    input
        .sources
        .iter()
        .flat_map(|source| period_points(source.spend, period))
        .flatten()
        .filter(|part| is_in_project(part, input.project))
        .any(|part| {
            let tokens = part.tokens();
            !tokens.tokens().is_zero() && tokens.cost().is_none()
        })
}

/// Adds `value` at the point `index` to the row `name` of `groups`, a new
/// row of `count` points when there is none yet.
fn add_to(
    groups: &mut Vec<UsageGroup>,
    name: String,
    index: usize,
    value: u64,
    count: usize,
) {
    let position = match groups.iter().position(|group| group.name == name) {
        Some(found) => found,
        None => {
            groups.push(UsageGroup {
                name,
                values: vec![0; count],
            });
            groups.len() - 1
        },
    };

    if let Some(slot) = groups[position].values.get_mut(index) {
        *slot += value;
    }
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

/// The value of `tokens` in `measure`: all its tokens, or what they cost
/// at `prices` for who served them, as `registry` names it, in
/// micro-dollars; nothing with no prices.
fn value_of(
    tokens: &ModelTokens,
    measure: UsageMeasure,
    prices: Option<&PriceTable>,
    registry: &Registry,
) -> u64 {
    match measure {
        UsageMeasure::Tokens => tokens.tokens().total(),
        UsageMeasure::Money => prices.map_or(0, |prices| {
            let tokens = std::slice::from_ref(tokens);

            MoneyTotal::of_models(tokens, prices, registry)
                .cost()
                .micro_dollars()
        }),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use otty_agents::{ProviderRef, TokenCount};

    use super::*;
    use crate::testing::price_table;

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

    /// Every project of `sources`, with no prices.
    fn input<'a>(sources: &'a [UsageSource<'a>]) -> UsageInput<'a> {
        UsageInput {
            sources,
            project: None,
            prices: None,
            home: Path::new("/home/user"),
            registry: &crate::testing::TEST_REGISTRY,
        }
    }

    /// The days 2026-11-10 to 2026-11-16 of Codex, each with 1 000 tokens
    /// of gpt-5 in otty and 50 in docs, through OpenAI.
    fn codex_spend() -> Spend {
        let days = (10..=16)
            .map(|day| {
                let gpt = |project: &str, input: u64| {
                    let tokens = ModelTokens::new(
                        Some("gpt-5"),
                        TokenCount::new(input, 0, 0),
                        1,
                    )
                    .with_provider(Some(ProviderRef::of_id("openai")));
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

    fn group(name: &str, values: &[u64]) -> UsageGroup {
        UsageGroup {
            name: String::from(name),
            values: values.to_vec(),
        }
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
            UsageMeasure::Tokens,
        );

        assert_eq!(
            groups,
            [
                group("sonnet", &[0, 0, 0, 0, 0, 0, 100]),
                group("opus", &[10, 11, 12, 13, 14, 15, 16]),
            ]
        );
        assert_eq!(groups[1].total(), 91);
    }

    #[test]
    fn by_project_names_each_folder_and_today_by_the_hour() {
        let spend = spend();
        let sources = [source("Claude Code", &spend)];

        let groups = group_usage(
            &input(&sources),
            UsagePeriod::Today,
            UsageTab::ByProject,
            UsageMeasure::Tokens,
        );

        let names: Vec<&str> =
            groups.iter().map(|group| group.name.as_str()).collect();
        assert_eq!(names, ["docs", "otty"]);
        assert_eq!(groups[0].values[13], 100);
        assert_eq!(groups[1].values[9], 16);
        assert_eq!(groups[1].total(), 16);
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
            UsageMeasure::Tokens,
        );
        let models = group_usage(
            &input(&sources),
            UsagePeriod::SevenDays,
            UsageTab::ByModel,
            UsageMeasure::Tokens,
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

        let groups = group_usage(
            &otty,
            UsagePeriod::SevenDays,
            UsageTab::ByProfile,
            UsageMeasure::Tokens,
        );

        assert_eq!(
            groups,
            [
                group("Codex", &[1_000; 7]),
                group("Claude Code", &[10, 11, 12, 13, 14, 15, 16]),
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
            UsageMeasure::Tokens,
        );

        let codex = groups
            .iter()
            .find(|group| group.name == "Codex")
            .expect("Codex has a row");
        assert_eq!(codex.values.len(), 16);
        assert_eq!(codex.values[..9], [0; 9]);
        assert_eq!(codex.values[9..], [1_050; 7]);
    }

    /// Spec 2026-11-23, Decision 3: the money of every profile counts at
    /// API prices, a profile paid by a subscription too.
    #[test]
    fn money_counts_every_profile() {
        let claude = spend();
        let codex = codex_spend();
        let prices = price_table();
        let sources = [source("Claude Code", &claude), source("Codex", &codex)];
        let priced = UsageInput {
            prices: Some(&prices),
            ..input(&sources)
        };

        let week = group_usage(
            &priced,
            UsagePeriod::SevenDays,
            UsageTab::ByProfile,
            UsageMeasure::Money,
        );

        let names: Vec<&str> =
            week.iter().map(|group| group.name.as_str()).collect();
        assert_eq!(names, ["Codex"], "the made-up prices price gpt-5 only");
        assert!(money_is_at_api_prices(&priced, UsagePeriod::SevenDays));
    }

    #[test]
    fn money_is_the_tokens_at_the_list_prices_in_micro_dollars() {
        let gpt = ProjectModelTokens::new(
            None,
            ModelTokens::new(
                Some("gpt-5"),
                TokenCount::new(1_000_000, 0, 0),
                1,
            ),
        );
        let unpriced = ProjectModelTokens::new(
            None,
            ModelTokens::new(Some("mystery"), TokenCount::new(9, 0, 0), 1),
        );
        let spend = Spend::new(
            vec![
                DaySpend::new(jiff::civil::date(2026, 11, 16), Vec::new())
                    .with_parts(vec![gpt, unpriced]),
            ],
            Vec::new(),
        );
        let sources = [source("Codex", &spend)];
        let prices = price_table();
        let priced = UsageInput {
            prices: Some(&prices),
            ..input(&sources)
        };

        let groups = group_usage(
            &priced,
            UsagePeriod::SevenDays,
            UsageTab::ByModel,
            UsageMeasure::Money,
        );
        let no_prices = group_usage(
            &input(&sources),
            UsagePeriod::SevenDays,
            UsageTab::ByModel,
            UsageMeasure::Money,
        );

        assert_eq!(groups, [group("gpt-5", &[1_250_000])]);
        assert_eq!(no_prices, [], "nothing costs anything with no prices");
    }
}
