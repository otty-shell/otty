//! "Where the numbers come from" on a profile's dashboard: each source of
//! its billing in order, with what it did at the latest read (spec
//! Decisions 5 and 6).

use otty_agents::{
    FetchedPrices, MoneyEstimate, SourceError, SourceFailure, SourceId,
};
use otty_app_i18n::{self as i18n, Key};
use otty_ui_kit::components::status_badge::StatusTone;

use super::super::bill::ProfileBill;
use super::super::limits::ProfileLimits;
use super::super::prices::ListPrices;
use super::super::spend::ProfileSpend;
use super::reset_time::date_time_text;

/// The sources of the list prices, best first (spec Decision 16).
const PRICE_SOURCES: [SourceId; 2] =
    [SourceId::LiteLlmPrices, SourceId::KeptPrices];

/// One source and its state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SourceRowViewModel {
    /// The name of the source, such as "Anthropic usage API", or for the
    /// prices that answered, with the time of their file: "LiteLLM prices,
    /// updated Nov 17, 2026, 10:00".
    pub(crate) name: String,
    /// What it did: answered, not needed, login expired, not available or
    /// failed.
    pub(crate) state: &'static str,
    /// The color of the state.
    pub(crate) tone: StatusTone,
    /// The tooltip of the state: why the source gave no answer, or what
    /// to do about it.
    pub(crate) tip: Option<String>,
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

/// The name a person reads of the source `id`, such as "Anthropic usage
/// API", in the user's language.
pub(crate) fn source_name(id: SourceId) -> &'static str {
    let key = match id {
        SourceId::SessionFiles => Key::AgentsSourceSessionFiles,
        SourceId::AnthropicUsageApi => Key::AgentsSourceAnthropicUsageApi,
        SourceId::ClaudeUsageCommand => Key::AgentsSourceClaudeUsageCommand,
        SourceId::ChatGptUsageApi => Key::AgentsSourceChatgptUsageApi,
        SourceId::CodexAppServer => Key::AgentsSourceCodexAppServer,
        SourceId::LatestCodexSession => Key::AgentsSourceLatestCodexSession,
        SourceId::AnthropicCostReport => Key::AgentsSourceAnthropicCostReport,
        SourceId::SessionFilesAtListPrices => {
            Key::AgentsSourceSessionFilesAtListPrices
        },
        SourceId::LiteLlmPrices => Key::AgentsSourceLitellmPrices,
        SourceId::KeptPrices => Key::AgentsSourceKeptPrices,
    };

    i18n::t(key)
}

/// The sources of the limits as the latest read walked them: the ones that
/// failed, the one that answered, and those after it as not needed. Empty
/// before the first read and for an agent with no way to read limits.
pub(crate) fn limit_sources(limits: &ProfileLimits) -> Vec<SourceRowViewModel> {
    if !limits.has_sources() {
        return Vec::new();
    }

    let walked: Vec<SourceRowViewModel> =
        match (limits.failures(), limits.last_good()) {
            (Some(failures), _) => failures
                .iter()
                .map(|failure| {
                    failed(source_name(failure.source()), failure.error())
                })
                .collect(),
            (None, Some((answer, _))) => answer
                .earlier_failures()
                .iter()
                .map(|failure| {
                    failed(source_name(failure.source()), failure.error())
                })
                .chain([answered(source_name(answer.source()))])
                .collect(),
            (None, None) => return Vec::new(),
        };

    if limits.sources().is_empty() {
        return walked;
    }

    limits
        .sources()
        .iter()
        .map(|id| {
            let name = source_name(*id);
            walked
                .iter()
                .find(|row| row.name == name)
                .cloned()
                .unwrap_or_else(|| not_needed(name))
        })
        .collect()
}

