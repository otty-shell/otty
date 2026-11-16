//! "Where the numbers come from" on a profile's dashboard: each source of
//! its billing in order, with what it did at the latest read (spec
//! Decisions 5 and 6).

use otty_agents::SourceError;
use otty_app_i18n::{self as i18n, Key};
use otty_ui_kit::components::status_badge::StatusTone;

use super::super::bill::ProfileBill;
use super::super::limits::ProfileLimits;
use super::super::spend::ProfileSpend;

/// One source and its state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SourceRowViewModel {
    /// The name of the source, such as "usage request".
    pub(crate) name: String,
    /// What it did: answered, not needed, login expired, not available or
    /// failed.
    pub(crate) state: &'static str,
    /// The color of the state.
    pub(crate) tone: StatusTone,
    /// Why it gave no answer, in short, when it says more than the state.
    pub(crate) reason: Option<String>,
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
                .map(|failure| failed(failure.source(), failure.error()))
                .collect(),
            (None, Some((answer, _))) => answer
                .earlier_failures()
                .iter()
                .map(|failure| failed(failure.source(), failure.error()))
                .chain([answered(answer.source())])
                .collect(),
            (None, None) => return Vec::new(),
        };

    if limits.sources().is_empty() {
        return walked;
    }

    limits
        .sources()
        .iter()
        .map(|name| {
            walked
                .iter()
                .find(|row| row.name == *name)
                .cloned()
                .unwrap_or_else(|| not_needed(name))
        })
        .collect()
}

/// The sources of the money of an API key profile in order: the bill of
/// its organization, when its agent has one, then its tokens at the list
/// prices, which are not needed while the bill answers. The bill shows
/// once it was read, or as not available with no admin key; the estimate
/// once the tokens were counted, and never for an agent that can not
/// count them.
pub(crate) fn money_sources(
    spend: &ProfileSpend,
    bill: BillSourceState<'_>,
) -> Vec<SourceRowViewModel> {
    let bill_name = i18n::t(Key::AgentsBillSource);

    match bill {
        BillSourceState::NoSource => estimate_sources(spend),
        BillSourceState::NoAdminKey => {
            let no_key = i18n::t(Key::AgentsNoAdminKey).to_string();
            let row = failed(bill_name, &SourceError::NotAvailable(no_key));
            [row].into_iter().chain(estimate_sources(spend)).collect()
        },
        BillSourceState::Read(read) => {
            if read.answer().is_some() {
                let estimate = i18n::t(Key::AgentsMoneySource);
                return vec![answered(bill_name), not_needed(estimate)];
            }

            read.failure()
                .map(|error| failed(bill_name, error))
                .into_iter()
                .chain(estimate_sources(spend))
                .collect()
        },
    }
}

/// The estimate as a source of the money: its tokens at the list prices.
/// Empty before the first count and for an agent that can not count
/// tokens.
fn estimate_sources(spend: &ProfileSpend) -> Vec<SourceRowViewModel> {
    let name = i18n::t(Key::AgentsMoneySource);

    if !spend.has_counter() {
        return Vec::new();
    }
    if let Some(error) = spend.failure() {
        return vec![failed(name, error)];
    }

    match spend.last_good() {
        Some(_) => vec![answered(name)],
        None => Vec::new(),
    }
}

/// The source `name` that answered.
fn answered(name: &str) -> SourceRowViewModel {
    SourceRowViewModel {
        name: name.to_string(),
        state: i18n::t(Key::AgentsSourceAnswered),
        tone: StatusTone::Ok,
        reason: None,
    }
}

/// The source `name` that was not asked, because a better one answered.
fn not_needed(name: &str) -> SourceRowViewModel {
    SourceRowViewModel {
        name: name.to_string(),
        state: i18n::t(Key::AgentsSourceNotNeeded),
        tone: StatusTone::Neutral,
        reason: None,
    }
}

/// The source `name` that gave no answer because of `error`.
fn failed(name: &str, error: &SourceError) -> SourceRowViewModel {
    let (state, tone, reason) = match error {
        SourceError::NotLoggedIn => {
            (Key::AgentsSourceLoginExpired, StatusTone::Warn, None)
        },
        SourceError::NotAvailable(reason) => (
            Key::AgentsSourceNotAvailable,
            StatusTone::Neutral,
            Some(reason.clone()),
        ),
        _ => (
            Key::AgentsSourceFailed,
            StatusTone::Error,
            Some(error.to_string()),
        ),
    };

    SourceRowViewModel {
        name: name.to_string(),
        state: i18n::t(state),
        tone,
        reason,
    }
}

#[cfg(test)]
mod tests {
    use otty_agents::Limits;

    use super::*;
    use crate::testing::{
        at, bill_of, five_hour_limits, limits_answer, spend_of,
    };

    const SOURCES: [&str; 3] = ["usage request", "app-server", "session file"];

