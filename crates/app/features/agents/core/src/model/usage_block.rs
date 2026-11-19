//! The usage block of a profile page (spec 2026-11-19, Decision 6): the
//! title with the total of the period and two drop-downs, a bar per hour
//! of today or per day in the look of the kit `DayBars`, the tabs By
//! project | By model | By account and a table of the rows of the tab with
//! their value and share. The row picked or under the pointer lights its
//! part of every bar. The last good numbers stay, greyed, after a failed
//! count.

use std::path::Path;
use std::time::SystemTime;

use jiff::tz::TimeZone;
use otty_agents::{Account, Dollars, LocalNow, PriceTable, Spend};
use otty_app_i18n::{self as i18n, Key};

use super::super::spend::ProfileSpend;
use super::super::usage::{UsageChoice, UsageMeasure, UsagePeriod, UsageTab};
use super::UNKNOWN;
use super::kept_answer::KeptAnswer;
use super::numbers::{dollars_text, short_count};
use super::reset_time::{date_text, date_time_text};
use super::tokens::count_failed_text;
use super::usage_groups::{UsageGroup, UsageInput, group_usage, period_points};

/// The usage block, as the choices of the user and what the profile can
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
    /// The tab shown: By model when By account can not be shown.
    pub(crate) tab: UsageTab,
    /// Why By account is off, such as "Kilo does not write which of its 2
    /// OpenAI accounts served a message"; `None` when it is on.
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
    /// What the money is, while it is shown.
    pub(crate) note: Option<String>,
    /// Why the latest count failed.
    pub(crate) failure: Option<String>,
}

impl UsageBlockViewModel {
    /// The block of `subject` as `choice` picks it, at `now`; `None` while
    /// there is nothing to show: the agent can not count tokens, or nothing
    /// was counted yet.
    pub(crate) fn of(
        subject: &UsageSubject<'_>,
        choice: &UsageChoice,
        now: SystemTime,
    ) -> Option<Self> {
        let spend = subject.spend;
        if !spend.has_counter() {
            return None;
        }

        let measure = if subject.can_show_money {
            choice.measure()
        } else {
            UsageMeasure::Tokens
        };
        let tab = match choice.tab() {
            UsageTab::ByAccount if subject.account_reason.is_some() => {
                UsageTab::ByModel
            },
            tab => tab,
        };
        let mut vm = Self {
            line: None,
            period: choice.period(),
            measure,
            tab,
            account_reason: subject.account_reason.map(String::from),
            bars: Vec::new(),
            current: None,
            axis: None,
            rows: Vec::new(),
            picked: None,
            is_stale: false,
            note: (measure == UsageMeasure::Money)
                .then(|| i18n::t(Key::AgentsUsageMoneyNote).to_string()),
            failure: None,
        };

        let (counted, read_at, failure) =
            match KeptAnswer::new(spend.last_good(), spend.failure()) {
                KeptAnswer::Nothing => return None,
                KeptAnswer::OnlyFailure(error) => {
                    vm.failure = Some(count_failed_text(error));
                    return Some(vm);
                },
                KeptAnswer::Answer {
                    value,
                    read_at,
                    failure,
                } => (value, read_at, failure),
            };

        let input = UsageInput {
            accounts: subject.accounts,
            prices: subject.prices,
            home: subject.home,
        };
        let points = period_points(counted, vm.period);
        let labels = point_labels(counted, vm.period);
        let groups = group_usage(&input, &points, tab, measure);
        let totals: Vec<u64> = (0..points.len())
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

        vm.is_stale = failure.is_some();
        vm.failure = failure.map(count_failed_text);
        vm.line = Some(if vm.is_stale {
            i18n::filled(
                Key::TplAgentsLastGoodNumbers,
                &[("time", &date_time_text(read_at, subject.time_zone))],
            )
        } else {
            total_text(&amount_text(total, measure), vm.period)
        });
        vm.bars = bars(&totals, &labels, lit, measure);
        vm.current = match lit {
            Some(_) => None,
            None => {
                current_point(vm.period, totals.len(), now, subject.time_zone)
            },
        };
        vm.axis = axis(&labels, vm.period);
        vm.picked = choice.picked().and_then(|picked| {
            groups.iter().position(|group| group.name == picked)
        });
        vm.rows = groups
            .iter()
            .map(|group| UsageTableRowViewModel {
                name: group.name.clone(),
                value: amount_text(group.total(), measure),
                share: share_text(group.total(), total),
            })
            .collect();

        Some(vm)
    }
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
    /// The project, the model or the account.
    pub(crate) name: String,
    /// Its value, such as "1.2M" or "~ $3.40".
    pub(crate) value: String,
    /// Its share of the period, such as "62 %".
    pub(crate) share: String,
}

