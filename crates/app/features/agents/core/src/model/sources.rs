//! "Where the numbers come from" on a profile's dashboard: a table of the
//! sources of each number of the dashboard, grouped by the number they
//! give, each group in the order its walk tries them, with what each source
//! did at the latest read and when its data is from (spec Decisions 5, 6
//! and 18).

use std::time::SystemTime;

use jiff::tz::TimeZone;
use otty_agents::{
    FetchedPrices, MoneyEstimate, PriceSource, SourceError, SourceFailure,
    SourceId, Unavailable,
};
use otty_app_i18n::{self as i18n, Key};
use otty_ui_kit::components::status_badge::StatusTone;

use super::super::bill::ProfileBill;
use super::super::failure_text::{
    renewed_text, source_error_text, source_name, unavailable_text,
};
use super::super::limits::ProfileLimits;
use super::super::prices::ListPrices;
use super::super::spend::ProfileSpend;
use super::reset_time::date_time_text;

/// One row of the table: a source, the number it gives and its state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SourceRowViewModel {
    /// The source.
    pub(crate) source: SourceId,
    /// The number of the dashboard the source gives, such as "Limits", on
    /// the first row of its group only.
    pub(crate) number: Option<&'static str>,
    /// The plain name of the source, such as "Anthropic usage API (direct
    /// request)".
    pub(crate) name: &'static str,
    /// The tooltip of the name: what the source does, in plain words.
    pub(crate) name_tip: &'static str,
    /// What it did: answered, not needed, login expired, not available or
    /// failed.
    pub(crate) state: &'static str,
    /// The color of the state.
    pub(crate) tone: StatusTone,
    /// The tooltip of the state: why the source gave no answer, what to do
    /// about it, or what its answer is.
    pub(crate) tip: Option<String>,
    /// When the data of the source is from, such as "Nov 17, 2026, 10:00",
    /// when it answered.
    pub(crate) updated: Option<String>,
}

/// What is known of the bill of an API key profile, the first source of
/// its money (spec Decisions 5 and 9).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BillSourceState<'a> {
    /// The agent has no bill OTTY can read: the estimate is the only
    /// source.
    NoSource,
    /// The agent has a bill, but the profile has no admin key.
    NoAdminKey,
    /// The profile has an admin key: what its reads of the bill gave.
    Read(&'a ProfileBill),
}

/// What the source `id` does, in plain words, for the tooltip of its name.
pub(crate) fn source_tip(id: SourceId) -> &'static str {
    let key = match id {
        SourceId::SessionFiles => Key::AgentsSourceTipSessionFiles,
        SourceId::AnthropicUsageApi => Key::AgentsSourceTipAnthropicUsageApi,
        SourceId::ClaudeUsageCommand => Key::AgentsSourceTipClaudeUsageCommand,
        SourceId::ChatGptUsageApi => Key::AgentsSourceTipChatgptUsageApi,
        SourceId::CodexAppServer => Key::AgentsSourceTipCodexAppServer,
        SourceId::LatestCodexSession => Key::AgentsSourceTipLatestCodexSession,
        SourceId::AnthropicCostReport => {
            Key::AgentsSourceTipAnthropicCostReport
        },
        SourceId::SessionFilesAtListPrices => {
            Key::AgentsSourceTipSessionFilesAtListPrices
        },
        SourceId::LiteLlmPrices => Key::AgentsSourceTipLitellmPrices,
        SourceId::KeptPrices => Key::AgentsSourceTipKeptPrices,
    };

    i18n::t(key)
}

/// The group of the limits as the latest read walked them: the sources
/// that failed, the one that answered with when its data is from, and
/// those after it as not needed. The data is from the time of the read, or
/// of the record the source took the limits from, written in the user's
/// `time_zone`. Empty before the first read and for an agent with no way
/// to read limits.
pub(crate) fn limit_sources(
    limits: &ProfileLimits,
    time_zone: Option<&TimeZone>,
) -> Vec<SourceRowViewModel> {
    if !limits.has_sources() {
        return Vec::new();
    }

    let rows = match (limits.failures(), limits.last_good()) {
        (Some(failures), _) => walk_rows(limits.sources(), failures, None),
        (None, Some((answer, read_at))) => {
            let data_from = answer.value().recorded_at().unwrap_or(read_at);
            let answered =
                updated_at(answered(answer.source()), data_from, time_zone);

            walk_rows(
                limits.sources(),
                answer.earlier_failures(),
                Some(answered),
            )
        },
        (None, None) => return Vec::new(),
    };

    grouped(Key::AgentsNumberLimits, rows)
}

