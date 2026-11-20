//! The usage block (spec 2026-11-19, Decision 6) of a profile page and of
//! the statistics popups of a project and of a session: the title with
//! the total of the period and two drop-downs, a bar per hour of today or
//! per day in the look of the kit `DayBars`, the tabs of the block and a
//! table of the rows of the tab with their value and share. The row picked
//! or under the pointer lights its part of every bar. The last good
//! numbers stay, greyed, after a failed count.

use std::path::Path;
use std::time::SystemTime;

use jiff::tz::TimeZone;
use otty_agents::{
    Dollars, LocalNow, PriceTable, Registry, SavedProfiles, SourceError, Spend,
};
use otty_app_i18n::{self as i18n, Key};

use super::super::usage::{
    UsageChoice, UsageList, UsageMeasure, UsagePeriod, UsageTab,
};
use super::UNKNOWN;
use super::numbers::{dollars_text, short_count};
use super::reset_time::{date_text, date_time_text};
use super::tokens::count_failed_text;
use super::usage_groups::{
    UsageGroup, UsageInput, UsageSource, group_usage, money_is_estimate,
    period_points, point_count,
};

/// The usage block, as the choices of the user and what its subject can
/// show make it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UsageBlockViewModel {
    /// The line under the title: the total of the period ("1.2M in 7
    /// days"), or when the last good numbers were counted.
    pub(crate) line: Option<String>,
    /// The period shown.
    pub(crate) period: UsagePeriod,
    /// The measure shown: tokens when money can not be shown.
    pub(crate) measure: UsageMeasure,
    /// The tab shown: By model when the picked one is not in `tabs` or By
    /// account can not be shown.
    pub(crate) tab: UsageTab,
    /// The tabs of the block, in their order.
    pub(crate) tabs: Vec<UsageTab>,
    /// Why By account is off, such as "the agent does not write which of
    /// its 2 accounts served a message"; `None` when it is on.
    pub(crate) account_reason: Option<String>,
    /// One bar per point of the period, oldest first.
    pub(crate) bars: Vec<UsageBarViewModel>,
    /// The bar of now in the accent color: this hour of today, or today.
    pub(crate) current: Option<usize>,
    /// What the first and the last bar are, written under the bars.
    pub(crate) axis: Option<(String, String)>,
    /// The rows of the tab, the largest first.
    pub(crate) rows: Vec<UsageTableRowViewModel>,
    /// The index of the picked row.
    pub(crate) picked: Option<usize>,
    /// Whether the numbers are the last good ones, drawn greyed.
    pub(crate) is_stale: bool,
    /// What the money is and who it leaves out, while it is shown.
    pub(crate) notes: Vec<String>,
    /// Why the latest count failed.
    pub(crate) failure: Option<String>,
    /// The lists of the drop-downs in a statistics popup, where no menu of
    /// the mode can open; `None` on a page, whose drop-downs open the
    /// menus of the mode.
    pub(crate) lists: Option<UsageListsViewModel>,
}