/// What the usage block of one profile is made of.
#[derive(Debug, Clone, Copy)]
pub(crate) struct UsageSubject<'a> {
    /// The count of the tokens of the profile.
    pub(crate) spend: &'a ProfileSpend,
    /// The accounts found in the profile.
    pub(crate) accounts: &'a [Account],
    /// The list prices the money is counted at.
    pub(crate) prices: Option<&'a PriceTable>,
    /// Whether the money can be shown: the profile pays per token.
    pub(crate) can_show_money: bool,
    /// Why By account is off; `None` when it is on.
    pub(crate) account_reason: Option<&'a str>,
    /// The home folder, which a project in it is named after.
    pub(crate) home: &'a Path,
    /// The user's time zone; UTC when it is not known.
    pub(crate) time_zone: Option<&'a TimeZone>,
}

/// The bars of the points with the `totals` and `labels`: each with the
/// part of the `lit` row, and its tooltip.
fn bars(
    totals: &[u64],
    labels: &[String],
    lit: Option<&UsageGroup>,
    measure: UsageMeasure,
) -> Vec<UsageBarViewModel> {
    totals
        .iter()
        .zip(labels)
        .enumerate()
        .map(|(index, (total, label))| {
            let amount = amount_text(*total, measure);
            let part = lit.map(|group| {
                let part = group.values.get(index).copied().unwrap_or_default();
                (group, part)
            });
            let tip = match part {
                Some((group, part)) => i18n::filled(
                    Key::TplAgentsBarTipLit,
                    &[
                        ("point", label),
                        ("amount", &amount),
                        ("name", &group.name),
                        ("part", &amount_text(part, measure)),
                    ],
                ),
                None => i18n::filled(
                    Key::TplAgentsBarTip,
                    &[("point", label), ("amount", &amount)],
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

/// What each point of `period` in `spend` is called: the hour of today,
/// such as "09:00", or the day, such as "Nov 13".
fn point_labels(spend: &Spend, period: UsagePeriod) -> Vec<String> {
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

/// `value` in `measure`: tokens written short, such as "1.2M", or
/// micro-dollars as an estimate, such as "~ $3.40".
fn amount_text(value: u64, measure: UsageMeasure) -> String {
    match measure {
        UsageMeasure::Tokens => short_count(value),
        UsageMeasure::Money => {
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
    use std::time::Duration;

    use otty_agents::{
        DaySpend, HourSpend, ModelTokens, ProjectModelTokens, SourceError,
        Spend, TokenCount,
    };

    use super::*;
    use crate::testing::{now, price_table};
    use crate::types::SpendCount;

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

    fn counted(spend: Spend) -> ProfileSpend {
        let mut counted = ProfileSpend::default();
        counted.apply(SpendCount::Counted(spend), now());
        counted
    }

    fn subject<'a>(
        spend: &'a ProfileSpend,
        prices: Option<&'a PriceTable>,
    ) -> UsageSubject<'a> {
        UsageSubject {
            spend,
            accounts: &[],
            prices,
            can_show_money: true,
            account_reason: None,
            home: Path::new("/home/user"),
            time_zone: None,
        }
    }

    fn block(
        subject: &UsageSubject<'_>,
        choice: &UsageChoice,
    ) -> UsageBlockViewModel {
        UsageBlockViewModel::of(subject, choice, now())
            .expect("the block shows")
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
        let spend = counted(spend());

        let vm = block(&subject(&spend, None), &UsageChoice::default());

        assert_eq!(vm.period, UsagePeriod::SevenDays);
        assert_eq!(vm.measure, UsageMeasure::Tokens);
        assert_eq!(vm.tab, UsageTab::ByModel);
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
        assert_eq!(vm.note, None);
    }

    #[test]
    fn the_picked_row_lights_its_part_of_every_bar_and_the_hovered_wins() {
        let spend = counted(spend());
        let mut choice = UsageChoice::default();
        choice.pick_row(String::from("opus"));

        let picked = block(&subject(&spend, None), &choice);
        choice.hover_row(Some(String::from("sonnet")));
        let hovered = block(&subject(&spend, None), &choice);

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
        let spend = counted(spend());
        let mut choice = UsageChoice::default();
        choice.pick_row(String::from("gone"));

        let vm = block(&subject(&spend, None), &choice);

        assert_eq!(vm.picked, None);
        assert!(vm.bars.iter().all(|bar| bar.lit.is_none()));
        assert_eq!(vm.current, Some(6));
    }

    #[test]
    fn today_has_a_bar_per_hour_with_this_hour_in_the_accent() {
        let spend = counted(spend());
        let mut choice = UsageChoice::default();
        choice.pick_period(UsagePeriod::Today);
        choice.show_tab(UsageTab::ByProject);

        let vm = block(&subject(&spend, None), &choice);

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
        let spend = counted(spend());
        let prices = price_table();
        let mut choice = UsageChoice::default();
        choice.pick_period(UsagePeriod::Today);
        choice.pick_measure(UsageMeasure::Money);

        let vm = block(&subject(&spend, Some(&prices)), &choice);

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
        assert_eq!(vm.note, Some(i18n::t(Key::AgentsUsageMoneyNote).into()));
    }

    #[test]
    fn money_and_by_account_fall_back_when_the_profile_can_not_show_them() {
        let spend = counted(spend());
        let reason = "Codex does not write which of its 2 OpenAI accounts";
        let subscription = UsageSubject {
            can_show_money: false,
            account_reason: Some(reason),
            ..subject(&spend, None)
        };
        let mut choice = UsageChoice::default();
        choice.pick_measure(UsageMeasure::Money);
        choice.show_tab(UsageTab::ByAccount);

        let vm = block(&subscription, &choice);

        assert_eq!(vm.measure, UsageMeasure::Tokens);
        assert_eq!(vm.tab, UsageTab::ByModel);
        assert_eq!(vm.account_reason.as_deref(), Some(reason));
        assert_eq!(vm.note, None);
    }

    #[test]
    fn a_failed_count_keeps_the_last_good_numbers_greyed() {
        let mut spend = counted(spend());
        let failed_at = now() + Duration::from_secs(600);
        spend.apply(
            SpendCount::Failed(SourceError::Network(String::from("down"))),
            failed_at,
        );

        let vm = block(&subject(&spend, None), &UsageChoice::default());

        assert!(vm.is_stale);
        assert_eq!(
            vm.line,
            Some(i18n::filled(
                Key::TplAgentsLastGoodNumbers,
                &[("time", &date_time_text(now(), None))]
            ))
        );
        assert_eq!(
            vm.failure,
            Some(count_failed_text(&SourceError::Network(String::from(
                "down"
            ))))
        );
        assert_eq!(vm.rows.len(), 2, "the last good rows stay");
    }

    #[test]
    fn nothing_counted_yet_shows_no_block_and_a_failure_shows_alone() {
        let nothing = ProfileSpend::default();
        let mut failed = ProfileSpend::default();
        failed.apply(
            SpendCount::Failed(SourceError::Network(String::from("down"))),
            now(),
        );

        let none = UsageBlockViewModel::of(
            &subject(&nothing, None),
            &UsageChoice::default(),
            now(),
        );
        let alone = block(&subject(&failed, None), &UsageChoice::default());

        assert_eq!(none, None);
        assert!(alone.rows.is_empty());
        assert!(alone.bars.is_empty());
        assert_eq!(alone.line, None);
        assert!(alone.failure.is_some());
    }

    #[test]
    fn an_empty_period_has_rows_with_no_share() {
        let spend = counted(Spend::new(
            vec![DaySpend::new(jiff::civil::date(2026, 11, 16), Vec::new())],
            Vec::new(),
        ));

        let vm = block(&subject(&spend, None), &UsageChoice::default());

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