/// The group of the tokens: the session files, which also give the
/// sessions, as the latest count left them, with the time of that count
/// written in the user's `time_zone` when it answered. Empty before the
/// first count and for an agent that can not count tokens.
pub(crate) fn token_sources(
    spend: &ProfileSpend,
    time_zone: Option<&TimeZone>,
) -> Vec<SourceRowViewModel> {
    let id = SourceId::SessionFiles;

    if !spend.has_counter() {
        return Vec::new();
    }

    let row = match (spend.failure(), spend.last_good()) {
        (Some(error), _) => failed(id, error),
        (None, Some((_, counted_at))) => {
            updated_at(answered(id), counted_at, time_zone)
        },
        (None, None) => return Vec::new(),
    };

    grouped(Key::AgentsTokens, vec![row])
}

/// The groups of the money of an API key profile and of its prices. The
/// money: the bill of its organization, when its agent has one, then its
/// tokens at the list prices of the `money` estimate, which are not needed
/// while the bill answers. The prices: the sources of the list prices the
/// estimate used (spec Decision 16), listed only with the estimate. The
/// bill shows once it was read, or as not available with no admin key; the
/// estimate once the tokens were counted and the prices read, and never for
/// an agent that can not count them. The tooltip of the source that
/// answered says what the money is: the bill of the whole organization, or
/// an estimate at the LiteLLM prices of a time. The times are written in the
/// user's `time_zone` (spec Decision 12).
pub(crate) fn money_sources(
    spend: &ProfileSpend,
    money: Option<&MoneyEstimate>,
    prices: &ListPrices,
    bill: BillSourceState<'_>,
    time_zone: Option<&TimeZone>,
) -> Vec<SourceRowViewModel> {
    let bill_id = SourceId::AnthropicCostReport;
    let estimate = || estimate_sources(spend, money, prices, time_zone);

    let (money_rows, price_rows) = match bill {
        BillSourceState::NoSource => estimate(),
        BillSourceState::NoAdminKey => {
            let no_key = SourceError::NotAvailable(Unavailable::NoAdminKey);
            let row = failed(bill_id, &no_key);
            let (estimate, prices) = estimate();
            ([row].into_iter().chain(estimate).collect(), prices)
        },
        BillSourceState::Read(read) => {
            match (read.answer(), read.reads().read_at()) {
                (Some(_), Some(read_at)) => {
                    let from_bill =
                        i18n::t(Key::AgentsMoneyFromBill).to_string();
                    let answered = answered_as(bill_id, from_bill);
                    let rows = vec![
                        updated_at(answered, read_at, time_zone),
                        not_needed(SourceId::SessionFilesAtListPrices),
                    ];
                    (rows, Vec::new())
                },
                _ => {
                    let failure =
                        read.failure().map(|error| failed(bill_id, error));
                    let (estimate, prices) = estimate();
                    (failure.into_iter().chain(estimate).collect(), prices)
                },
            }
        },
    };

    let mut rows = grouped(Key::AgentsNumberMoney, money_rows);
    rows.extend(grouped(Key::AgentsNumberPrices, price_rows));
    rows
}

/// The estimate as a source of the money, its tokens at the list prices of
/// `money` as old as their count, and then, apart, the sources of the
/// `prices`. Both empty before the first count or the first read of the
/// prices, and for an agent that can not count tokens. With no prices from
/// any source, the estimate is not available and the price sources say
/// why.
fn estimate_sources(
    spend: &ProfileSpend,
    money: Option<&MoneyEstimate>,
    prices: &ListPrices,
    time_zone: Option<&TimeZone>,
) -> (Vec<SourceRowViewModel>, Vec<SourceRowViewModel>) {
    let id = SourceId::SessionFilesAtListPrices;

    if !spend.has_counter() {
        return (Vec::new(), Vec::new());
    }
    if let Some(error) = spend.failure() {
        return (vec![failed(id, error)], Vec::new());
    }
    let Some((_, counted_at)) = spend.last_good() else {
        return (Vec::new(), Vec::new());
    };

    let estimate = match (money, prices.last_good()) {
        (Some(_), Some(fetched)) => {
            let tip = estimate_text(fetched, time_zone);
            updated_at(answered_as(id, tip), counted_at, time_zone)
        },
        _ if prices.has_failed() => {
            let no_prices = i18n::t(Key::AgentsNoPrices).to_string();
            not_available(id, no_prices)
        },
        _ => return (Vec::new(), Vec::new()),
    };

    (vec![estimate], price_sources(prices, time_zone))
}

