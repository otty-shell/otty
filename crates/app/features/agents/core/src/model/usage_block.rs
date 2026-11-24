//! The usage block (spec 2026-11-19, Decision 6) of a profile page and of
//! the statistics popups of a project and of a session: the title with
//! the total of the period and the period drop-down, a bar per hour of
//! today or per day in the look of the kit `DayBars`, the tabs of the
//! block and a table of the rows of the tab with their tokens, their money
//! and their share. The row picked or under the pointer lights its part of
//! every bar. Tokens and money show side by side, with no switch between
//! them, and the bars are as tall as their tokens, since a model with no
//! price would drop out of bars of money (issue 14 of spec
//! 2026-11-23-accounts-apart-from-profiles). Money is the tokens of every
//! profile at API prices, on a subscription too, and carries the mark "at
//! API prices", or the cost the agent wrote, with no mark (Decision 3).
//! The last good numbers stay, greyed, after a failed count.

use std::path::Path;
use std::time::SystemTime;

use jiff::tz::TimeZone;
use otty_agents::{FetchedPrices, LocalNow, MoneyTotal, SourceError, Spend};
use otty_app_i18n::{self as i18n, Key};

use super::super::usage::{UsageChoice, UsagePeriod, UsageTab};
use super::money::MoneyKind;
use super::numbers::{dollars_text, short_count};
use super::reset_time::{date_text, date_time_text};
use super::tokens::count_failed_text;
use super::usage_groups::{
    UsageGroup, UsageInput, UsageSource, group_usage, period_points,
    point_count,
};
use super::{NOT_READ_YET, UNKNOWN};

/// The usage block, as the choices of the user and what its subject can
/// show make it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UsageBlockViewModel {
    /// The line under the title: the tokens and the money of the period
    /// ("400K tokens · ~$12.40 in 7 days"), or when the last good numbers
    /// were counted.
    pub(crate) line: Option<String>,
    /// The period shown.
    pub(crate) period: UsagePeriod,
    /// The tab shown: By model when the picked one is not in `tabs`.
    pub(crate) tab: UsageTab,
    /// The tabs of the block, in their order.
    pub(crate) tabs: Vec<UsageTab>,
    /// One bar per point of the period, oldest first.
    pub(crate) bars: Vec<UsageBarViewModel>,
    /// The bar of now in the accent color: this hour of today, or today.
    pub(crate) current: Option<usize>,
    /// What the first and the last bar are, written under the bars.
    pub(crate) axis: Option<(String, String)>,
    /// The rows of the tab, the most tokens first.
    pub(crate) rows: Vec<UsageTableRowViewModel>,
    /// The index of the picked row.
    pub(crate) picked: Option<usize>,
    /// Whether the numbers are the last good ones, drawn greyed.
    pub(crate) is_stale: bool,
    /// What the money is once the prices are read: at API prices, which
    /// brings its mark under the table, or exact, the cost the agent
    /// wrote. `None` while the prices are not read.
    pub(crate) money_kind: Option<MoneyKind>,
    /// Why the latest count failed.
    pub(crate) failure: Option<String>,
    /// How the period drop-down opens its choices.
    pub(crate) period_picker: PeriodPicker,
}

