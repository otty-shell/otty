//! The numbers of the usage block (spec 2026-11-19, Decisions 4 and 6):
//! the tokens of a period of one profile or of several, one point per hour
//! of today or per day, in one project or in all, added up by project, by
//! profile, by model or by account, in tokens or in money at the list
//! prices.

use std::path::Path;

use otty_agents::{
    Account, DaySpend, HourSpend, ModelTokens, MoneyTotal, PriceTable,
    ProjectModelTokens, Registry, SavedProfiles, Spend,
};
use otty_app_domain::folder_path::folder_name;
use otty_app_i18n::{self as i18n, Key};

use super::super::usage::{UsageMeasure, UsagePeriod, UsageTab};
use super::UNKNOWN;
use super::account_name::account_name;

/// Days in the period of 7 days.
const SEVEN_DAYS: usize = 7;
/// Days in the period of 30 days.
const THIRTY_DAYS: usize = 30;

/// One row of the table of the usage block: a project, a profile, a model
/// or an account, with its value at each point of the period.
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
    /// The accounts found in the profile, which tell the account that
    /// paid for tokens that do not name it.
    pub(crate) accounts: &'a [Account],
    /// Whether its tokens count in money: the profile pays per token. The
    /// tokens of a subscription have no price per request.
    pub(crate) is_priced: bool,
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
    /// What `agents.json` holds: the names the user gave the accounts.
    pub(crate) saved: Option<&'a SavedProfiles>,
    /// The registry, which names the providers of the accounts.
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
/// with nothing in the period is left out, and so is the money of a
/// source that is not priced.
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
        let prices = input.prices.filter(|_| source.is_priced);
        let offset = count - points.len();

        for (index, point) in points.iter().enumerate() {
            for part in point.iter() {
                if !is_in_project(part, input.project) {
                    continue;
                }
                let value = value_of(part.tokens(), measure, prices);
                if value == 0 {
                    continue;
                }

                let name = name_of(part, tab, source, input);
                add_to(&mut groups, name, offset + index, value, count);
            }
        }
    }

    groups.sort_by(|a, b| {
        b.total().cmp(&a.total()).then_with(|| a.name.cmp(&b.name))
    });
    groups
}