/// The sources of the list prices as their latest read walked them: the
/// ones that failed, the one that answered with the time its file was
/// fetched, written in the user's `time_zone`, and
/// those after it as not needed.
fn price_sources(
    prices: &ListPrices,
    time_zone: Option<&TimeZone>,
) -> Vec<SourceRowViewModel> {
    let sources = &PriceSource::SOURCES;

    match prices.latest() {
        Some(Ok(answer)) => {
            let fetched_at = answer.value().fetched_at();
            let answered =
                updated_at(answered(answer.source()), fetched_at, time_zone);

            walk_rows(sources, answer.earlier_failures(), Some(answered))
        },
        Some(Err(failures)) => walk_rows(sources, failures, None),
        None => Vec::new(),
    }
}

/// The rows of a walk over `sources`, best first: each source that
/// failed, as `failures` say, the one that `answered`, and those after it
/// as not needed. With no list of the sources, only those the walk tried.
fn walk_rows(
    sources: &[SourceId],
    failures: &[SourceFailure],
    answered: Option<SourceRowViewModel>,
) -> Vec<SourceRowViewModel> {
    let walked: Vec<SourceRowViewModel> = failures
        .iter()
        .map(|failure| failed(failure.source(), failure.error()))
        .chain(answered)
        .collect();
    if sources.is_empty() {
        return walked;
    }

    sources
        .iter()
        .map(|id| {
            walked
                .iter()
                .find(|row| row.source == *id)
                .cloned()
                .unwrap_or_else(|| not_needed(*id))
        })
        .collect()
}

/// What the estimate is: "Tokens from the session files at the LiteLLM
/// prices updated Nov 17, 2026, 10:00, an estimate".
fn estimate_text(
    prices: &FetchedPrices,
    time_zone: Option<&TimeZone>,
) -> String {
    let time = date_time_text(prices.fetched_at(), time_zone);

    i18n::filled(Key::TplAgentsMoneyEstimate, &[("time", &time)])
}

/// The row of the source `id` in the `state` of `tone`, with `tip` as the
/// tooltip of the state.
fn source_row(
    id: SourceId,
    state: Key,
    tone: StatusTone,
    tip: Option<String>,
) -> SourceRowViewModel {
    SourceRowViewModel {
        source: id,
        number: None,
        name: source_name(id),
        name_tip: source_tip(id),
        state: i18n::t(state),
        tone,
        tip,
        updated: None,
    }
}

/// The source `id` that answered.
fn answered(id: SourceId) -> SourceRowViewModel {
    source_row(id, Key::AgentsSourceAnswered, StatusTone::Ok, None)
}

/// The source `id` that answered, with `tip` saying what its answer is.
fn answered_as(id: SourceId, tip: String) -> SourceRowViewModel {
    source_row(id, Key::AgentsSourceAnswered, StatusTone::Ok, Some(tip))
}

/// The source `id` that was not asked, because a better one answered.
fn not_needed(id: SourceId) -> SourceRowViewModel {
    source_row(id, Key::AgentsSourceNotNeeded, StatusTone::Neutral, None)
}

/// The source `id` that gave no answer because of `error`. The tooltip
/// says why, or, for a login the user has to renew, what to do.
fn failed(id: SourceId, error: &SourceError) -> SourceRowViewModel {
    let (state, tone, tip) = match error {
        SourceError::NotLoggedIn => (
            Key::AgentsSourceLoginExpired,
            StatusTone::Warn,
            i18n::t(Key::AgentsLimitsLogInAgain).to_string(),
        ),
        SourceError::LoginExpired(agent) => (
            Key::AgentsSourceLoginExpired,
            StatusTone::Warn,
            renewed_text(agent),
        ),
        SourceError::NotAvailable(reason) => {
            return not_available(id, unavailable_text(reason));
        },
        _ => (
            Key::AgentsSourceFailed,
            StatusTone::Error,
            source_error_text(error),
        ),
    };

    source_row(id, state, tone, Some(tip))
}