impl UsageBlockViewModel {
    /// The block of `subject` as `choice` picks it, at `now`.
    pub(crate) fn of(
        subject: &UsageSubject<'_>,
        choice: &UsageChoice,
        now: SystemTime,
    ) -> Self {
        let tab = match choice.tab() {
            tab if subject.tabs.contains(&tab) => tab,
            _ => UsageTab::ByModel,
        };
        let period = choice.period();
        let input = UsageInput {
            sources: subject.sources,
            project: subject.project,
            home: subject.home,
        };
        let prices = subject.prices;

        let labels = point_labels(subject.sources, period);
        let groups = group_usage(&input, period, tab);
        let points = point_totals(&groups, point_count(&input, period));
        let total_tokens: u64 = points.iter().map(|point| point.0).sum();
        let total_money = points
            .iter()
            .fold(MoneyTotal::default(), |sum, point| sum + point.1);
        let lit_name = choice.hovered().or(choice.picked());
        let lit = groups
            .iter()
            .find(|group| Some(group.name.as_str()) == lit_name);

        let money_kind = prices.map(|prices| {
            MoneyKind::at_prices(
                total_money.is_estimate(),
                prices,
                subject.time_zone,
            )
        });
        let current = match lit {
            Some(_) => None,
            None => current_point(period, points.len(), now, subject.time_zone),
        };
        let picked = choice.picked().and_then(|picked| {
            groups.iter().position(|group| group.name == picked)
        });
        let rows = groups
            .iter()
            .map(|group| UsageTableRowViewModel {
                name: group.name.clone(),
                tokens: short_count(group.total_tokens()),
                money: money_cell_text(&group.total_money(), prices),
                share: share_text(group.total_tokens(), total_tokens),
            })
            .collect();
        let total = tokens_and_money_text(total_tokens, &total_money, prices);

        Self {
            line: Some(total_text(&total, period)),
            period,
            tab,
            tabs: subject.tabs.to_vec(),
            bars: bars(&points, &labels, lit, prices),
            current,
            axis: axis(&labels, period),
            rows,
            picked,
            is_stale: false,
            money_kind,
            failure: None,
            period_picker: PeriodPicker::Menu,
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
        self.money_kind = None;
        self.failure = Some(count_failed_text(failure));

        self
    }

    /// The same block in a statistics popup: its period drop-down opens
    /// its list under it, open as `choice` has it.
    pub(crate) fn in_popup(mut self, choice: &UsageChoice) -> Self {
        self.period_picker = PeriodPicker::List {
            is_open: choice.is_period_list_open(),
        };

        self
    }
}

/// How the period drop-down of a usage block opens its choices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PeriodPicker {
    /// On a page: the period menu of the mode opens at the pointer.
    Menu,
    /// In a statistics popup, where no menu of the mode can open: the
    /// list of the periods opens under the drop-down.
    List {
        /// Whether the list is open.
        is_open: bool,
    },
}

/// One bar: its tokens, the tokens of the lit row and its tooltip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UsageBarViewModel {
    /// The tokens of the point, which make the bar as tall as it is.
    pub(crate) value: u64,
    /// The tokens of the lit row, while a row is lit.
    pub(crate) lit: Option<u64>,
    /// What the tooltip says, the tokens and the money, such as "Nov 13:
    /// 120K tokens · ~$3.10".
    pub(crate) tip: String,
}

/// One row of the table: its name, its tokens, their money and its share
/// of the tokens of the period.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UsageTableRowViewModel {
    /// The project, the profile or the model.
    pub(crate) name: String,
    /// Its tokens, such as "1.2M".
    pub(crate) tokens: String,
    /// Their money, such as "$3.40"; "—" for a model with no price and
    /// "…" while the prices are not read.
    pub(crate) money: String,
    /// Its share of the tokens of the period, such as "62 %".
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
    /// The list prices the agent parts priced the tokens at, with when
    /// they were fetched, which the mark "at API prices" tells; `None`
    /// while they are not read, and the money is "…".
    pub(crate) prices: Option<&'a FetchedPrices>,
    /// The home folder, which a project in it is named after.
    pub(crate) home: &'a Path,
    /// The user's time zone; UTC when it is not known.
    pub(crate) time_zone: Option<&'a TimeZone>,
}

/// The tokens and the money of each of `count` points over every row of
/// `groups`.
fn point_totals(groups: &[UsageGroup], count: usize) -> Vec<(u64, MoneyTotal)> {
    (0..count)
        .map(|index| {
            let tokens = groups
                .iter()
                .filter_map(|group| group.tokens.get(index))
                .sum();
            let money = groups
                .iter()
                .filter_map(|group| group.money.get(index))
                .fold(MoneyTotal::default(), |sum, money| sum + *money);
            (tokens, money)
        })
        .collect()
}