/// Whether the money of `period` over `input` is an estimate at the list
/// prices: a source that pays per token has tokens there, in its project,
/// whose cost its agent did not write. An agent that writes it has
/// exact money.
pub(crate) fn money_is_estimate(
    input: &UsageInput<'_>,
    period: UsagePeriod,
) -> bool {
    input
        .sources
        .iter()
        .filter(|source| source.is_priced)
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
/// project folder, its profile, its model, or the account that paid for
/// it, else "no account". A project or a model the files did not name is
/// "—".
fn name_of(
    part: &ProjectModelTokens,
    tab: UsageTab,
    source: &UsageSource<'_>,
    input: &UsageInput<'_>,
) -> String {
    match tab {
        UsageTab::ByProject => part.project().map_or_else(
            || String::from(UNKNOWN),
            |folder| folder_name(folder, Some(input.home)),
        ),
        UsageTab::ByProfile => source.name.to_string(),
        UsageTab::ByModel => {
            part.tokens().model().unwrap_or(UNKNOWN).to_string()
        },
        UsageTab::ByAccount => source
            .accounts
            .iter()
            .find(|account| {
                account.pays_for(part.tokens(), source.accounts, input.registry)
            })
            .map_or_else(
                || i18n::t(Key::AgentsNoAccount).to_string(),
                |account| account_name(input.registry, input.saved, account),
            ),
    }
}

/// The value of `tokens` in `measure`: all its tokens, or what they cost
/// at `prices` in micro-dollars; nothing with no prices.
fn value_of(
    tokens: &ModelTokens,
    measure: UsageMeasure,
    prices: Option<&PriceTable>,
) -> u64 {
    match measure {
        UsageMeasure::Tokens => tokens.tokens().total(),
        UsageMeasure::Money => prices.map_or(0, |prices| {
            MoneyTotal::of_models(std::slice::from_ref(tokens), prices)
                .cost()
                .micro_dollars()
        }),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use otty_agents::{
        AccountIdentity, AccountKind, AccountLogin, FoundIn, ProviderRef,
        TokenCount,
    };

    use super::*;
    use crate::testing::price_table;

    fn home() -> PathBuf {
        PathBuf::from("/home/user")
    }

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

    /// The tokens of the profile `name` in `spend`, paid through
    /// `accounts`; its money counts.
    fn source<'a>(
        name: &'a str,
        spend: &'a Spend,
        accounts: &'a [Account],
    ) -> UsageSource<'a> {
        UsageSource {
            name,
            spend,
            accounts,
            is_priced: true,
        }
    }

    /// Every project of `sources`, with no prices.
    fn input<'a>(sources: &'a [UsageSource<'a>]) -> UsageInput<'a> {
        UsageInput {
            sources,
            project: None,
            prices: None,
            home: Path::new("/home/user"),
            saved: None,
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

    fn chatgpt() -> Account {
        Account::new(
            ProviderRef::of_id("openai"),
            AccountIdentity::AccountId(String::from("made-up-chatgpt")),
            AccountLogin::Email(String::from("me@example.org")),
            AccountKind::Limits,
            FoundIn::ProfileFolder(home().join(".codex")),
        )
    }

    fn claude_max() -> Account {
        Account::new(
            ProviderRef::of_id("anthropic"),
            AccountIdentity::AccountId(String::from("made-up-max")),
            AccountLogin::Email(String::from("you@example.com")),
            AccountKind::Limits,
            FoundIn::ProfileFolder(home().join(".claude")),
        )
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
        let sources = [source("Claude Code", &spend, &[])];

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
        let sources = [source("Claude Code", &spend, &[])];

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
        let sources = [source("Claude Code", &spend, &[])];

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

    #[test]
    fn by_account_names_the_account_that_paid_and_tokens_with_none() {
        let spend = spend();
        let local = ProjectModelTokens::new(
            Some(PathBuf::from("/home/user/otty")),
            ModelTokens::new(Some("qwen"), TokenCount::new(3, 0, 0), 1),
        );
        let hours = (0..24)
            .map(|hour| {
                let parts = match hour {
                    9 => period_points(&spend, UsagePeriod::Today)[9].to_vec(),
                    10 => vec![local.clone()],
                    _ => Vec::new(),
                };
                HourSpend::new(hour, parts)
            })
            .collect();
        let spend = Spend::new(Vec::new(), Vec::new()).with_hours(hours);
        let accounts = [claude_max()];
        let sources = [source("Claude Code", &spend, &accounts)];

        let groups = group_usage(
            &input(&sources),
            UsagePeriod::Today,
            UsageTab::ByAccount,
            UsageMeasure::Tokens,
        );

        assert_eq!(groups[0].name, "Anthropic · you@example.com");
        assert_eq!(groups[0].values[9], 16);
        assert_eq!(groups[1].name, i18n::t(Key::AgentsNoAccount));
        assert_eq!(groups[1].values[10], 3);
    }

    /// The project popup adds up each profile with a session in the
    /// project, and only what it spent there (spec 2026-11-19, Decision
    /// 6).
    #[test]
    fn by_profile_names_each_profile_and_counts_only_the_project() {
        let claude = spend();
        let codex = codex_spend();
        let sources = [
            source("Claude Code", &claude, &[]),
            source("Codex", &codex, &[]),
        ];
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

    /// Each profile's tokens go to the accounts of that profile.
    #[test]
    fn by_account_sums_each_profile_through_its_own_accounts() {
        let claude = spend();
        let codex = codex_spend();
        let claude_accounts = [claude_max()];
        let codex_accounts = [chatgpt()];
        let sources = [
            source("Claude Code", &claude, &claude_accounts),
            source("Codex", &codex, &codex_accounts),
        ];
        let otty = UsageInput {
            project: Some(Path::new("/home/user/otty")),
            ..input(&sources)
        };

        let groups = group_usage(
            &otty,
            UsagePeriod::SevenDays,
            UsageTab::ByAccount,
            UsageMeasure::Tokens,
        );

        let totals: Vec<(&str, u64)> = groups
            .iter()
            .map(|group| (group.name.as_str(), group.total()))
            .collect();
        assert_eq!(
            totals,
            [
                ("OpenAI · me@example.org", 7_000),
                ("Anthropic · you@example.com", 91),
            ]
        );
    }

    /// A profile counted over fewer days still has today as its last
    /// point.
    #[test]
    fn the_days_of_profiles_counted_over_fewer_days_line_up_at_today() {
        let claude = spend();
        let codex = codex_spend();
        let sources = [
            source("Claude Code", &claude, &[]),
            source("Codex", &codex, &[]),
        ];

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

    /// Spec 2026-11-19, issue 07: money only from known prices; a profile
    /// paid by a subscription is left out.
    #[test]
    fn money_leaves_out_a_profile_paid_by_a_subscription() {
        let codex = codex_spend();
        let prices = price_table();
        let sources = [
            source("Codex · Key", &codex, &[]),
            UsageSource {
                is_priced: false,
                ..source("Codex", &codex, &[])
            },
        ];
        let priced = UsageInput {
            prices: Some(&prices),
            ..input(&sources)
        };

        let groups = group_usage(
            &priced,
            UsagePeriod::Today,
            UsageTab::ByProfile,
            UsageMeasure::Money,
        );
        let week = group_usage(
            &priced,
            UsagePeriod::SevenDays,
            UsageTab::ByProfile,
            UsageMeasure::Money,
        );

        assert!(groups.is_empty(), "Codex counted no hours: {groups:?}");
        let names: Vec<&str> =
            week.iter().map(|group| group.name.as_str()).collect();
        assert_eq!(names, ["Codex · Key"]);
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
        let sources = [source("Codex", &spend, &[])];
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
