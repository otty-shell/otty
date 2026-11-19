//! The numbers of the usage block (spec 2026-11-19, Decisions 4 and 6):
//! the tokens of a period, one point per hour of today or per day, added
//! up by project, by model or by account, in tokens or in money at the
//! list prices.

use std::path::Path;

use otty_agents::{
    Account, DaySpend, HourSpend, ModelTokens, MoneyTotal, PriceTable,
    ProjectModelTokens, Spend,
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

/// One row of the table of the usage block: a project, a model or an
/// account, with its value at each point of the period.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UsageGroup {
    /// The name of the row, such as "otty" or "claude-opus-4-5".
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

/// What the usage of one profile is counted from.
#[derive(Debug, Clone, Copy)]
pub(crate) struct UsageInput<'a> {
    /// The accounts found in the profile, which tell the account that
    /// paid for tokens that do not name it.
    pub(crate) accounts: &'a [Account],
    /// The list prices the money is counted at; with none, the money is
    /// zero.
    pub(crate) prices: Option<&'a PriceTable>,
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

/// The rows of `tab` over `points`, each with its value at each point in
/// `measure`, the largest first; rows with as much by their name. A row
/// with nothing in the period is left out.
pub(crate) fn group_usage(
    input: &UsageInput<'_>,
    points: &[&[ProjectModelTokens]],
    tab: UsageTab,
    measure: UsageMeasure,
) -> Vec<UsageGroup> {
    let mut groups: Vec<UsageGroup> = Vec::new();

    for (index, point) in points.iter().enumerate() {
        for part in point.iter() {
            let value = value_of(part.tokens(), measure, input.prices);
            if value == 0 {
                continue;
            }

            let name = name_of(part, tab, input);
            let group = match groups.iter().position(|group| group.name == name)
            {
                Some(found) => &mut groups[found],
                None => {
                    groups.push(UsageGroup {
                        name,
                        values: vec![0; points.len()],
                    });
                    let last = groups.len() - 1;
                    &mut groups[last]
                },
            };
            if let Some(slot) = group.values.get_mut(index) {
                *slot += value;
            }
        }
    }

    groups.sort_by(|a, b| {
        b.total().cmp(&a.total()).then_with(|| a.name.cmp(&b.name))
    });
    groups
}

/// The name of the row of `tab` that `part` counts in: its project
/// folder, its model, or the account that paid for it, else "no
/// account". A project or a model the files did not name is "—".
fn name_of(
    part: &ProjectModelTokens,
    tab: UsageTab,
    input: &UsageInput<'_>,
) -> String {
    match tab {
        UsageTab::ByProject => part.project().map_or_else(
            || String::from(UNKNOWN),
            |folder| folder_name(folder, Some(input.home)),
        ),
        UsageTab::ByModel => {
            part.tokens().model().unwrap_or(UNKNOWN).to_string()
        },
        UsageTab::ByAccount => input
            .accounts
            .iter()
            .find(|account| account.pays_for(part.tokens(), input.accounts))
            .map_or_else(
                || i18n::t(Key::AgentsNoAccount).to_string(),
                account_name,
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
        AccountIdentity, AccountKind, AccountLogin, FoundIn, Provider,
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
                .with_provider(Some(Provider::Anthropic));

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

    fn input<'a>(accounts: &'a [Account]) -> UsageInput<'a> {
        UsageInput {
            accounts,
            prices: None,
            home: Path::new("/home/user"),
        }
    }

    fn claude_max() -> Account {
        Account::new(
            Provider::Anthropic,
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
        let points = period_points(&spend, UsagePeriod::SevenDays);

        let groups = group_usage(
            &input(&[]),
            &points,
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
        let points = period_points(&spend, UsagePeriod::Today);

        let groups = group_usage(
            &input(&[]),
            &points,
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
        let points = [std::slice::from_ref(&unnamed)];

        let projects = group_usage(
            &input(&[]),
            &points,
            UsageTab::ByProject,
            UsageMeasure::Tokens,
        );
        let models = group_usage(
            &input(&[]),
            &points,
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
        let today = period_points(&spend, UsagePeriod::Today);
        let points = [today[9], std::slice::from_ref(&local)];
        let accounts = [claude_max()];

        let groups = group_usage(
            &input(&accounts),
            &points,
            UsageTab::ByAccount,
            UsageMeasure::Tokens,
        );

        assert_eq!(
            groups,
            [
                group("Anthropic · you@example.com", &[16, 0]),
                group(i18n::t(Key::AgentsNoAccount), &[0, 3]),
            ]
        );
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
        let point = [gpt, unpriced];
        let prices = price_table();
        let priced = UsageInput {
            prices: Some(&prices),
            ..input(&[])
        };

        let groups = group_usage(
            &priced,
            &[&point[..]],
            UsageTab::ByModel,
            UsageMeasure::Money,
        );
        let no_prices = group_usage(
            &input(&[]),
            &[&point[..]],
            UsageTab::ByModel,
            UsageMeasure::Money,
        );

        assert_eq!(groups, [group("gpt-5", &[1_250_000])]);
        assert_eq!(no_prices, [], "nothing costs anything with no prices");
    }
}