/// The source `id` that can not be used here, with why in its tooltip.
fn not_available(id: SourceId, why: String) -> SourceRowViewModel {
    source_row(
        id,
        Key::AgentsSourceNotAvailable,
        StatusTone::Neutral,
        Some(why),
    )
}

/// `row` with its data from `time`, written in the user's `time_zone`.
fn updated_at(
    row: SourceRowViewModel,
    time: SystemTime,
    time_zone: Option<&TimeZone>,
) -> SourceRowViewModel {
    SourceRowViewModel {
        updated: Some(date_time_text(time, time_zone)),
        ..row
    }
}

/// `rows` as the group of the number `number`: its name on the first row
/// only.
fn grouped(
    number: Key,
    rows: Vec<SourceRowViewModel>,
) -> Vec<SourceRowViewModel> {
    rows.into_iter()
        .enumerate()
        .map(|(index, row)| SourceRowViewModel {
            number: (index == 0).then(|| i18n::t(number)),
            ..row
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::Duration;

    use otty_agents::Limits;
    use otty_agents::SourceId::{
        ChatGptUsageApi, CodexAppServer, LatestCodexSession,
    };

    use super::*;
    use crate::testing::{
        at, bill_of, fetched_prices, five_hour_limits, limits_answer,
        price_table, prices_answer, spend_of,
    };

    const SOURCES: [SourceId; 3] =
        [ChatGptUsageApi, CodexAppServer, LatestCodexSession];

    /// 2026-11-16T10:00:00Z.
    const NOV_16: u64 = 1_794_823_200;

    fn read(
        limits: &mut ProfileLimits,
        answers: Vec<(SourceId, Result<Limits, SourceError>)>,
        seconds: u64,
    ) {
        limits.apply(limits_answer(answers), SOURCES.to_vec(), at(seconds));
    }

    /// The limits rows of `limits`, in UTC.
    fn limit_rows(limits: &ProfileLimits) -> Vec<SourceRowViewModel> {
        limit_sources(limits, Some(&TimeZone::UTC))
    }

    /// `row` as the first row of the group of `number`.
    fn first_of(number: Key, row: SourceRowViewModel) -> SourceRowViewModel {
        SourceRowViewModel {
            number: Some(i18n::t(number)),
            ..row
        }
    }

    /// `row` with its data from `time`.
    fn updated(row: SourceRowViewModel, time: &str) -> SourceRowViewModel {
        SourceRowViewModel {
            updated: Some(String::from(time)),
            ..row
        }
    }

    #[test]
    fn every_source_has_its_translated_name_and_tooltip() {
        let names = [
            (
                SourceId::SessionFiles,
                Key::AgentsSourceSessionFiles,
                Key::AgentsSourceTipSessionFiles,
            ),
            (
                SourceId::AnthropicUsageApi,
                Key::AgentsSourceAnthropicUsageApi,
                Key::AgentsSourceTipAnthropicUsageApi,
            ),
            (
                SourceId::ClaudeUsageCommand,
                Key::AgentsSourceClaudeUsageCommand,
                Key::AgentsSourceTipClaudeUsageCommand,
            ),
            (
                SourceId::ChatGptUsageApi,
                Key::AgentsSourceChatgptUsageApi,
                Key::AgentsSourceTipChatgptUsageApi,
            ),
            (
                SourceId::CodexAppServer,
                Key::AgentsSourceCodexAppServer,
                Key::AgentsSourceTipCodexAppServer,
            ),
            (
                SourceId::LatestCodexSession,
                Key::AgentsSourceLatestCodexSession,
                Key::AgentsSourceTipLatestCodexSession,
            ),
            (
                SourceId::AnthropicCostReport,
                Key::AgentsSourceAnthropicCostReport,
                Key::AgentsSourceTipAnthropicCostReport,
            ),
            (
                SourceId::SessionFilesAtListPrices,
                Key::AgentsSourceSessionFilesAtListPrices,
                Key::AgentsSourceTipSessionFilesAtListPrices,
            ),
            (
                SourceId::LiteLlmPrices,
                Key::AgentsSourceLitellmPrices,
                Key::AgentsSourceTipLitellmPrices,
            ),
            (
                SourceId::KeptPrices,
                Key::AgentsSourceKeptPrices,
                Key::AgentsSourceTipKeptPrices,
            ),
        ];

        for (id, name, tip) in names {
            assert_eq!(source_name(id), i18n::t(name), "{id:?}");
            assert_eq!(source_tip(id), i18n::t(tip), "{id:?}");
        }
    }

    #[test]
    fn the_sources_show_who_failed_who_answered_and_who_was_not_needed() {
        let mut limits = ProfileLimits::default();
        read(
            &mut limits,
            vec![
                (ChatGptUsageApi, Err(SourceError::NotLoggedIn)),
                (CodexAppServer, Ok(five_hour_limits(42.0))),
            ],
            NOV_16,
        );

        let rows = limit_rows(&limits);

        assert_eq!(
            rows,
            vec![
                first_of(
                    Key::AgentsNumberLimits,
                    failed(ChatGptUsageApi, &SourceError::NotLoggedIn)
                ),
                updated(answered(CodexAppServer), "Nov 16, 2026, 10:00"),
                not_needed(LatestCodexSession),
            ]
        );
        assert_eq!(rows[0].tone, StatusTone::Warn);
        assert_eq!(rows[0].state, i18n::t(Key::AgentsSourceLoginExpired));
    }

    /// Owner, issue 42: the number a row gives is written once, on the
    /// first row of its group.
    #[test]
    fn the_number_is_written_on_the_first_row_of_its_group_only() {
        let mut limits = ProfileLimits::default();
        read(
            &mut limits,
            vec![(ChatGptUsageApi, Ok(five_hour_limits(1.0)))],
            NOV_16,
        );

        let numbers: Vec<_> =
            limit_rows(&limits).iter().map(|row| row.number).collect();

        assert_eq!(
            numbers,
            [Some(i18n::t(Key::AgentsNumberLimits)), None, None]
        );
    }

    /// Owner, issue 42: each name has a tooltip that says what the source
    /// does.
    #[test]
    fn every_row_has_the_tooltip_of_its_name() {
        let mut limits = ProfileLimits::default();
        read(
            &mut limits,
            vec![(ChatGptUsageApi, Ok(five_hour_limits(1.0)))],
            NOV_16,
        );

        let tips: Vec<_> =
            limit_rows(&limits).iter().map(|row| row.name_tip).collect();

        assert_eq!(
            tips,
            [
                i18n::t(Key::AgentsSourceTipChatgptUsageApi),
                i18n::t(Key::AgentsSourceTipCodexAppServer),
                i18n::t(Key::AgentsSourceTipLatestCodexSession),
            ]
        );
    }

    /// Owner, issue 42: Updated says when the data of the source that
    /// answered is from: the time of the read, or of the record the limits
    /// were taken from.
    #[test]
    fn the_source_that_answered_says_when_its_data_is_from() {
        let mut read_now = ProfileLimits::default();
        read(
            &mut read_now,
            vec![(ChatGptUsageApi, Ok(five_hour_limits(1.0)))],
            NOV_16,
        );
        let an_hour_ago = at(NOV_16) - Duration::from_secs(3600);
        let recorded = five_hour_limits(1.0).with_recorded_at(an_hour_ago);
        let mut from_a_record = ProfileLimits::default();
        read(
            &mut from_a_record,
            vec![
                (ChatGptUsageApi, Err(SourceError::NotLoggedIn)),
                (
                    CodexAppServer,
                    Err(SourceError::NotAvailable(
                        Unavailable::ProgramNotFound(String::from("codex")),
                    )),
                ),
                (LatestCodexSession, Ok(recorded)),
            ],
            NOV_16,
        );

        let times = |limits: &ProfileLimits| -> Vec<Option<String>> {
            limit_rows(limits)
                .into_iter()
                .map(|row| row.updated)
                .collect()
        };

        assert_eq!(
            times(&read_now),
            [Some(String::from("Nov 16, 2026, 10:00")), None, None]
        );
        assert_eq!(
            times(&from_a_record),
            [None, None, Some(String::from("Nov 16, 2026, 09:00"))]
        );
    }

    #[test]
    fn the_tooltip_of_each_failing_state_says_why_or_what_to_do() {
        let mut limits = ProfileLimits::default();
        read(
            &mut limits,
            vec![(ChatGptUsageApi, Ok(five_hour_limits(1.0)))],
            10,
        );
        let network = SourceError::Network(String::from("timed out"));
        let missing = SourceError::NotAvailable(Unavailable::ProgramNotFound(
            String::from("claude"),
        ));
        read(
            &mut limits,
            vec![
                (ChatGptUsageApi, Err(network)),
                (CodexAppServer, Err(missing)),
                (LatestCodexSession, Err(SourceError::NotLoggedIn)),
            ],
            20,
        );

        let states: Vec<_> = limit_rows(&limits)
            .into_iter()
            .map(|row| (row.state, row.tone, row.tip, row.updated))
            .collect();

        assert_eq!(
            states,
            vec![
                (
                    i18n::t(Key::AgentsSourceFailed),
                    StatusTone::Error,
                    Some(String::from("network error: timed out")),
                    None,
                ),
                (
                    i18n::t(Key::AgentsSourceNotAvailable),
                    StatusTone::Neutral,
                    Some(String::from("claude is not found")),
                    None,
                ),
                (
                    i18n::t(Key::AgentsSourceLoginExpired),
                    StatusTone::Warn,
                    Some(i18n::t(Key::AgentsLimitsLogInAgain).to_string()),
                    None,
                ),
            ]
        );
    }

    /// Owner, issue 29: a login the agent renews by itself says so, not
    /// "log in again".
    #[test]
    fn a_login_the_agent_renews_is_expired_with_its_reason() {
        let mut limits = ProfileLimits::default();
        let renews = String::from("Claude Code");
        read(
            &mut limits,
            vec![
                (ChatGptUsageApi, Err(SourceError::LoginExpired(renews))),
                (CodexAppServer, Ok(five_hour_limits(42.0))),
            ],
            10,
        );

        let rows = limit_rows(&limits);

        assert_eq!(rows[0].state, i18n::t(Key::AgentsSourceLoginExpired));
        assert_eq!(rows[0].tone, StatusTone::Warn);
        assert_eq!(
            rows[0].tip,
            Some(String::from("Claude Code renews it when it runs"))
        );
    }

    #[test]
    fn a_source_that_answered_or_was_not_needed_has_no_state_tooltip() {
        let mut limits = ProfileLimits::default();
        read(
            &mut limits,
            vec![(ChatGptUsageApi, Ok(five_hour_limits(1.0)))],
            10,
        );

        let tips: Vec<_> =
            limit_rows(&limits).into_iter().map(|row| row.tip).collect();

        assert_eq!(tips, [None, None, None]);
    }

    #[test]
    fn no_read_and_no_way_to_read_show_no_sources() {
        let mut no_sources = ProfileLimits::default();
        no_sources.apply(
            crate::types::LimitsRead::NoSources,
            Vec::new(),
            at(10),
        );

        assert!(limit_rows(&ProfileLimits::default()).is_empty());
        assert!(limit_rows(&no_sources).is_empty());
    }

    /// Owner, issue 42: the tokens and the session counts have their row
    /// too, with the time of the count.
    #[test]
    fn the_tokens_come_from_the_session_files_with_the_time_of_the_count() {
        let rows = token_sources(&counted(), Some(&TimeZone::UTC));

        assert_eq!(
            rows,
            vec![first_of(
                Key::AgentsTokens,
                updated(
                    answered(SourceId::SessionFiles),
                    "Nov 16, 2026, 10:00"
                )
            )]
        );
    }

    #[test]
    fn a_failed_count_of_the_tokens_says_why() {
        let mut spend = counted();
        let error = SourceError::Unreadable {
            path: PathBuf::from("projects/"),
            detail: String::from("Permission denied"),
        };
        spend.apply(
            crate::types::SpendCount::Failed(error.clone()),
            at(NOV_16 + 60),
        );

        let rows = token_sources(&spend, Some(&TimeZone::UTC));

        assert_eq!(
            rows,
            vec![first_of(
                Key::AgentsTokens,
                failed(SourceId::SessionFiles, &error)
            )]
        );
    }

    #[test]
    fn no_count_yet_and_no_way_to_count_show_no_token_row() {
        let mut no_counter = ProfileSpend::default();
        no_counter.apply(crate::types::SpendCount::NoCounter, at(NOV_16));

        assert!(
            token_sources(&ProfileSpend::default(), Some(&TimeZone::UTC))
                .is_empty()
        );
        assert!(token_sources(&no_counter, Some(&TimeZone::UTC)).is_empty());
    }

    /// The prices fetched from LiteLLM at 2026-11-16T10:00:00Z.
    fn good_prices() -> ListPrices {
        let fetched_at = at(NOV_16);

        let mut prices = ListPrices::default();
        prices.apply(
            prices_answer(&[], Some(fetched_prices(fetched_at))),
            fetched_at,
        );
        prices
    }

    /// A read of the prices whose fetch failed: the kept file of three
    /// days before answered, or there was none.
    fn failed_fetch(kept: bool) -> ListPrices {
        let down = (
            SourceId::LiteLlmPrices,
            SourceError::Network(String::from("down")),
        );
        let answer = if kept {
            let three_days_ago = at(NOV_16 - 3 * 24 * 3600);
            prices_answer(&[down], Some(fetched_prices(three_days_ago)))
        } else {
            let none = (
                SourceId::KeptPrices,
                SourceError::NotAvailable(Unavailable::NoKeptPriceFile),
            );
            prices_answer(&[down, none], None)
        };

        let mut prices = ListPrices::default();
        prices.apply(answer, at(NOV_16));
        prices
    }

    /// The price source `id` that answered, with its file of `time`. Its
    /// state has no tooltip: the tooltip of its name says where the prices
    /// come from (owner, 2026-11-18).
    fn answered_prices(id: SourceId, time: &str) -> SourceRowViewModel {
        updated(answered(id), time)
    }

    /// The money sources of [`counted`] tokens with no bill, at `prices`.
    fn estimate_rows(prices: &ListPrices) -> Vec<SourceRowViewModel> {
        let money = prices
            .table()
            .map(|table| MoneyEstimate::new(&spend_of(1, 2), table));

        money_sources(
            &counted(),
            money.as_ref(),
            prices,
            BillSourceState::NoSource,
            Some(&TimeZone::UTC),
        )
    }

    /// Owner, issues 37 and 42: the prices are a group of their own, and
    /// the time of their file is in Updated, not in the name.
    #[test]
    fn the_estimate_and_the_litellm_prices_are_two_groups() {
        let rows = estimate_rows(&good_prices());

        assert_eq!(
            rows,
            vec![
                first_of(Key::AgentsNumberMoney, answered_estimate()),
                first_of(
                    Key::AgentsNumberPrices,
                    answered_prices(
                        SourceId::LiteLlmPrices,
                        "Nov 16, 2026, 10:00"
                    )
                ),
                not_needed(SourceId::KeptPrices),
            ]
        );
        assert_eq!(rows[1].name, "LiteLLM prices", "{rows:?}");
    }

    #[test]
    fn when_the_fetch_failed_the_kept_prices_answer_with_their_time() {
        let rows = estimate_rows(&failed_fetch(true));

        assert_eq!(
            rows,
            vec![
                first_of(
                    Key::AgentsNumberMoney,
                    SourceRowViewModel {
                        tip: Some(estimate_tip("Nov 13, 2026, 10:00")),
                        ..answered_estimate()
                    }
                ),
                first_of(
                    Key::AgentsNumberPrices,
                    failed(
                        SourceId::LiteLlmPrices,
                        &SourceError::Network(String::from("down"))
                    )
                ),
                answered_prices(SourceId::KeptPrices, "Nov 13, 2026, 10:00"),
            ]
        );
    }

    #[test]
    fn with_no_prices_the_estimate_and_each_price_source_say_why() {
        let rows = estimate_rows(&failed_fetch(false));

        assert_eq!(
            rows,
            vec![
                first_of(
                    Key::AgentsNumberMoney,
                    not_available(
                        SourceId::SessionFilesAtListPrices,
                        i18n::t(Key::AgentsNoPrices).to_string()
                    )
                ),
                first_of(
                    Key::AgentsNumberPrices,
                    failed(
                        SourceId::LiteLlmPrices,
                        &SourceError::Network(String::from("down"))
                    )
                ),
                failed(
                    SourceId::KeptPrices,
                    &SourceError::NotAvailable(Unavailable::NoKeptPriceFile)
                ),
            ]
        );
    }

    #[test]
    fn before_the_prices_are_read_no_estimate_shows() {
        assert!(estimate_rows(&ListPrices::default()).is_empty());
    }

    #[test]
    fn a_failed_count_makes_the_estimate_fail_with_no_prices_listed() {
        let mut failing = ProfileSpend::default();
        let error = SourceError::NotAvailable(Unavailable::NoSessionFiles);
        failing.apply(crate::types::SpendCount::Failed(error.clone()), at(10));

        assert_eq!(
            money_sources(
                &failing,
                None,
                &good_prices(),
                BillSourceState::NoSource,
                Some(&TimeZone::UTC)
            ),
            vec![first_of(
                Key::AgentsNumberMoney,
                failed(SourceId::SessionFilesAtListPrices, &error)
            )]
        );
        assert!(
            money_sources(
                &ProfileSpend::default(),
                None,
                &good_prices(),
                BillSourceState::NoSource,
                Some(&TimeZone::UTC)
            )
            .is_empty()
        );
    }

    /// Tokens counted at 2026-11-16T10:00:00Z.
    fn counted() -> ProfileSpend {
        let mut counted = ProfileSpend::default();
        counted.apply(
            crate::types::SpendCount::Counted(spend_of(1, 2)),
            at(NOV_16),
        );
        counted
    }

    /// The estimate of the tokens of [`counted`] at the made-up prices.
    fn estimate() -> MoneyEstimate {
        MoneyEstimate::new(&spend_of(1, 2), &price_table())
    }

    /// What the estimate at the LiteLLM prices fetched at `time` is.
    fn estimate_tip(time: &str) -> String {
        i18n::filled(Key::TplAgentsMoneyEstimate, &[("time", time)])
    }

    /// The estimate that answered at the prices of [`good_prices`]: its
    /// tooltip says it is an estimate at them, and its data is as old as
    /// the count of the tokens.
    fn answered_estimate() -> SourceRowViewModel {
        updated(
            answered_as(
                SourceId::SessionFilesAtListPrices,
                estimate_tip("Nov 16, 2026, 10:00"),
            ),
            "Nov 16, 2026, 10:00",
        )
    }

    /// The money sources of [`counted`] tokens at good prices, with the
    /// bill as `bill` says.
    fn with_bill(bill: BillSourceState<'_>) -> Vec<SourceRowViewModel> {
        money_sources(
            &counted(),
            Some(&estimate()),
            &good_prices(),
            bill,
            Some(&TimeZone::UTC),
        )
    }

    /// The group of the good prices after the estimate.
    fn price_rows() -> [SourceRowViewModel; 2] {
        [
            first_of(
                Key::AgentsNumberPrices,
                answered_prices(SourceId::LiteLlmPrices, "Nov 16, 2026, 10:00"),
            ),
            not_needed(SourceId::KeptPrices),
        ]
    }

    #[test]
    fn with_no_admin_key_the_bill_is_not_available_and_the_estimate_answers() {
        let rows = with_bill(BillSourceState::NoAdminKey);

        let mut expected = vec![
            first_of(
                Key::AgentsNumberMoney,
                failed(
                    SourceId::AnthropicCostReport,
                    &SourceError::NotAvailable(Unavailable::NoAdminKey),
                ),
            ),
            answered_estimate(),
        ];
        expected.extend(price_rows());
        assert_eq!(rows, expected);
        assert_eq!(rows[0].state, i18n::t(Key::AgentsSourceNotAvailable));
    }

    #[test]
    fn a_bill_that_answered_makes_the_estimate_not_needed() {
        let mut bill = ProfileBill::default();
        bill.apply(Ok(bill_of(1, 2)), at(NOV_16 + 60));

        let rows = with_bill(BillSourceState::Read(&bill));

        assert_eq!(
            rows,
            vec![
                first_of(
                    Key::AgentsNumberMoney,
                    updated(
                        answered_as(
                            SourceId::AnthropicCostReport,
                            i18n::t(Key::AgentsMoneyFromBill).to_string()
                        ),
                        "Nov 16, 2026, 10:01"
                    )
                ),
                not_needed(SourceId::SessionFilesAtListPrices),
            ]
        );
    }

    #[test]
    fn a_refused_key_fails_and_the_estimate_answers() {
        let mut bill = ProfileBill::default();
        bill.apply(Err(SourceError::KeyRefused), at(10));

        let rows = with_bill(BillSourceState::Read(&bill));

        let mut expected = vec![
            first_of(
                Key::AgentsNumberMoney,
                failed(SourceId::AnthropicCostReport, &SourceError::KeyRefused),
            ),
            answered_estimate(),
        ];
        expected.extend(price_rows());
        assert_eq!(rows, expected);
        assert_eq!(rows[0].tip, Some(String::from("the key was refused")));
    }

    #[test]
    fn a_bill_not_read_yet_shows_only_the_estimate() {
        let bill = ProfileBill::default();

        let rows = with_bill(BillSourceState::Read(&bill));

        let mut expected =
            vec![first_of(Key::AgentsNumberMoney, answered_estimate())];
        expected.extend(price_rows());
        assert_eq!(rows, expected);
    }
}