/// The bars of the `points` with their `labels`, as tall as their tokens:
/// each with the tokens of the `lit` row, and its tooltip with the tokens
/// and the money at `prices`.
fn bars(
    points: &[(u64, MoneyTotal)],
    labels: &[String],
    lit: Option<&UsageGroup>,
    prices: Option<&FetchedPrices>,
) -> Vec<UsageBarViewModel> {
    points
        .iter()
        .zip(labels)
        .enumerate()
        .map(|(index, ((tokens, money), label))| {
            let amount = tokens_and_money_text(*tokens, money, prices);
            let part = lit.map(|group| {
                let tokens = group.tokens.get(index).copied();
                let money = group.money.get(index).copied();
                (group, tokens.unwrap_or_default(), money.unwrap_or_default())
            });

            let tip = match part {
                Some((group, tokens, money)) => i18n::filled(
                    Key::TplAgentsBarTipLit,
                    &[
                        ("point", label),
                        ("amount", &amount),
                        ("name", &group.name),
                        (
                            "part",
                            &tokens_and_money_text(tokens, &money, prices),
                        ),
                    ],
                ),
                None => i18n::filled(
                    Key::TplAgentsBarTip,
                    &[("point", label), ("amount", &amount)],
                ),
            };

            UsageBarViewModel {
                value: *tokens,
                lit: part.map(|(_, tokens, _)| tokens),
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

/// The total of the period: "400K tokens · ~$12.40 today" or "400K tokens
/// · ~$12.40 in 7 days".
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

/// `tokens` and their `money` in one line, such as "120K tokens · ~$3.10":
/// the money starts with "~" when some of it is at API prices, is "—"
/// when none of it has a price and "…" while the `prices` are not read.
fn tokens_and_money_text(
    tokens: u64,
    money: &MoneyTotal,
    prices: Option<&FetchedPrices>,
) -> String {
    let cell = money_cell_text(money, prices);
    let money = if prices.is_some() && has_price(money) && money.is_estimate() {
        format!("~{cell}")
    } else {
        cell
    };

    i18n::filled(
        Key::TplAgentsTokensAndMoney,
        &[("tokens", &short_count(tokens)), ("money", &money)],
    )
}

/// `money` as a cell of the Money column writes it: "$3.40", "—" when
/// none of it has a price, such as a model the price list does not name,
/// and "…" while the `prices` are not read.
fn money_cell_text(
    money: &MoneyTotal,
    prices: Option<&FetchedPrices>,
) -> String {
    if prices.is_none() {
        return String::from(NOT_READ_YET);
    }
    if !has_price(money) {
        return String::from(UNKNOWN);
    }

    dollars_text(money.cost())
}

/// Whether some of `money` has a price: it costs something, or no request
/// of it went unpriced.
fn has_price(money: &MoneyTotal) -> bool {
    money.cost().micro_dollars() > 0 || money.unpriced_requests() == 0
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
        DaySpend, Dollars, HourSpend, ModelTokens, MoneyTotal,
        ProjectModelTokens, SourceError, Spend, TokenCount,
    };

    use super::*;
    use crate::testing::{fetched_prices, now};

    /// `input` tokens of `model` in `project`, priced as an agent part
    /// prices them at the made-up list prices: gpt-5 at $1.25 a million
    /// input tokens, any other model with no price.
    fn part(project: &str, model: &str, input: u64) -> ProjectModelTokens {
        let money = match model {
            "gpt-5" => MoneyTotal::at_api_prices(
                Dollars::from_micro_dollars(input * 5 / 4),
                1,
            ),
            _ => MoneyTotal::unpriced(1),
        };

        ProjectModelTokens::new(
            Some(PathBuf::from(project)),
            ModelTokens::new(Some(model), TokenCount::new(input, 0, 0), 1)
                .with_money(money),
        )
    }

    /// 2026-11-01 to 2026-11-16: `day` tokens of opus in otty each day,
    /// and 100 of sonnet in docs today; today by the hour, 16 of opus at
    /// 09:00 and a million of gpt-5 in docs at 13:00.
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
        }
    }

    fn subject<'a>(
        sources: &'a [UsageSource<'a>],
        prices: Option<&'a FetchedPrices>,
    ) -> UsageSubject<'a> {
        UsageSubject {
            sources,
            project: None,
            tabs: &UsageTab::PROFILE_PAGE,
            prices,
            home: Path::new("/home/user"),
            time_zone: None,
        }
    }

    fn block(
        subject: &UsageSubject<'_>,
        choice: &UsageChoice,
    ) -> UsageBlockViewModel {
        UsageBlockViewModel::of(subject, choice, now())
    }

    fn row(
        name: &str,
        tokens: &str,
        money: &str,
        share: &str,
    ) -> UsageTableRowViewModel {
        UsageTableRowViewModel {
            name: String::from(name),
            tokens: String::from(tokens),
            money: String::from(money),
            share: String::from(share),
        }
    }

    /// "{tokens} tokens · {money}".
    fn both(tokens: &str, money: &str) -> String {
        i18n::filled(
            Key::TplAgentsTokensAndMoney,
            &[("tokens", tokens), ("money", money)],
        )
    }

    fn in_seven_days(amount: &str) -> Option<String> {
        Some(i18n::filled(
            Key::TplAgentsAmountInDays,
            &[("amount", amount), ("count", "7")],
        ))
    }

    /// Issue 14 of spec 2026-11-23: before the prices are read the money
    /// is "…", in the total and in every row.
    #[test]
    fn it_opens_on_seven_days_by_model_with_the_money_not_read_yet() {
        let spend = spend();
        let sources = [source(&spend)];

        let vm = block(&subject(&sources, None), &UsageChoice::default());

        assert_eq!(vm.period, UsagePeriod::SevenDays);
        assert_eq!(vm.tab, UsageTab::ByModel);
        assert_eq!(vm.tabs, UsageTab::PROFILE_PAGE);
        assert_eq!(vm.line, in_seven_days(&both("191", "…")));
        assert_eq!(
            vm.rows,
            [
                row("sonnet", "100", "…", "52 %"),
                row("opus", "91", "…", "48 %")
            ]
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
        assert_eq!(vm.money_kind, None, "no mark before the prices");
        assert_eq!(
            vm.period_picker,
            PeriodPicker::Menu,
            "a page opens the menu of the mode"
        );
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
                    ("amount", &both("116", "…")),
                    ("name", "opus"),
                    ("part", &both("16", "…")),
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

    /// Issue 14: the bars are as tall as their tokens whatever the money,
    /// and the tooltip of a bar says both.
    #[test]
    fn today_has_a_bar_per_hour_as_tall_as_its_tokens() {
        let spend = spend();
        let sources = [source(&spend)];
        let prices = fetched_prices(now());
        let mut choice = UsageChoice::default();
        choice.pick_period(UsagePeriod::Today);
        choice.show_tab(UsageTab::ByProject);

        let vm = block(&subject(&sources, Some(&prices)), &choice);

        assert_eq!(vm.bars.len(), 24);
        // The tests look from 13:46 UTC.
        assert_eq!(vm.current, Some(13));
        assert_eq!(
            vm.axis,
            Some((String::from("00:00"), String::from("23:00")))
        );
        assert_eq!(vm.bars[9].value, 16, "opus has no price, and a bar");
        assert_eq!(vm.bars[13].value, 1_000_000, "tokens, not money");
        assert_eq!(
            vm.bars[9].tip,
            i18n::filled(
                Key::TplAgentsBarTip,
                &[("point", "09:00"), ("amount", &both("16", "—"))]
            )
        );
        assert_eq!(
            vm.bars[13].tip,
            i18n::filled(
                Key::TplAgentsBarTip,
                &[("point", "13:00"), ("amount", &both("1M", "~$1.25"))]
            )
        );
        assert_eq!(
            vm.rows,
            [
                row("docs", "1M", "$1.25", "100 %"),
                row("otty", "16", "—", "0 %")
            ]
        );
    }

    /// Issue 14 and spec 2026-11-23, Decision 3: the total says the
    /// tokens and the money, which is at API prices and so starts with
    /// "~" and carries the mark "at API prices" with the time of the
    /// prices in its tooltip. A model with no price shows "—" in Money.
    #[test]
    fn the_total_says_the_tokens_and_the_money_at_api_prices_with_the_mark() {
        let spend = spend();
        let sources = [source(&spend)];
        let prices = fetched_prices(now());
        let mut choice = UsageChoice::default();
        choice.pick_period(UsagePeriod::Today);

        let vm = block(&subject(&sources, Some(&prices)), &choice);

        assert_eq!(
            vm.rows,
            [
                row("gpt-5", "1M", "$1.25", "100 %"),
                row("opus", "16", "—", "0 %")
            ]
        );
        assert_eq!(
            vm.line,
            Some(i18n::filled(
                Key::TplAgentsAmountToday,
                &[("amount", &both("1M", "~$1.25"))]
            ))
        );
        let mark = vm.money_kind.as_ref().and_then(MoneyKind::mark);
        assert_eq!(mark, MoneyKind::at_prices(true, &prices, None).mark());
        assert_eq!(
            mark.map(|mark| mark.label),
            Some(i18n::t(Key::AgentsApiPricesMark))
        );
    }

    /// An agent that writes what each answer cost, such as OpenCode: that
    /// money is exact, with no "~" and no mark "at API prices" (issue 01
    /// of spec 2026-11-23 keeps it).
    #[test]
    fn money_the_agent_wrote_is_exact() {
        let tokens =
            ModelTokens::new(Some("gpt-5"), TokenCount::new(10, 0, 0), 1)
                .with_money(MoneyTotal::written(Dollars::from_micro_dollars(
                    500_000,
                )));
        let today = DaySpend::new(jiff::civil::date(2026, 11, 16), Vec::new())
            .with_parts(vec![ProjectModelTokens::new(
                Some(PathBuf::from("/home/user/otty")),
                tokens,
            )]);
        let spend = Spend::new(vec![today], Vec::new());
        let sources = [source(&spend)];
        let prices = fetched_prices(now());

        let vm =
            block(&subject(&sources, Some(&prices)), &UsageChoice::default());

        assert_eq!(vm.rows, [row("gpt-5", "10", "$0.50", "100 %")]);
        assert_eq!(vm.line, in_seven_days(&both("10", "$0.50")));
        assert_eq!(
            vm.bars[0].tip,
            i18n::filled(
                Key::TplAgentsBarTip,
                &[("point", "Nov 16"), ("amount", &both("10", "$0.50"))]
            )
        );
        assert_eq!(vm.money_kind, Some(MoneyKind::Exact));
    }

    /// A failure with nothing counted shows no sums, so no mark either.
    #[test]
    fn a_failure_with_no_count_has_no_mark() {
        let spend = spend();
        let sources = [source(&spend)];
        let prices = fetched_prices(now());
        let down = SourceError::Network(String::from("down"));

        let vm =
            block(&subject(&sources, Some(&prices)), &UsageChoice::default())
                .only_failure(&down);

        assert_eq!(vm.money_kind, None);
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

        assert_eq!(vm.rows, [row("opus", "91", "…", "100 %")]);
    }

    /// In a popup no menu of the mode can open: the list of the periods
    /// opens under its field.
    #[test]
    fn in_a_popup_the_list_of_the_periods_opens_under_its_field() {
        let spend = spend();
        let sources = [source(&spend)];
        let mut choice = UsageChoice::default();
        choice.toggle_period_list();

        let vm = block(&subject(&sources, None), &choice).in_popup(&choice);

        assert_eq!(vm.period_picker, PeriodPicker::List { is_open: true });
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