/// The sources of the money of an API key profile in order: the bill of
/// its organization, when its agent has one, then its tokens at the list
/// prices of the `money` estimate, which are not needed while the bill
/// answers, then the sources of those `prices` (spec Decision 16). The bill
/// shows once it was read, or as not available with no admin key; the
/// estimate once the tokens were counted and the prices read, and never for
/// an agent that can not count them. The tooltip of the source that
/// answered says what the money is: the bill of the whole organization, or
/// an estimate at the LiteLLM prices of a time, written in a time zone
/// `utc_offset_seconds` east of UTC (spec Decision 12).
pub(crate) fn money_sources(
    spend: &ProfileSpend,
    money: Option<&MoneyEstimate>,
    prices: &ListPrices,
    bill: BillSourceState<'_>,
    utc_offset_seconds: Option<i32>,
) -> Vec<SourceRowViewModel> {
    let bill_name = source_name(SourceId::AnthropicCostReport);
    let estimate =
        || estimate_sources(spend, money, prices, utc_offset_seconds);

    match bill {
        BillSourceState::NoSource => estimate(),
        BillSourceState::NoAdminKey => {
            let no_key = i18n::t(Key::AgentsNoAdminKey).to_string();
            let row = failed(bill_name, &SourceError::NotAvailable(no_key));
            [row].into_iter().chain(estimate()).collect()
        },
        BillSourceState::Read(read) => {
            if read.answer().is_some() {
                let from_bill = i18n::t(Key::AgentsMoneyFromBill).to_string();
                let estimate = source_name(SourceId::SessionFilesAtListPrices);
                return vec![
                    answered_as(bill_name, from_bill),
                    not_needed(estimate),
                ];
            }

            read.failure()
                .map(|error| failed(bill_name, error))
                .into_iter()
                .chain(estimate())
                .collect()
        },
    }
}

/// The estimate as a source of the money, its tokens at the list prices of
/// `money`, then the sources of the `prices`. Empty before the first count
/// or the first read of the prices, and for an agent that can not count
/// tokens. With no prices from any source, the estimate is not available
/// and the price sources say why.
fn estimate_sources(
    spend: &ProfileSpend,
    money: Option<&MoneyEstimate>,
    prices: &ListPrices,
    utc_offset_seconds: Option<i32>,
) -> Vec<SourceRowViewModel> {
    let name = source_name(SourceId::SessionFilesAtListPrices);

    if !spend.has_counter() {
        return Vec::new();
    }
    if let Some(error) = spend.failure() {
        return vec![failed(name, error)];
    }
    if spend.last_good().is_none() {
        return Vec::new();
    }

    let estimate = match (money, prices.last_good()) {
        (Some(_), Some(fetched)) => {
            answered_as(name, estimate_text(fetched, utc_offset_seconds))
        },
        _ if prices.has_failed() => {
            let no_prices = i18n::t(Key::AgentsNoPrices).to_string();
            failed(name, &SourceError::NotAvailable(no_prices))
        },
        _ => return Vec::new(),
    };

    [estimate]
        .into_iter()
        .chain(price_sources(prices, utc_offset_seconds))
        .collect()
}

/// The sources of the list prices as their latest read walked them: the
/// ones that failed, the one that answered with the time of its file, and
/// those after it as not needed.
fn price_sources(
    prices: &ListPrices,
    utc_offset_seconds: Option<i32>,
) -> Vec<SourceRowViewModel> {
    let failed_row = |failure: &SourceFailure| {
        failed(source_name(failure.source()), failure.error())
    };

    match prices.latest() {
        Some(Ok(answer)) => {
            let source = answer.source();
            let name = i18n::agents_template(
                Key::TplAgentsSourceUpdated,
                &[
                    ("name", source_name(source)),
                    (
                        "time",
                        &date_time_text(
                            answer.value().fetched_at(),
                            utc_offset_seconds,
                        ),
                    ),
                ],
            );
            let answered = SourceRowViewModel {
                name,
                state: i18n::t(Key::AgentsSourceAnswered),
                tone: StatusTone::Ok,
                tip: Some(i18n::t(Key::AgentsPricesFrom).to_string()),
            };
            let after = PRICE_SOURCES
                .iter()
                .skip_while(|id| **id != source)
                .skip(1)
                .map(|id| not_needed(source_name(*id)));

            answer
                .earlier_failures()
                .iter()
                .map(failed_row)
                .chain([answered])
                .chain(after)
                .collect()
        },
        Some(Err(failures)) => failures.iter().map(failed_row).collect(),
        None => Vec::new(),
    }
}

/// What the estimate is: "Tokens from the session files at the LiteLLM
/// prices updated Nov 17, 2026, 10:00, an estimate".
fn estimate_text(
    prices: &FetchedPrices,
    utc_offset_seconds: Option<i32>,
) -> String {
    let time = date_time_text(prices.fetched_at(), utc_offset_seconds);

    i18n::agents_template(Key::TplAgentsMoneyEstimate, &[("time", &time)])
}

/// The source `name` that answered.
fn answered(name: &str) -> SourceRowViewModel {
    SourceRowViewModel {
        name: name.to_string(),
        state: i18n::t(Key::AgentsSourceAnswered),
        tone: StatusTone::Ok,
        tip: None,
    }
}

/// The source `name` that answered, with `tip` saying what its answer is.
fn answered_as(name: &str, tip: String) -> SourceRowViewModel {
    SourceRowViewModel {
        tip: Some(tip),
        ..answered(name)
    }
}