impl UsageBlockViewModel {
    /// The block of `subject` as `choice` picks it, at `now`.
    pub(crate) fn of(
        subject: &UsageSubject<'_>,
        choice: &UsageChoice,
        now: SystemTime,
    ) -> Self {
        let measure = if subject.can_show_money {
            choice.measure()
        } else {
            UsageMeasure::Tokens
        };
        let tab = match choice.tab() {
            UsageTab::ByAccount if subject.account_reason.is_some() => {
                UsageTab::ByModel
            },
            tab if subject.tabs.contains(&tab) => tab,
            _ => UsageTab::ByModel,
        };
        let period = choice.period();
        let input = UsageInput {
            sources: subject.sources,
            project: subject.project,
            prices: subject.prices,
            home: subject.home,
            saved: subject.saved,
            registry: subject.registry,
        };
        let amount = match measure {
            UsageMeasure::Money if money_is_estimate(&input, period) => {
                Amount::EstimatedMoney
            },
            UsageMeasure::Money => Amount::Money,
            _ => Amount::Tokens,
        };

        let mut notes = Vec::new();
        if amount == Amount::EstimatedMoney {
            notes.push(i18n::t(Key::AgentsUsageMoneyNote).to_string());
        }
        if measure == UsageMeasure::Money {
            notes.extend(subject.left_out.clone());
        }

        let labels = point_labels(subject.sources, period);
        let groups = group_usage(&input, period, tab, measure);
        let totals: Vec<u64> = (0..point_count(&input, period))
            .map(|index| {
                groups
                    .iter()
                    .filter_map(|group| group.values.get(index))
                    .sum()
            })
            .collect();
        let total: u64 = totals.iter().sum();
        let lit_name = choice.hovered().or(choice.picked());
        let lit = groups
            .iter()
            .find(|group| Some(group.name.as_str()) == lit_name);

        let current = match lit {
            Some(_) => None,
            None => current_point(period, totals.len(), now, subject.time_zone),
        };
        let picked = choice.picked().and_then(|picked| {
            groups.iter().position(|group| group.name == picked)
        });
        let rows = groups
            .iter()
            .map(|group| UsageTableRowViewModel {
                name: group.name.clone(),
                value: amount_text(group.total(), amount),
                share: share_text(group.total(), total),
            })
            .collect();

        Self {
            line: Some(total_text(&amount_text(total, amount), period)),
            period,
            measure,
            tab,
            tabs: subject.tabs.to_vec(),
            account_reason: subject.account_reason.map(String::from),
            bars: bars(&totals, &labels, lit, amount),
            current,
            axis: axis(&labels, period),
            rows,
            picked,
            is_stale: false,
            notes,
            failure: None,
            lists: None,
        }
    }

    /// The same block of the last good numbers, counted at `read_at`,
    /// greyed with why the latest count failed, `failure`; the time is
    /// written in `time_zone`.
    pub(crate) fn with_stale(
        mut self,
        read_at: SystemTime,
        failure: &SourceError,
        time_zone: Option<&TimeZone>,
    ) -> Self {
        self.is_stale = true;
        self.failure = Some(count_failed_text(failure));
        self.line = Some(i18n::filled(
            Key::TplAgentsLastGoodNumbers,
            &[("time", &date_time_text(read_at, time_zone))],
        ));

        self
    }

    /// The same block with only why the count failed, `failure`: nothing
    /// was counted yet.
    pub(crate) fn only_failure(mut self, failure: &SourceError) -> Self {
        self.line = None;
        self.bars = Vec::new();
        self.current = None;
        self.axis = None;
        self.rows = Vec::new();
        self.picked = None;
        self.failure = Some(count_failed_text(failure));

        self
    }

    /// The same block in a statistics popup: its drop-downs open their
    /// lists under them, the one of `choice` open, Money with
    /// `money_reason` when it is off.
    pub(crate) fn in_popup(
        mut self,
        choice: &UsageChoice,
        money_reason: Option<String>,
    ) -> Self {
        self.lists = Some(UsageListsViewModel {
            open: choice.open_list(),
            money_reason,
        });

        self
    }
}

/// The lists of the drop-downs of a usage block in a popup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UsageListsViewModel {
    /// The list that is open.
    pub(crate) open: Option<UsageList>,
    /// Why Money is off, written after it in its list; `None` when it is
    /// on.
    pub(crate) money_reason: Option<String>,
}

/// One bar: its value, the part of the lit row and its tooltip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UsageBarViewModel {
    /// The value of the point: tokens or micro-dollars.
    pub(crate) value: u64,
    /// The part of the lit row, while a row is lit.
    pub(crate) lit: Option<u64>,
    /// What the tooltip says, such as "Nov 13: 2.9M · otty 1.2M".
    pub(crate) tip: String,
}

/// One row of the table: its name, its value and its share of the period.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UsageTableRowViewModel {
    /// The project, the profile, the model or the account.
    pub(crate) name: String,
    /// Its value, such as "1.2M" or "~ $3.40".
    pub(crate) value: String,
    /// Its share of the period, such as "62 %".
    pub(crate) share: String,
}