    fn read(
        limits: &mut ProfileLimits,
        answers: Vec<(&'static str, Result<Limits, SourceError>)>,
        seconds: u64,
    ) {
        limits.apply(limits_answer(answers), SOURCES.to_vec(), at(seconds));
    }

    #[test]
    fn the_sources_show_who_failed_who_answered_and_who_was_not_needed() {
        let mut limits = ProfileLimits::default();
        read(
            &mut limits,
            vec![
                ("usage request", Err(SourceError::NotLoggedIn)),
                ("app-server", Ok(five_hour_limits(42.0))),
            ],
            10,
        );

        let rows = limit_sources(&limits);

        assert_eq!(
            rows,
            vec![
                failed("usage request", &SourceError::NotLoggedIn),
                answered("app-server"),
                not_needed("session file"),
            ]
        );
        assert_eq!(rows[0].tone, StatusTone::Warn);
        assert_eq!(rows[0].state, i18n::t(Key::AgentsSourceLoginExpired));
    }

    #[test]
    fn a_read_where_no_source_answered_shows_each_failure_with_its_reason() {
        let mut limits = ProfileLimits::default();
        read(
            &mut limits,
            vec![("usage request", Ok(five_hour_limits(1.0)))],
            10,
        );
        let network = SourceError::Network(String::from("timed out"));
        let missing = SourceError::NotAvailable(String::from("no program"));
        let odd = SourceError::UnexpectedAnswer(String::from("no json"));
        read(
            &mut limits,
            vec![
                ("usage request", Err(network.clone())),
                ("app-server", Err(missing.clone())),
                ("session file", Err(odd.clone())),
            ],
            20,
        );

        let rows = limit_sources(&limits);

        assert_eq!(
            rows,
            vec![
                SourceRowViewModel {
                    name: String::from("usage request"),
                    state: i18n::t(Key::AgentsSourceFailed),
                    tone: StatusTone::Error,
                    reason: Some(String::from("network error: timed out")),
                },
                SourceRowViewModel {
                    name: String::from("app-server"),
                    state: i18n::t(Key::AgentsSourceNotAvailable),
                    tone: StatusTone::Neutral,
                    reason: Some(String::from("no program")),
                },
                failed("session file", &odd),
            ]
        );
    }

    #[test]
    fn no_read_and_no_way_to_read_show_no_sources() {
        let mut no_sources = ProfileLimits::default();
        no_sources.apply(Err(Vec::new()), Vec::new(), at(10));

        assert!(limit_sources(&ProfileLimits::default()).is_empty());
        assert!(limit_sources(&no_sources).is_empty());
    }

    #[test]
    fn the_money_comes_from_the_session_files_at_list_prices() {
        let mut counted = ProfileSpend::default();
        counted.apply(Some(Ok(spend_of(1, 2))), at(10));
        let mut failing = ProfileSpend::default();
        let error = SourceError::NotAvailable(String::from("no folder"));
        failing.apply(Some(Err(error.clone())), at(10));

        let source = i18n::t(Key::AgentsMoneySource);
        assert_eq!(
            money_sources(&counted, BillSourceState::NoSource),
            vec![answered(source)]
        );
        assert_eq!(
            money_sources(&failing, BillSourceState::NoSource),
            vec![failed(source, &error)]
        );
        assert!(
            money_sources(&ProfileSpend::default(), BillSourceState::NoSource)
                .is_empty()
        );
    }

    fn counted() -> ProfileSpend {
        let mut counted = ProfileSpend::default();
        counted.apply(Some(Ok(spend_of(1, 2))), at(10));
        counted
    }

    #[test]
    fn with_no_admin_key_the_bill_is_not_available_and_the_estimate_answers() {
        let rows = money_sources(&counted(), BillSourceState::NoAdminKey);

        assert_eq!(
            rows,
            vec![
                SourceRowViewModel {
                    name: i18n::t(Key::AgentsBillSource).to_string(),
                    state: i18n::t(Key::AgentsSourceNotAvailable),
                    tone: StatusTone::Neutral,
                    reason: Some(i18n::t(Key::AgentsNoAdminKey).to_string()),
                },
                answered(i18n::t(Key::AgentsMoneySource)),
            ]
        );
    }

    #[test]
    fn a_bill_that_answered_makes_the_estimate_not_needed() {
        let mut bill = ProfileBill::default();
        bill.apply(Ok(bill_of(1, 2)));

        let rows = money_sources(&counted(), BillSourceState::Read(&bill));

        assert_eq!(
            rows,
            vec![
                answered(i18n::t(Key::AgentsBillSource)),
                not_needed(i18n::t(Key::AgentsMoneySource)),
            ]
        );
    }

    #[test]
    fn a_refused_key_fails_and_the_estimate_answers() {
        let mut bill = ProfileBill::default();
        bill.apply(Err(SourceError::KeyRefused));

        let rows = money_sources(&counted(), BillSourceState::Read(&bill));

        assert_eq!(
            rows,
            vec![
                SourceRowViewModel {
                    name: i18n::t(Key::AgentsBillSource).to_string(),
                    state: i18n::t(Key::AgentsSourceFailed),
                    tone: StatusTone::Error,
                    reason: Some(String::from("the key was refused")),
                },
                answered(i18n::t(Key::AgentsMoneySource)),
            ]
        );
    }

    #[test]
    fn a_bill_not_read_yet_shows_only_the_estimate() {
        let bill = ProfileBill::default();

        let rows = money_sources(&counted(), BillSourceState::Read(&bill));

        assert_eq!(rows, vec![answered(i18n::t(Key::AgentsMoneySource))]);
    }
}