/// The source `name` that was not asked, because a better one answered.
fn not_needed(name: &str) -> SourceRowViewModel {
    SourceRowViewModel {
        name: name.to_string(),
        state: i18n::t(Key::AgentsSourceNotNeeded),
        tone: StatusTone::Neutral,
        tip: None,
    }
}

/// The source `name` that gave no answer because of `error`. The tooltip
/// says why, or, for a login the user has to renew, what to do.
fn failed(name: &str, error: &SourceError) -> SourceRowViewModel {
    let (state, tone, tip) = match error {
        SourceError::NotLoggedIn => (
            Key::AgentsSourceLoginExpired,
            StatusTone::Warn,
            i18n::t(Key::AgentsLimitsLogInAgain).to_string(),
        ),
        SourceError::LoginExpired(reason) => (
            Key::AgentsSourceLoginExpired,
            StatusTone::Warn,
            reason.clone(),
        ),
        SourceError::NotAvailable(reason) => (
            Key::AgentsSourceNotAvailable,
            StatusTone::Neutral,
            reason.clone(),
        ),
        _ => (
            Key::AgentsSourceFailed,
            StatusTone::Error,
            error.to_string(),
        ),
    };

    SourceRowViewModel {
        name: name.to_string(),
        state: i18n::t(state),
        tone,
        tip: Some(tip),
    }
}

#[cfg(test)]
mod tests {
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

    fn read(
        limits: &mut ProfileLimits,
        answers: Vec<(SourceId, Result<Limits, SourceError>)>,
        seconds: u64,
    ) {
        limits.apply(limits_answer(answers), SOURCES.to_vec(), at(seconds));
    }

    #[test]
    fn every_source_has_its_translated_name() {
        let names = [
            (SourceId::SessionFiles, Key::AgentsSourceSessionFiles),
            (
                SourceId::AnthropicUsageApi,
                Key::AgentsSourceAnthropicUsageApi,
            ),
            (
                SourceId::ClaudeUsageCommand,
                Key::AgentsSourceClaudeUsageCommand,
            ),
            (SourceId::ChatGptUsageApi, Key::AgentsSourceChatgptUsageApi),
            (SourceId::CodexAppServer, Key::AgentsSourceCodexAppServer),
            (
                SourceId::LatestCodexSession,
                Key::AgentsSourceLatestCodexSession,
            ),
            (
                SourceId::AnthropicCostReport,
                Key::AgentsSourceAnthropicCostReport,
            ),
            (
                SourceId::SessionFilesAtListPrices,
                Key::AgentsSourceSessionFilesAtListPrices,
            ),
            (SourceId::LiteLlmPrices, Key::AgentsSourceLitellmPrices),
            (SourceId::KeptPrices, Key::AgentsSourceKeptPrices),
        ];

        for (id, key) in names {
            assert_eq!(source_name(id), i18n::t(key), "{id:?}");
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
            10,
        );

        let rows = limit_sources(&limits);

        assert_eq!(
            rows,
            vec![
                failed(source_name(ChatGptUsageApi), &SourceError::NotLoggedIn),
                answered(source_name(CodexAppServer)),
                not_needed(source_name(LatestCodexSession)),
            ]
        );
        assert_eq!(rows[0].tone, StatusTone::Warn);
        assert_eq!(rows[0].state, i18n::t(Key::AgentsSourceLoginExpired));
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
        let missing = SourceError::NotAvailable(String::from("no program"));
        read(
            &mut limits,
            vec![
                (ChatGptUsageApi, Err(network)),
                (CodexAppServer, Err(missing)),
                (LatestCodexSession, Err(SourceError::NotLoggedIn)),
            ],
            20,
        );

        let rows = limit_sources(&limits);

        assert_eq!(
            rows,
            vec![
                SourceRowViewModel {
                    name: source_name(ChatGptUsageApi).to_string(),
                    state: i18n::t(Key::AgentsSourceFailed),
                    tone: StatusTone::Error,
                    tip: Some(String::from("network error: timed out")),
                },
                SourceRowViewModel {
                    name: source_name(CodexAppServer).to_string(),
                    state: i18n::t(Key::AgentsSourceNotAvailable),
                    tone: StatusTone::Neutral,
                    tip: Some(String::from("no program")),
                },
                SourceRowViewModel {
                    name: source_name(LatestCodexSession).to_string(),
                    state: i18n::t(Key::AgentsSourceLoginExpired),
                    tone: StatusTone::Warn,
                    tip: Some(i18n::t(Key::AgentsLimitsLogInAgain).to_string()),
                },
            ]
        );
    }