/// What a usage block is made of: the tokens of one profile or of several,
/// and what the block can show of them.
#[derive(Debug, Clone)]
pub(crate) struct UsageSubject<'a> {
    /// The tokens of each profile.
    pub(crate) sources: &'a [UsageSource<'a>],
    /// The project the tokens are counted in; every project with none.
    pub(crate) project: Option<&'a Path>,
    /// The tabs of the block.
    pub(crate) tabs: &'a [UsageTab],
    /// The list prices the money is counted at.
    pub(crate) prices: Option<&'a PriceTable>,
    /// Whether the money can be shown: a profile pays per token and the
    /// prices are known.
    pub(crate) can_show_money: bool,
    /// Why By account is off; `None` when it is on.
    pub(crate) account_reason: Option<&'a str>,
    /// The line that names the profiles paid by a subscription, which the
    /// money leaves out.
    pub(crate) left_out: Option<String>,
    /// The home folder, which a project in it is named after.
    pub(crate) home: &'a Path,
    /// What `agents.json` holds: the names the user gave the accounts.
    pub(crate) saved: Option<&'a SavedProfiles>,
    /// The registry, which names the providers of the accounts.
    pub(crate) registry: &'a Registry,
    /// The user's time zone; UTC when it is not known.
    pub(crate) time_zone: Option<&'a TimeZone>,
}

/// What the values of the block are, so they are written right.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Amount {
    /// Tokens.
    Tokens,
    /// Money the agents wrote for each answer: exact.
    Money,
    /// Money with some tokens at the list prices: an estimate, with "~".
    EstimatedMoney,
}

/// The bars of the points with the `totals` and `labels`: each with the
/// part of the `lit` row, and its tooltip.
fn bars(
    totals: &[u64],
    labels: &[String],
    lit: Option<&UsageGroup>,
    amount: Amount,
) -> Vec<UsageBarViewModel> {
    totals
        .iter()
        .zip(labels)
        .enumerate()
        .map(|(index, (total, label))| {
            let total_amount = amount_text(*total, amount);
            let part = lit.map(|group| {
                let part = group.values.get(index).copied().unwrap_or_default();
                (group, part)
            });
            let tip = match part {
                Some((group, part)) => i18n::filled(
                    Key::TplAgentsBarTipLit,
                    &[
                        ("point", label),
                        ("amount", &total_amount),
                        ("name", &group.name),
                        ("part", &amount_text(part, amount)),
                    ],
                ),
                None => i18n::filled(
                    Key::TplAgentsBarTip,
                    &[("point", label), ("amount", &total_amount)],
                ),
            };

            UsageBarViewModel {
                value: *total,
                lit: part.map(|(_, part)| part),
                tip,
            }
        })
        .collect()
}