    /// Owner, issue 29: a login the agent renews by itself says so, not
    /// "log in again".
    #[test]
    fn a_login_the_agent_renews_is_expired_with_its_reason() {
        let mut limits = ProfileLimits::default();
        let renews = String::from("Claude Code renews it when it runs");
        read(
            &mut limits,
            vec![
                (ChatGptUsageApi, Err(SourceError::LoginExpired(renews))),
                (CodexAppServer, Ok(five_hour_limits(42.0))),
            ],
            10,
        );

        let rows = limit_sources(&limits);

        assert_eq!(
            rows[0],
            SourceRowViewModel {
                name: source_name(ChatGptUsageApi).to_string(),
                state: i18n::t(Key::AgentsSourceLoginExpired),
                tone: StatusTone::Warn,
                tip: Some(String::from("Claude Code renews it when it runs")),
            }
        );
    }

    #[test]
    fn a_source_that_answered_or_was_not_needed_has_no_tooltip() {
        let mut limits = ProfileLimits::default();
        read(
            &mut limits,
            vec![(ChatGptUsageApi, Ok(five_hour_limits(1.0)))],
            10,
        );

        let tips: Vec<_> = limit_sources(&limits)
            .into_iter()
            .map(|row| row.tip)
            .collect();

        assert_eq!(tips, [None, None, None]);
    }

    #[test]
    fn no_read_and_no_way_to_read_show_no_sources() {
        let mut no_sources = ProfileLimits::default();
        no_sources.apply(Err(Vec::new()), Vec::new(), at(10));

        assert!(limit_sources(&ProfileLimits::default()).is_empty());
        assert!(limit_sources(&no_sources).is_empty());
    }

    /// 2026-11-16T10:00:00Z.
    const NOV_16: u64 = 1_794_823_200;

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
                SourceError::NotAvailable(String::from("no kept price file")),
            );
            prices_answer(&[down, none], None)
        };

        let mut prices = ListPrices::default();
        prices.apply(answer, at(NOV_16));
        prices
    }

    /// The name of the price source `id` with the time of its file.
    fn updated(id: SourceId, time: &str) -> String {
        i18n::agents_template(
            Key::TplAgentsSourceUpdated,
            &[("name", source_name(id)), ("time", time)],
        )
    }

    /// The price source `id` that answered, with its file of `time`.
    fn answered_prices(id: SourceId, time: &str) -> SourceRowViewModel {
        SourceRowViewModel {
            name: updated(id, time),
            state: i18n::t(Key::AgentsSourceAnswered),
            tone: StatusTone::Ok,
            tip: Some(i18n::t(Key::AgentsPricesFrom).to_string()),
        }
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
            Some(0),
        )
    }

    /// Owner, issue 37: the sources name the prices and when they were
    /// fetched.
    #[test]
    fn the_estimate_names_the_litellm_prices_and_when_they_were_fetched() {
        let rows = estimate_rows(&good_prices());

        assert_eq!(
            rows,
            vec![
                answered_estimate("Nov 16, 2026, 10:00"),
                answered_prices(SourceId::LiteLlmPrices, "Nov 16, 2026, 10:00"),
                not_needed(source_name(SourceId::KeptPrices)),
            ]
        );
        assert_eq!(
            rows[1].name, "LiteLLM prices, updated Nov 16, 2026, 10:00",
            "{rows:?}"
        );
    }

    #[test]
    fn when_the_fetch_failed_the_kept_prices_answer_with_their_time() {
        let rows = estimate_rows(&failed_fetch(true));

        assert_eq!(
            rows,
            vec![
                answered_estimate("Nov 13, 2026, 10:00"),
                failed(
                    source_name(SourceId::LiteLlmPrices),
                    &SourceError::Network(String::from("down"))
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
                failed(
                    source_name(SourceId::SessionFilesAtListPrices),
                    &SourceError::NotAvailable(
                        i18n::t(Key::AgentsNoPrices).to_string()
                    )
                ),
                failed(
                    source_name(SourceId::LiteLlmPrices),
                    &SourceError::Network(String::from("down"))
                ),
                failed(
                    source_name(SourceId::KeptPrices),
                    &SourceError::NotAvailable(String::from(
                        "no kept price file"
                    ))
                ),
            ]
        );
    }

    #[test]
    fn before_the_prices_are_read_no_estimate_shows() {
        assert!(estimate_rows(&ListPrices::default()).is_empty());
    }

    #[test]
    fn the_money_comes_from_the_session_files_at_list_prices() {
        let mut failing = ProfileSpend::default();
        let error = SourceError::NotAvailable(String::from("no folder"));
        failing.apply(Some(Err(error.clone())), at(10));

        let source = source_name(SourceId::SessionFilesAtListPrices);
        assert_eq!(
            money_sources(
                &failing,
                None,
                &good_prices(),
                BillSourceState::NoSource,
                Some(0)
            ),
            vec![failed(source, &error)]
        );
        assert!(
            money_sources(
                &ProfileSpend::default(),
                None,
                &good_prices(),
                BillSourceState::NoSource,
                Some(0)
            )
            .is_empty()
        );
    }

    fn counted() -> ProfileSpend {
        let mut counted = ProfileSpend::default();
        counted.apply(Some(Ok(spend_of(1, 2))), at(10));
        counted
    }

    /// The estimate of the tokens of [`counted`] at the made-up prices.
    fn estimate() -> MoneyEstimate {
        MoneyEstimate::new(&spend_of(1, 2), &price_table())
    }

    /// The estimate that answered: its tooltip says it is an estimate at
    /// the LiteLLM prices fetched at `time`.
    fn answered_estimate(time: &str) -> SourceRowViewModel {
        SourceRowViewModel {
            name: source_name(SourceId::SessionFilesAtListPrices).to_string(),
            state: i18n::t(Key::AgentsSourceAnswered),
            tone: StatusTone::Ok,
            tip: Some(i18n::agents_template(
                Key::TplAgentsMoneyEstimate,
                &[("time", time)],
            )),
        }
    }

    /// The money sources of [`counted`] tokens at good prices, with the
    /// bill as `bill` says.
    fn with_bill(bill: BillSourceState<'_>) -> Vec<SourceRowViewModel> {
        money_sources(
            &counted(),
            Some(&estimate()),
            &good_prices(),
            bill,
            Some(0),
        )
    }

    /// The rows of the good prices after the estimate.
    fn price_rows() -> [SourceRowViewModel; 2] {
        [
            answered_prices(SourceId::LiteLlmPrices, "Nov 16, 2026, 10:00"),
            not_needed(source_name(SourceId::KeptPrices)),
        ]
    }

    #[test]
    fn with_no_admin_key_the_bill_is_not_available_and_the_estimate_answers() {
        let rows = with_bill(BillSourceState::NoAdminKey);

        let mut expected = vec![
            SourceRowViewModel {
                name: source_name(SourceId::AnthropicCostReport).to_string(),
                state: i18n::t(Key::AgentsSourceNotAvailable),
                tone: StatusTone::Neutral,
                tip: Some(i18n::t(Key::AgentsNoAdminKey).to_string()),
            },
            answered_estimate("Nov 16, 2026, 10:00"),
        ];
        expected.extend(price_rows());
        assert_eq!(rows, expected);
    }

    #[test]
    fn a_bill_that_answered_makes_the_estimate_not_needed() {
        let mut bill = ProfileBill::default();
        bill.apply(Ok(bill_of(1, 2)), at(10));

        let rows = with_bill(BillSourceState::Read(&bill));

        assert_eq!(
            rows,
            vec![
                SourceRowViewModel {
                    name: source_name(SourceId::AnthropicCostReport)
                        .to_string(),
                    state: i18n::t(Key::AgentsSourceAnswered),
                    tone: StatusTone::Ok,
                    tip: Some(i18n::t(Key::AgentsMoneyFromBill).to_string()),
                },
                not_needed(source_name(SourceId::SessionFilesAtListPrices)),
            ]
        );
    }

    #[test]
    fn a_refused_key_fails_and_the_estimate_answers() {
        let mut bill = ProfileBill::default();
        bill.apply(Err(SourceError::KeyRefused), at(10));

        let rows = with_bill(BillSourceState::Read(&bill));

        let mut expected = vec![
            SourceRowViewModel {
                name: source_name(SourceId::AnthropicCostReport).to_string(),
                state: i18n::t(Key::AgentsSourceFailed),
                tone: StatusTone::Error,
                tip: Some(String::from("the key was refused")),
            },
            answered_estimate("Nov 16, 2026, 10:00"),
        ];
        expected.extend(price_rows());
        assert_eq!(rows, expected);
    }

    #[test]
    fn a_bill_not_read_yet_shows_only_the_estimate() {
        let bill = ProfileBill::default();

        let rows = with_bill(BillSourceState::Read(&bill));

        let mut expected = vec![answered_estimate("Nov 16, 2026, 10:00")];
        expected.extend(price_rows());
        assert_eq!(rows, expected);
    }
}