/// What each point of `period` is called: the hour of today, such as
/// "09:00", or the day, such as "Nov 13", from the source counted over the
/// most days.
fn point_labels(
    sources: &[UsageSource<'_>],
    period: UsagePeriod,
) -> Vec<String> {
    let longest = sources
        .iter()
        .max_by_key(|source| period_points(source.spend, period).len());

    longest.map_or_else(Vec::new, |source| labels_of(source.spend, period))
}

/// What each point of `period` in `spend` is called.
fn labels_of(spend: &Spend, period: UsagePeriod) -> Vec<String> {
    match period {
        UsagePeriod::Today => spend
            .today_by_hour()
            .iter()
            .map(|hour| hour_text(hour.hour()))
            .collect(),
        _ => {
            let days = period_points(spend, period).len();
            let counted = spend.last_30_days_by_day();
            let first = counted.len().saturating_sub(days);

            counted[first..]
                .iter()
                .map(|day| date_text(day.date()))
                .collect()
        },
    }
}

/// The bar of now among `count` points of `period`: the current hour of
/// today in the user's time zone, or today, the last day.
fn current_point(
    period: UsagePeriod,
    count: usize,
    now: SystemTime,
    time_zone: Option<&TimeZone>,
) -> Option<usize> {
    let last = count.checked_sub(1)?;
    if period != UsagePeriod::Today {
        return Some(last);
    }

    let local = LocalNow::new(now, time_zone.cloned().unwrap_or(TimeZone::UTC));
    let hour = usize::from(local.time_of_day(now).hour().unsigned_abs());
    Some(hour.min(last))
}

/// What is written under the first and the last bar: "00:00" and "23:00"
/// for today, the first day and "Today" for days.
fn axis(labels: &[String], period: UsagePeriod) -> Option<(String, String)> {
    let first = labels.first()?.clone();
    let last = match period {
        UsagePeriod::Today => labels.last()?.clone(),
        _ => i18n::t(Key::AgentsToday).to_string(),
    };

    Some((first, last))
}

/// The total of the period: "1.2M today" or "1.2M in 7 days".
fn total_text(amount: &str, period: UsagePeriod) -> String {
    let days = match period {
        UsagePeriod::Today => {
            return i18n::filled(
                Key::TplAgentsAmountToday,
                &[("amount", amount)],
            );
        },
        UsagePeriod::SevenDays => "7",
        UsagePeriod::ThirtyDays => "30",
    };

    i18n::filled(
        Key::TplAgentsAmountInDays,
        &[("amount", amount), ("count", days)],
    )
}

/// `value` as `amount` writes it: tokens written short, such as "1.2M",
/// or micro-dollars, such as "$3.40", or "~ $3.40" for an estimate.
fn amount_text(value: u64, amount: Amount) -> String {
    match amount {
        Amount::Tokens => short_count(value),
        Amount::Money => dollars_text(Dollars::from_micro_dollars(value)),
        Amount::EstimatedMoney => {
            let dollars = dollars_text(Dollars::from_micro_dollars(value));
            i18n::filled(Key::TplAgentsAboutAmount, &[("amount", &dollars)])
        },
    }
}

/// The share `value` is of `total`, such as "62 %"; "—" of nothing.
fn share_text(value: u64, total: u64) -> String {
    if total == 0 {
        return String::from(UNKNOWN);
    }

    let percent = (value as f64 * 100.0 / total as f64).round();
    i18n::filled(Key::TplAgentsShare, &[("percent", &format!("{percent}"))])
}

/// An hour of today, such as "09:00".
fn hour_text(hour: i8) -> String {
    format!("{hour:02}:00")
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use otty_agents::{
        DaySpend, Dollars, HourSpend, ModelTokens, ProjectModelTokens,
        SourceError, Spend, TokenCount,
    };

    use super::*;
    use crate::testing::{now, price_table};

    fn part(project: &str, model: &str, input: u64) -> ProjectModelTokens {
        ProjectModelTokens::new(
            Some(PathBuf::from(project)),
            ModelTokens::new(Some(model), TokenCount::new(input, 0, 0), 1),
        )
    }

    /// 2026-11-01 to 2026-11-16: `day` tokens of opus in otty each day,
    /// and 100 of sonnet in docs today, at 13:00.
    fn spend() -> Spend {
        let days = (1..=16)
            .map(|day| {
                let mut parts = vec![part("/home/user/otty", "opus", day)];
                if day == 16 {
                    parts.push(part("/home/user/docs", "sonnet", 100));
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
                    9 => vec![part("/home/user/otty", "opus", 16)],
                    13 => vec![part("/home/user/docs", "gpt-5", 1_000_000)],
                    _ => Vec::new(),
                };
                HourSpend::new(hour, parts)
            })
            .collect();

        Spend::new(days, Vec::new()).with_hours(hours)
    }

    /// The tokens of `spend` as the one profile "Claude Code".
    fn source(spend: &Spend) -> UsageSource<'_> {
        UsageSource {
            name: "Claude Code",
            spend,
            accounts: &[],
            is_priced: true,
        }
    }

    fn subject<'a>(
        sources: &'a [UsageSource<'a>],
        prices: Option<&'a PriceTable>,
    ) -> UsageSubject<'a> {
        UsageSubject {
            sources,
            project: None,
            tabs: &UsageTab::PROFILE_PAGE,
            prices,
            can_show_money: true,
            account_reason: None,
            left_out: None,
            home: Path::new("/home/user"),
            saved: None,
            registry: &crate::testing::TEST_REGISTRY,
            time_zone: None,
        }
    }

    fn block(
        subject: &UsageSubject<'_>,
        choice: &UsageChoice,
    ) -> UsageBlockViewModel {
        UsageBlockViewModel::of(subject, choice, now())
    }

    fn row(name: &str, value: &str, share: &str) -> UsageTableRowViewModel {
        UsageTableRowViewModel {
            name: String::from(name),
            value: String::from(value),
            share: String::from(share),
        }
    }

    #[test]
    fn it_opens_on_seven_days_of_tokens_by_model() {
        let spend = spend();
        let sources = [source(&spend)];

        let vm = block(&subject(&sources, None), &UsageChoice::default());

        assert_eq!(vm.period, UsagePeriod::SevenDays);
        assert_eq!(vm.measure, UsageMeasure::Tokens);
        assert_eq!(vm.tab, UsageTab::ByModel);
        assert_eq!(vm.tabs, UsageTab::PROFILE_PAGE);
        assert_eq!(
            vm.line,
            Some(i18n::filled(
                Key::TplAgentsAmountInDays,
                &[("amount", "191"), ("count", "7")]
            ))
        );
        assert_eq!(
            vm.rows,
            [row("sonnet", "100", "52 %"), row("opus", "91", "48 %")]
        );
        let values: Vec<u64> = vm.bars.iter().map(|bar| bar.value).collect();
        assert_eq!(values, [10, 11, 12, 13, 14, 15, 116]);
        assert!(vm.bars.iter().all(|bar| bar.lit.is_none()));
        assert_eq!(vm.current, Some(6), "today, the last bar");
        assert_eq!(
            vm.axis,
            Some((String::from("Nov 10"), i18n::t(Key::AgentsToday).into()))
        );
        assert_eq!(vm.picked, None);
        assert!(!vm.is_stale);
        assert!(vm.notes.is_empty());
        assert_eq!(vm.lists, None, "a page opens the menus of the mode");
    }

    #[test]
    fn the_picked_row_lights_its_part_of_every_bar_and_the_hovered_wins() {
        let spend = spend();
        let sources = [source(&spend)];
        let mut choice = UsageChoice::default();
        choice.pick_row(String::from("opus"));

        let picked = block(&subject(&sources, None), &choice);
        choice.hover_row(Some(String::from("sonnet")));
        let hovered = block(&subject(&sources, None), &choice);

        let lit = |vm: &UsageBlockViewModel| -> Vec<Option<u64>> {
            vm.bars.iter().map(|bar| bar.lit).collect()
        };
        assert_eq!(picked.picked, Some(1));
        assert_eq!(
            lit(&picked),
            [10, 11, 12, 13, 14, 15, 16].map(Some).to_vec()
        );
        assert_eq!(picked.current, None, "the lit parts take the accent");
        assert_eq!(
            picked.bars[6].tip,
            i18n::filled(
                Key::TplAgentsBarTipLit,
                &[
                    ("point", "Nov 16"),
                    ("amount", "116"),
                    ("name", "opus"),
                    ("part", "16"),
                ]
            )
        );
        assert_eq!(lit(&hovered), [0, 0, 0, 0, 0, 0, 100].map(Some).to_vec());
        assert_eq!(hovered.picked, Some(1), "the pick stays in the table");
    }

    #[test]
    fn a_picked_row_the_tab_does_not_list_lights_nothing() {
        let spend = spend();
        let sources = [source(&spend)];
        let mut choice = UsageChoice::default();
        choice.pick_row(String::from("gone"));

        let vm = block(&subject(&sources, None), &choice);

        assert_eq!(vm.picked, None);
        assert!(vm.bars.iter().all(|bar| bar.lit.is_none()));
        assert_eq!(vm.current, Some(6));
    }

    #[test]
    fn today_has_a_bar_per_hour_with_this_hour_in_the_accent() {
        let spend = spend();
        let sources = [source(&spend)];
        let mut choice = UsageChoice::default();
        choice.pick_period(UsagePeriod::Today);
        choice.show_tab(UsageTab::ByProject);

        let vm = block(&subject(&sources, None), &choice);

        assert_eq!(vm.bars.len(), 24);
        // The tests look from 13:46 UTC.
        assert_eq!(vm.current, Some(13));
        assert_eq!(
            vm.axis,
            Some((String::from("00:00"), String::from("23:00")))
        );
        assert_eq!(
            vm.bars[9].tip,
            i18n::filled(
                Key::TplAgentsBarTip,
                &[("point", "09:00"), ("amount", "16")]
            )
        );
        assert_eq!(
            vm.line,
            Some(i18n::filled(Key::TplAgentsAmountToday, &[("amount", "1M")]))
        );
        assert_eq!(
            vm.rows,
            [row("docs", "1M", "100 %"), row("otty", "16", "0 %")]
        );
    }

    #[test]
    fn money_is_an_estimate_at_the_list_prices_with_its_note() {
        let spend = spend();
        let sources = [source(&spend)];
        let prices = price_table();
        let mut choice = UsageChoice::default();
        choice.pick_period(UsagePeriod::Today);
        choice.pick_measure(UsageMeasure::Money);

        let vm = block(&subject(&sources, Some(&prices)), &choice);

        assert_eq!(vm.measure, UsageMeasure::Money);
        assert_eq!(
            vm.rows,
            [row(
                "gpt-5",
                &i18n::filled(
                    Key::TplAgentsAboutAmount,
                    &[("amount", "$1.25")]
                ),
                "100 %"
            )]
        );
        assert_eq!(vm.notes, [i18n::t(Key::AgentsUsageMoneyNote)]);
    }

    /// An agent that writes what each answer cost: that money is exact,
    /// with no "~" and no note of an estimate.
    #[test]
    fn money_the_agent_wrote_is_exact() {
        let tokens =
            ModelTokens::new(Some("gpt-5"), TokenCount::new(10, 0, 0), 1)
                .with_cost(Some(Dollars::from_micro_dollars(500_000)));
        let today = DaySpend::new(jiff::civil::date(2026, 11, 16), Vec::new())
            .with_parts(vec![ProjectModelTokens::new(
                Some(PathBuf::from("/home/user/otty")),
                tokens,
            )]);
        let spend = Spend::new(vec![today], Vec::new());
        let sources = [source(&spend)];
        let prices = price_table();
        let mut choice = UsageChoice::default();
        choice.pick_measure(UsageMeasure::Money);

        let vm = block(&subject(&sources, Some(&prices)), &choice);

        assert_eq!(vm.rows, [row("gpt-5", "$0.50", "100 %")]);
        assert!(vm.notes.is_empty(), "{:?}", vm.notes);
    }

    /// Spec 2026-11-19, issue 07: the money of a project names the
    /// profiles paid by a subscription that it leaves out.
    #[test]
    fn the_money_names_the_profiles_it_leaves_out() {
        let spend = spend();
        let sources = [source(&spend)];
        let prices = price_table();
        let left_out = String::from("Left out: Codex");
        let project = UsageSubject {
            left_out: Some(left_out.clone()),
            ..subject(&sources, Some(&prices))
        };
        let mut choice = UsageChoice::default();

        let tokens = block(&project, &choice);
        choice.pick_measure(UsageMeasure::Money);
        let money = block(&project, &choice);

        assert!(tokens.notes.is_empty(), "only the money leaves out");
        assert_eq!(
            money.notes,
            [i18n::t(Key::AgentsUsageMoneyNote).to_string(), left_out]
        );
    }

    #[test]
    fn money_and_by_account_fall_back_when_the_profile_can_not_show_them() {
        let spend = spend();
        let sources = [source(&spend)];
        let reason = "Codex does not write which of its 2 OpenAI accounts";
        let subscription = UsageSubject {
            can_show_money: false,
            account_reason: Some(reason),
            ..subject(&sources, None)
        };
        let mut choice = UsageChoice::default();
        choice.pick_measure(UsageMeasure::Money);
        choice.show_tab(UsageTab::ByAccount);

        let vm = block(&subscription, &choice);

        assert_eq!(vm.measure, UsageMeasure::Tokens);
        assert_eq!(vm.tab, UsageTab::ByModel);
        assert_eq!(vm.account_reason.as_deref(), Some(reason));
        assert!(vm.notes.is_empty());
    }

    /// A tab picked on a profile page, which the popup does not have,
    /// shows By model.
    #[test]
    fn a_tab_the_block_does_not_have_falls_back_to_by_model() {
        let spend = spend();
        let sources = [source(&spend)];
        let popup = UsageSubject {
            tabs: &UsageTab::SESSION_POPUP,
            ..subject(&sources, None)
        };
        let mut choice = UsageChoice::default();
        choice.show_tab(UsageTab::ByProject);

        let vm = block(&popup, &choice);

        assert_eq!(vm.tab, UsageTab::ByModel);
        assert_eq!(vm.tabs, UsageTab::SESSION_POPUP);
    }

    #[test]
    fn a_project_counts_only_what_was_spent_in_it() {
        let spend = spend();
        let sources = [source(&spend)];
        let otty = UsageSubject {
            project: Some(Path::new("/home/user/otty")),
            ..subject(&sources, None)
        };

        let vm = block(&otty, &UsageChoice::default());

        assert_eq!(vm.rows, [row("opus", "91", "100 %")]);
    }

    /// In a popup no menu of the mode can open: the lists open under
    /// their fields, Money with its reason when it is off.
    #[test]
    fn in_a_popup_the_lists_open_under_their_fields() {
        let spend = spend();
        let sources = [source(&spend)];
        let mut choice = UsageChoice::default();
        choice.toggle_list(UsageList::Measure);
        let reason = String::from("Paid by a subscription");

        let vm = block(&subject(&sources, None), &choice)
            .in_popup(&choice, Some(reason.clone()));

        assert_eq!(
            vm.lists,
            Some(UsageListsViewModel {
                open: Some(UsageList::Measure),
                money_reason: Some(reason),
            })
        );
    }

    #[test]
    fn a_failed_count_keeps_the_last_good_numbers_greyed() {
        let spend = spend();
        let sources = [source(&spend)];
        let down = SourceError::Network(String::from("down"));

        let vm = block(&subject(&sources, None), &UsageChoice::default())
            .with_stale(now(), &down, None);

        assert!(vm.is_stale);
        assert_eq!(
            vm.line,
            Some(i18n::filled(
                Key::TplAgentsLastGoodNumbers,
                &[("time", &date_time_text(now(), None))]
            ))
        );
        assert_eq!(vm.failure, Some(count_failed_text(&down)));
        assert_eq!(vm.rows.len(), 2, "the last good rows stay");
    }

    #[test]
    fn a_failure_with_no_count_shows_alone() {
        let down = SourceError::Network(String::from("down"));

        let vm = block(&subject(&[], None), &UsageChoice::default())
            .only_failure(&down);

        assert!(vm.rows.is_empty());
        assert!(vm.bars.is_empty());
        assert_eq!(vm.line, None);
        assert_eq!(vm.failure, Some(count_failed_text(&down)));
    }

    #[test]
    fn an_empty_period_has_rows_with_no_share() {
        let spend = Spend::new(
            vec![DaySpend::new(jiff::civil::date(2026, 11, 16), Vec::new())],
            Vec::new(),
        );
        let sources = [source(&spend)];

        let vm = block(&subject(&sources, None), &UsageChoice::default());

        assert!(vm.rows.is_empty());
        assert_eq!(vm.bars.len(), 1);
        assert_eq!(UNKNOWN, "—");
    }

    #[test]
    fn a_share_of_nothing_is_a_dash() {
        assert_eq!(share_text(0, 0), UNKNOWN);
        assert_eq!(share_text(1, 3), "33 %");
    }
}
