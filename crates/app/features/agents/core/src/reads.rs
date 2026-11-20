//! Which reads Agents mode asks for, and when (spec Decision 10).
//!
//! At the start of OTTY the mode is not on screen, yet it reads what it
//! shows: the profiles, then the sessions of each profile, then the limits,
//! then the tokens, then the list prices, then the money, one kind after
//! another so the machine is not loaded with all of them at once. The
//! limits and the bills are read once per account. The list
//! prices are read only while a profile pays with an API key, at most once
//! a day, and again after a failed fetch (spec Decision 16). The limits read then never
//! starts the agent's program: neither `codex app-server` nor `claude` in a
//! hidden terminal. On screen, a read of the profiles asks for every kind
//! at once, as the Refresh button does. Opening the mode reads again only
//! what is older than the period of its timer, the limits a source failed
//! while the program could not start, and the sessions of a profile the
//! New session dialog started a session of.

use std::time::SystemTime;

use super::event::AgentsOutcome;
use super::freshness::Which;
use super::state::AgentsState;

/// A kind of read of Agents mode, in the order the start of OTTY reads
/// them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReadKind {
    /// `agents.json` and the state of each profile.
    Profiles,
    /// The sessions of each profile.
    Sessions,
    /// The limits of each account of a connected profile.
    Limits,
    /// The tokens each profile spent.
    Spend,
    /// The list prices of the money estimate, while a profile pays with an
    /// API key.
    Prices,
    /// The bill of each organization with an admin key.
    Bills,
    /// The numbers of each account that tells its own with its own key:
    /// money left, a key limit, the bill of an organization the user
    /// added, the limits of a ChatGPT login no profile reads.
    AccountNumbers,
}

impl ReadKind {
    /// The kind read after this one at the start; `None` after the last.
    fn next(self) -> Option<Self> {
        match self {
            Self::Profiles => Some(Self::Sessions),
            Self::Sessions => Some(Self::Limits),
            Self::Limits => Some(Self::Spend),
            Self::Spend => Some(Self::Prices),
            Self::Prices => Some(Self::Bills),
            Self::Bills => Some(Self::AccountNumbers),
            Self::AccountNumbers => None,
        }
    }
}

/// OTTY started: read the profiles with the mode away, unless they were
/// read or a read is on its way; the other kinds follow one after another.
pub(crate) fn start_in_background(
    state: &mut AgentsState,
) -> Vec<AgentsOutcome> {
    let profiles = state.profiles();
    if state.is_active() || profiles.is_loaded() || profiles.is_loading() {
        return Vec::new();
    }

    state.set_startup_reads(Some(ReadKind::Profiles));
    load_profiles(state)
}

/// Agents mode opened at `now`: the reads of the start that did not begin
/// yet begin at once, and what is older than the period is read again, as
/// are the sessions of a profile a new session was started with.
pub(crate) fn on_open(
    state: &mut AgentsState,
    now: SystemTime,
) -> Vec<AgentsOutcome> {
    state.set_startup_reads(None);

    if state.profiles().is_loading() {
        // The profiles read on its way asks for every kind when it ends.
        return Vec::new();
    }
    if !state.profiles().are_fresh(now) {
        return load_profiles(state);
    }

    let mut outcomes: Vec<AgentsOutcome> = state
        .profiles_mut()
        .start_wanted_session_reads()
        .into_iter()
        .map(AgentsOutcome::LoadSessions)
        .collect();
    outcomes.extend(load_after_sessions(state, Which::Stale(now), now));
    outcomes
}

/// The profiles were read at `now` and their sessions asked for: at the
/// start the next kind follows once the sessions are read, else every kind
/// is read now.
pub(crate) fn after_profiles(
    state: &mut AgentsState,
    now: SystemTime,
) -> Vec<AgentsOutcome> {
    if state.startup_reads().is_some() {
        state.set_startup_reads(Some(ReadKind::Sessions));
        return go_on_at_start(state, now);
    }

    load_after_sessions(state, Which::Every, now)
}

/// The limits of a profile were read at `now`: on screen, limits that
/// wait for the agent's program are read again with it; at the start the
/// next kind follows once every limit is read.
pub(crate) fn after_limits(
    state: &mut AgentsState,
    now: SystemTime,
) -> Vec<AgentsOutcome> {
    if state.is_active() {
        return load_limits(state, Which::WaitingForPrograms);
    }

    go_on_at_start(state, now)
}

/// A read of the start ended at `now`: once no read of the kind of the
/// start is on its way, ask for the next kind that has something to read.
pub(crate) fn go_on_at_start(
    state: &mut AgentsState,
    now: SystemTime,
) -> Vec<AgentsOutcome> {
    while let Some(kind) = state.startup_reads() {
        if state.profiles().is_reading_any(kind) {
            return Vec::new();
        }

        let next = kind.next();
        state.set_startup_reads(next);

        let outcomes = match next {
            Some(ReadKind::Limits) => load_limits(state, Which::Every),
            Some(ReadKind::Spend) => load_spend(state, Which::Every),
            Some(ReadKind::Prices) => load_prices(state, Which::Every, now),
            Some(ReadKind::Bills) => load_bills(state, Which::Every),
            Some(ReadKind::AccountNumbers) => {
                load_account_numbers(state, Which::Every)
            },
            _ => Vec::new(),
        };
        if !outcomes.is_empty() {
            return outcomes;
        }
    }

    Vec::new()
}

/// The timer ticked: while the mode is on screen, read the limits and the
/// bills of every profile and the numbers of the accounts that tell their
/// own again.
pub(crate) fn on_tick(state: &mut AgentsState) -> Vec<AgentsOutcome> {
    if !state.is_active() {
        return Vec::new();
    }

    let mut outcomes = load_limits(state, Which::Every);
    outcomes.extend(load_bills(state, Which::Every));
    outcomes.extend(load_account_numbers(state, Which::Every));
    outcomes
}

/// Ask for the numbers of each account that tells its own with its own
/// key and that `which` picks, unless a read of it is on its way. A key in
/// the system keyring is read only while the mode is on screen.
pub(crate) fn load_account_numbers(
    state: &mut AgentsState,
    which: Which,
) -> Vec<AgentsOutcome> {
    let is_on_screen = state.is_active();

    state
        .profiles_mut()
        .start_own_reads(which, is_on_screen)
        .into_iter()
        .map(|(account, place)| AgentsOutcome::LoadAccountNumbers {
            account,
            place,
        })
        .collect()
}

/// Ask for the profiles, unless a read is on its way.
pub(crate) fn load_profiles(state: &mut AgentsState) -> Vec<AgentsOutcome> {
    if !state.profiles_mut().start_loading() {
        return Vec::new();
    }

    vec![AgentsOutcome::LoadProfiles]
}

/// Ask at once for every kind read after the sessions that `which` wants
/// at `now`: the limits, the tokens, the list prices and the bills.
fn load_after_sessions(
    state: &mut AgentsState,
    which: Which,
    now: SystemTime,
) -> Vec<AgentsOutcome> {
    let mut outcomes = load_limits(state, which);
    outcomes.extend(load_spend(state, which));
    outcomes.extend(load_prices(state, which, now));
    outcomes.extend(load_bills(state, which));
    outcomes.extend(load_account_numbers(state, which));
    outcomes
}

/// Ask for the limits of each connected profile `which` picks that has no
/// read on its way: with the agent's program while the mode is on screen,
/// without it while the mode is away.
fn load_limits(state: &mut AgentsState, which: Which) -> Vec<AgentsOutcome> {
    let is_on_screen = state.is_active();

    state
        .profiles_mut()
        .start_limit_reads(which, is_on_screen)
        .into_iter()
        .map(|profile| {
            if is_on_screen {
                AgentsOutcome::LoadLimits(profile)
            } else {
                AgentsOutcome::LoadLimitsInBackground(profile)
            }
        })
        .collect()
}

/// Ask for the tokens of each shown profile `which` picks that has no
/// count on its way.
fn load_spend(state: &mut AgentsState, which: Which) -> Vec<AgentsOutcome> {
    state
        .profiles_mut()
        .start_spend_reads(which)
        .into_iter()
        .map(AgentsOutcome::LoadSpend)
        .collect()
}

/// Ask for the list prices when a profile pays with an API key and
/// `which` wants them at `now`, unless a read is on its way: once a day,
/// and again after a failed fetch (spec Decision 16).
fn load_prices(
    state: &mut AgentsState,
    which: Which,
    now: SystemTime,
) -> Vec<AgentsOutcome> {
    state
        .profiles_mut()
        .start_price_read(which, now)
        .map(|fetch_again| AgentsOutcome::LoadPrices { fetch_again })
        .into_iter()
        .collect()
}

/// Ask for the bill of each organization with an admin key that an API
/// key profile `which` picks pays through, once per organization.
fn load_bills(state: &mut AgentsState, which: Which) -> Vec<AgentsOutcome> {
    state
        .profiles_mut()
        .start_bill_reads(which)
        .into_iter()
        .map(|(profile, admin_key)| AgentsOutcome::LoadBill {
            profile,
            admin_key,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use otty_agents::SourceId::{
        AnthropicUsageApi, ChatGptUsageApi, CodexAppServer, LatestCodexSession,
    };
    use otty_agents::{SourceError, SourceId, Unavailable};

    use super::*;
    use crate::event::AgentsIntent;
    use crate::reducer::reduce;
    use crate::testing::{
        agents, api_key, at, bill_loaded, bill_of, billed_claude_loaded,
        chatgpt_login, claude, claude_work, codex, codex_work, connected, env,
        fetched_prices, five_hour_limits, fresh_profiles_loaded, limits_answer,
        limits_loaded, load_bill, organization, prices_answer, prices_loaded,
        profiles_loaded, profiles_with_accounts_loaded, saved_with_admin_key,
        sessions_loaded, spend_loaded, spend_of,
    };

    /// The limits of `profile` answered at `read_at`.
    fn limits_answered(
        profile: &otty_agents::Profile,
        read_at: SystemTime,
    ) -> AgentsIntent {
        let source = if profile.agent() == codex().agent() {
            ChatGptUsageApi
        } else {
            AnthropicUsageApi
        };
        let answer = limits_answer(vec![(source, Ok(five_hour_limits(10.0)))]);

        limits_loaded(profile, answer, read_at)
    }

    /// The limits of `profile` that no source gave, at `read_at`.
    fn limits_failed(
        profile: &otty_agents::Profile,
        read_at: SystemTime,
    ) -> AgentsIntent {
        let failed = limits_answer(vec![(
            AnthropicUsageApi,
            Err(SourceError::Network(String::from("down"))),
        )]);

        limits_loaded(profile, failed, read_at)
    }

    fn counted(profile: &otty_agents::Profile, at: SystemTime) -> AgentsIntent {
        spend_loaded(
            profile,
            crate::types::SpendCount::Counted(spend_of(1, 2)),
            at,
        )
    }

    /// Agents mode away, with the start read through to the end for the
    /// connected Claude Code profile: everything read just now when
    /// `is_fresh`, else long ago; the limits as `limits` says.
    fn started(
        is_fresh: bool,
        limits: fn(&otty_agents::Profile, SystemTime) -> AgentsIntent,
    ) -> AgentsState {
        let mut state =
            AgentsState::new(agents(), crate::testing::registry(), env());
        let shown = [(claude(), connected())];
        let (profiles, read_at) = if is_fresh {
            (fresh_profiles_loaded(&shown), SystemTime::now())
        } else {
            (profiles_loaded(&shown), at(10))
        };
        for intent in [
            AgentsIntent::LoadInBackground,
            profiles,
            sessions_loaded(&claude(), Vec::new()),
            limits(&claude(), read_at),
            counted(&claude(), read_at),
        ] {
            let _ = reduce(&mut state, intent);
        }

        state
    }

    #[test]
    fn at_start_the_profiles_are_read_with_the_mode_away() {
        let mut state =
            AgentsState::new(agents(), crate::testing::registry(), env());

        let started = reduce(&mut state, AgentsIntent::LoadInBackground);
        let again = reduce(&mut state, AgentsIntent::LoadInBackground);

        assert_eq!(started, vec![AgentsOutcome::LoadProfiles]);
        assert!(again.is_empty(), "a read is on its way");
        assert!(!state.is_active());
    }

    /// The limits read at the start never starts the agent's program.
    #[test]
    fn at_start_each_kind_is_read_after_the_one_before_ended() {
        let mut state =
            AgentsState::new(agents(), crate::testing::registry(), env());
        let _ = reduce(&mut state, AgentsIntent::LoadInBackground);
        let now = SystemTime::now();

        let profiles = reduce(
            &mut state,
            fresh_profiles_loaded(&[
                (claude(), connected()),
                (codex(), connected()),
            ]),
        );
        let one_sessions =
            reduce(&mut state, sessions_loaded(&claude(), Vec::new()));
        let all_sessions =
            reduce(&mut state, sessions_loaded(&codex(), Vec::new()));
        let one_limits = reduce(&mut state, limits_answered(&claude(), now));
        let all_limits = reduce(&mut state, limits_answered(&codex(), now));
        let one_count = reduce(&mut state, counted(&claude(), now));
        let all_counts = reduce(&mut state, counted(&codex(), now));

        assert_eq!(
            profiles,
            vec![
                AgentsOutcome::LoadSessions(claude()),
                AgentsOutcome::LoadSessions(codex()),
            ]
        );
        assert!(one_sessions.is_empty());
        assert_eq!(
            all_sessions,
            vec![
                AgentsOutcome::LoadLimitsInBackground(claude()),
                AgentsOutcome::LoadLimitsInBackground(codex()),
            ]
        );
        assert!(one_limits.is_empty());
        assert_eq!(
            all_limits,
            vec![
                AgentsOutcome::LoadSpend(claude()),
                AgentsOutcome::LoadSpend(codex()),
            ]
        );
        assert!(one_count.is_empty());
        assert!(all_counts.is_empty(), "no profile has a bill to read");
    }

    /// A second read of the profiles answers while the sessions read the
    /// first one asked for runs: no second sessions read starts, and the
    /// start goes on only once the running one answers.
    #[test]
    fn a_second_profiles_answer_starts_no_second_sessions_read() {
        let mut state =
            AgentsState::new(agents(), crate::testing::registry(), env());
        let shown = [(claude(), connected())];
        let _ = reduce(&mut state, AgentsIntent::LoadInBackground);
        let _ = reduce(&mut state, fresh_profiles_loaded(&shown));
        let _ = reduce(&mut state, AgentsIntent::Refresh);

        let second = reduce(&mut state, fresh_profiles_loaded(&shown));
        let sessions =
            reduce(&mut state, sessions_loaded(&claude(), Vec::new()));

        assert!(second.is_empty(), "{second:?}");
        assert_eq!(
            sessions,
            vec![AgentsOutcome::LoadLimitsInBackground(claude())]
        );
    }

    /// An API key profile has no limits: the start goes on to its tokens,
    /// then its bill.
    #[test]
    fn at_start_a_kind_with_nothing_to_read_is_passed() {
        let mut state =
            AgentsState::new(agents(), crate::testing::registry(), env());
        let _ = reduce(&mut state, AgentsIntent::LoadInBackground);
        let profile = claude();
        let mut loaded = billed_claude_loaded();
        if let AgentsIntent::ProfilesLoaded(profiles) = &mut loaded {
            profiles.read_at = SystemTime::now();
        }
        let _ = reduce(&mut state, loaded);

        let after_sessions =
            reduce(&mut state, sessions_loaded(&profile, Vec::new()));
        let after_count = reduce(&mut state, counted(&profile, at(10)));
        let after_prices = reduce(&mut state, good_prices(SystemTime::now()));
        let after_bill =
            reduce(&mut state, bill_loaded(&profile, Ok(bill_of(1, 2))));

        assert_eq!(
            after_sessions,
            vec![AgentsOutcome::LoadSpend(profile.clone())]
        );
        assert_eq!(after_count, vec![load_prices(false)]);
        assert_eq!(after_prices, vec![load_bill(&profile)]);
        assert!(after_bill.is_empty());
    }

    /// The outcomes of `outcomes` that read limits, in their order.
    fn limit_reads(outcomes: Vec<AgentsOutcome>) -> Vec<AgentsOutcome> {
        outcomes
            .into_iter()
            .filter(|outcome| matches!(outcome, AgentsOutcome::LoadLimits(_)))
            .collect()
    }

    /// The limits `profile` shows: the last good answer and its time.
    fn shown_limits(
        state: &AgentsState,
        profile: &otty_agents::Profile,
    ) -> Option<SystemTime> {
        state
            .profiles()
            .get(profile)
            .and_then(|shown| shown.limits().last_good())
            .map(|(_, read_at)| read_at)
    }

    /// Spec Decision 5, owner's machine: Codex and a second Codex folder
    /// logged in to one ChatGPT account cause one limits read, and both
    /// show its answer with one "Updated" time.
    #[test]
    fn two_profiles_of_one_account_have_their_limits_read_once() {
        let mut state =
            AgentsState::new(agents(), crate::testing::registry(), env());
        let _ = reduce(&mut state, AgentsIntent::Enter);

        let loaded = reduce(
            &mut state,
            profiles_with_accounts_loaded(
                &[
                    (codex(), connected(), vec![chatgpt_login()]),
                    (codex_work(), connected(), vec![chatgpt_login()]),
                    (claude(), connected(), Vec::new()),
                ],
                otty_agents::SavedProfiles::default(),
            ),
        );
        let _ = reduce(&mut state, limits_answered(&codex(), at(20)));
        let ticked = reduce(&mut state, AgentsIntent::LimitsTick);

        assert_eq!(
            limit_reads(loaded),
            [
                AgentsOutcome::LoadLimits(codex()),
                AgentsOutcome::LoadLimits(claude()),
            ]
        );
        assert_eq!(shown_limits(&state, &codex()), Some(at(20)));
        assert_eq!(shown_limits(&state, &codex_work()), Some(at(20)));
        assert_eq!(shown_limits(&state, &claude()), None, "its own read");
        assert_eq!(
            limit_reads(ticked),
            [AgentsOutcome::LoadLimits(codex())],
            "the read of Claude Code is still on its way"
        );
    }

    /// A profile that joins an account while its read is on its way waits
    /// for that read instead of asking again.
    #[test]
    fn a_profile_of_an_account_being_read_waits_for_that_read() {
        let one = [(codex(), connected(), vec![chatgpt_login()])];
        let two = [
            (codex(), connected(), vec![chatgpt_login()]),
            (codex_work(), connected(), vec![chatgpt_login()]),
        ];
        let mut state =
            AgentsState::new(agents(), crate::testing::registry(), env());
        let _ = reduce(&mut state, AgentsIntent::Enter);
        let _ = reduce(
            &mut state,
            profiles_with_accounts_loaded(
                &one,
                otty_agents::SavedProfiles::default(),
            ),
        );
        let _ = reduce(&mut state, AgentsIntent::Refresh);

        let reloaded = reduce(
            &mut state,
            profiles_with_accounts_loaded(
                &two,
                otty_agents::SavedProfiles::default(),
            ),
        );
        let _ = reduce(&mut state, limits_answered(&codex(), at(30)));

        assert!(limit_reads(reloaded).is_empty());
        assert_eq!(shown_limits(&state, &codex_work()), Some(at(30)));
    }

    /// Spec Decision 5: the bill of an organization is read once, with its
    /// admin key, and shows in each profile that pays through it.
    #[test]
    fn two_profiles_of_one_organization_have_its_bill_read_once() {
        let mut state =
            AgentsState::new(agents(), crate::testing::registry(), env());
        let _ = reduce(&mut state, AgentsIntent::Enter);

        let loaded = reduce(
            &mut state,
            profiles_with_accounts_loaded(
                &[
                    (claude(), api_key(), vec![organization()]),
                    (claude_work(), api_key(), vec![organization()]),
                ],
                saved_with_admin_key(),
            ),
        );
        let _ = reduce(&mut state, bill_loaded(&claude(), Ok(bill_of(12, 3))));

        let bills: Vec<_> = loaded
            .into_iter()
            .filter(|outcome| matches!(outcome, AgentsOutcome::LoadBill { .. }))
            .collect();
        assert_eq!(bills, [load_bill(&claude())]);
        for profile in [claude(), claude_work()] {
            let bill = state
                .profiles()
                .get(&profile)
                .and_then(|shown| shown.bill().answer().cloned());
            assert_eq!(bill, Some(bill_of(12, 3)), "{}", profile.name());
        }
    }

    fn load_prices(fetch_again: bool) -> AgentsOutcome {
        AgentsOutcome::LoadPrices { fetch_again }
    }

    /// Prices fetched from LiteLLM at `fetched_at`, read then.
    fn good_prices(fetched_at: SystemTime) -> AgentsIntent {
        prices_loaded(
            prices_answer(&[], Some(fetched_prices(fetched_at))),
            fetched_at,
        )
    }

    /// A read of the prices at `read_at` whose fetch failed, and the kept
    /// file gave the prices fetched at `kept_at`, or no kept file.
    fn failed_fetch(
        read_at: SystemTime,
        kept_at: Option<SystemTime>,
    ) -> AgentsIntent {
        let down = (
            SourceId::LiteLlmPrices,
            SourceError::Network(String::from("down")),
        );
        let answer = match kept_at {
            Some(kept_at) => {
                prices_answer(&[down], Some(fetched_prices(kept_at)))
            },
            None => prices_answer(
                &[
                    down,
                    (
                        SourceId::KeptPrices,
                        SourceError::NotAvailable(Unavailable::NoKeptPriceFile),
                    ),
                ],
                None,
            ),
        };

        prices_loaded(answer, read_at)
    }

    /// The prices read when the API key profile was read on screen.
    fn prices_asked(state: &mut AgentsState) -> Vec<AgentsOutcome> {
        let outcomes =
            reduce(state, fresh_profiles_loaded(&[(claude(), api_key())]));

        outcomes
            .into_iter()
            .filter(|outcome| {
                matches!(outcome, AgentsOutcome::LoadPrices { .. })
            })
            .collect()
    }

    #[test]
    fn on_screen_the_prices_are_read_with_an_api_key_profile_only() {
        let mut with_api_key =
            AgentsState::new(agents(), crate::testing::registry(), env());
        let _ = reduce(&mut with_api_key, AgentsIntent::Enter);
        let mut subscriptions =
            AgentsState::new(agents(), crate::testing::registry(), env());
        let _ = reduce(&mut subscriptions, AgentsIntent::Enter);

        let api_key_reads = prices_asked(&mut with_api_key);
        let subscription_reads = reduce(
            &mut subscriptions,
            fresh_profiles_loaded(&[
                (claude(), connected()),
                (codex(), connected()),
            ]),
        );

        assert_eq!(api_key_reads, vec![load_prices(false)]);
        assert!(
            !subscription_reads.iter().any(|outcome| matches!(
                outcome,
                AgentsOutcome::LoadPrices { .. }
            )),
            "{subscription_reads:?}"
        );
    }

    #[test]
    fn refresh_fetches_the_prices_again_after_a_failed_fetch() {
        for kept_at in [Some(at(10)), None] {
            let mut state =
                AgentsState::new(agents(), crate::testing::registry(), env());
            let _ = reduce(&mut state, AgentsIntent::Enter);
            let _ = prices_asked(&mut state);
            let _ =
                reduce(&mut state, failed_fetch(SystemTime::now(), kept_at));

            let _ = reduce(&mut state, AgentsIntent::Refresh);
            let refreshed = prices_asked(&mut state);

            assert_eq!(refreshed, vec![load_prices(true)], "{kept_at:?}");
        }
    }

    #[test]
    fn refresh_reads_no_prices_after_a_good_fetch_within_the_day() {
        let mut state =
            AgentsState::new(agents(), crate::testing::registry(), env());
        let _ = reduce(&mut state, AgentsIntent::Enter);
        let _ = prices_asked(&mut state);
        let _ = reduce(&mut state, good_prices(SystemTime::now()));

        let _ = reduce(&mut state, AgentsIntent::Refresh);
        let refreshed = prices_asked(&mut state);

        assert!(refreshed.is_empty(), "{refreshed:?}");
    }

    #[test]
    fn prices_fetched_more_than_a_day_ago_are_read_again() {
        let day = std::time::Duration::from_secs(24 * 3600);
        let mut state =
            AgentsState::new(agents(), crate::testing::registry(), env());
        let _ = reduce(&mut state, AgentsIntent::Enter);
        let _ = prices_asked(&mut state);
        let _ = reduce(&mut state, good_prices(SystemTime::now() - day));

        let _ = reduce(&mut state, AgentsIntent::Refresh);
        let refreshed = prices_asked(&mut state);

        assert_eq!(refreshed, vec![load_prices(false)]);
    }

    #[test]
    fn opening_the_mode_reads_the_prices_again_after_an_old_failure() {
        let mut state =
            AgentsState::new(agents(), crate::testing::registry(), env());
        for intent in [
            AgentsIntent::LoadInBackground,
            fresh_profiles_loaded(&[(claude(), api_key())]),
            sessions_loaded(&claude(), Vec::new()),
            counted(&claude(), SystemTime::now()),
            failed_fetch(at(10), None),
        ] {
            let _ = reduce(&mut state, intent);
        }

        let opened = reduce(&mut state, AgentsIntent::Enter);

        assert_eq!(opened, vec![load_prices(true)]);
    }

    #[test]
    fn opening_the_mode_after_a_fresh_start_reads_nothing_again() {
        let mut state = started(true, limits_answered);

        let opened = reduce(&mut state, AgentsIntent::Enter);

        assert!(opened.is_empty(), "{opened:?}");
        assert!(state.is_active());
    }

    #[test]
    fn opening_the_mode_reads_the_profiles_again_when_they_are_old() {
        let mut state = started(false, limits_answered);

        let opened = reduce(&mut state, AgentsIntent::Enter);

        assert_eq!(opened, vec![AgentsOutcome::LoadProfiles]);
    }

    /// The profiles are fresh, the limits and tokens were read long ago.
    #[test]
    fn opening_the_mode_reads_again_what_is_older_than_the_period() {
        let mut state =
            AgentsState::new(agents(), crate::testing::registry(), env());
        for intent in [
            AgentsIntent::LoadInBackground,
            fresh_profiles_loaded(&[(claude(), connected())]),
            sessions_loaded(&claude(), Vec::new()),
            limits_answered(&claude(), at(10)),
            counted(&claude(), at(10)),
        ] {
            let _ = reduce(&mut state, intent);
        }

        let opened = reduce(&mut state, AgentsIntent::Enter);

        assert_eq!(
            opened,
            vec![
                AgentsOutcome::LoadLimits(claude()),
                AgentsOutcome::LoadSpend(claude()),
            ]
        );
    }

    /// The start was still reading the sessions: the other kinds are asked
    /// for at once, and the end of the sessions read asks for nothing more.
    #[test]
    fn opening_the_mode_during_the_start_reads_the_other_kinds_at_once() {
        let mut state =
            AgentsState::new(agents(), crate::testing::registry(), env());
        let _ = reduce(&mut state, AgentsIntent::LoadInBackground);
        let _ = reduce(
            &mut state,
            fresh_profiles_loaded(&[(claude(), connected())]),
        );

        let opened = reduce(&mut state, AgentsIntent::Enter);
        let sessions =
            reduce(&mut state, sessions_loaded(&claude(), Vec::new()));

        assert_eq!(
            opened,
            vec![
                AgentsOutcome::LoadLimits(claude()),
                AgentsOutcome::LoadSpend(claude()),
            ]
        );
        assert!(sessions.is_empty(), "{sessions:?}");
    }

    #[test]
    fn opening_the_mode_while_the_first_profiles_read_runs_waits_for_it() {
        let mut state =
            AgentsState::new(agents(), crate::testing::registry(), env());
        let _ = reduce(&mut state, AgentsIntent::LoadInBackground);

        let opened = reduce(&mut state, AgentsIntent::Enter);
        let profiles = reduce(
            &mut state,
            fresh_profiles_loaded(&[(claude(), connected())]),
        );

        assert!(opened.is_empty());
        assert_eq!(
            profiles,
            vec![
                AgentsOutcome::LoadSessions(claude()),
                AgentsOutcome::LoadLimits(claude()),
                AgentsOutcome::LoadSpend(claude()),
            ]
        );
    }

    /// The hidden terminal did not start at the start, and no other source
    /// answered: on screen the limits are read again with it.
    #[test]
    fn limits_that_failed_at_the_start_are_read_again_on_screen() {
        let mut state = started(true, limits_failed);

        let opened = reduce(&mut state, AgentsIntent::Enter);

        assert_eq!(opened, vec![AgentsOutcome::LoadLimits(claude())]);
    }

    /// `codex app-server` did not start at the start, and a later source
    /// answered: on screen the limits are read again with it.
    #[test]
    fn limits_a_later_source_gave_at_the_start_are_read_again_on_screen() {
        let mut state = started(true, |profile, read_at| {
            let answer = limits_answer(vec![
                (
                    ChatGptUsageApi,
                    Err(SourceError::Network(String::from("down"))),
                ),
                (
                    CodexAppServer,
                    Err(SourceError::NotAvailable(Unavailable::ReadStopped)),
                ),
                (LatestCodexSession, Ok(five_hour_limits(10.0))),
            ]);

            limits_loaded(profile, answer, read_at)
        });

        let opened = reduce(&mut state, AgentsIntent::Enter);

        assert_eq!(opened, vec![AgentsOutcome::LoadLimits(claude())]);
    }

    #[test]
    fn a_limits_read_of_the_start_that_fails_on_screen_is_read_again_at_once() {
        let mut state =
            AgentsState::new(agents(), crate::testing::registry(), env());
        for intent in [
            AgentsIntent::LoadInBackground,
            fresh_profiles_loaded(&[(claude(), connected())]),
            sessions_loaded(&claude(), Vec::new()),
        ] {
            let _ = reduce(&mut state, intent);
        }
        let opened = reduce(&mut state, AgentsIntent::Enter);

        let failed =
            reduce(&mut state, limits_failed(&claude(), SystemTime::now()));
        let failed_on_screen =
            reduce(&mut state, limits_failed(&claude(), SystemTime::now()));

        assert_eq!(opened, vec![AgentsOutcome::LoadSpend(claude())]);
        assert_eq!(failed, vec![AgentsOutcome::LoadLimits(claude())]);
        assert!(failed_on_screen.is_empty(), "the terminal had its chance");
    }

    #[test]
    fn no_read_starts_on_the_timer_while_the_mode_is_away() {
        let mut state = started(true, limits_answered);

        let ticked = reduce(&mut state, AgentsIntent::LimitsTick);

        assert!(ticked.is_empty());
    }

    /// A ChatGPT login only Kilo keeps, found in the made-up Kilo folder.
    fn kilo_login(kilo: &otty_agents::Profile) -> otty_agents::Account {
        otty_agents::Account::new(
            otty_agents::ProviderRef::of_id("openai"),
            otty_agents::AccountIdentity::AccountId(String::from(
                "made-up-kilo-login",
            )),
            otty_agents::AccountLogin::Email(String::from("kilo@example.com")),
            otty_agents::AccountKind::Limits,
            otty_agents::FoundIn::ProfileFolder(
                kilo.config_dir().to_path_buf(),
            ),
        )
    }

    fn kilo() -> otty_agents::Profile {
        otty_agents::Profile::default_for(
            otty_agents::AgentId::new("kilo"),
            "Kilo",
            std::path::PathBuf::from("/home/user/.local/share/kilo"),
        )
    }

    fn own_reads(
        outcomes: Vec<AgentsOutcome>,
    ) -> Vec<(otty_agents::Account, otty_agents::KeyPlace)> {
        outcomes
            .into_iter()
            .filter_map(|outcome| match outcome {
                AgentsOutcome::LoadAccountNumbers { account, place } => {
                    Some((account, place))
                },
                _ => None,
            })
            .collect()
    }

    /// Spec 2026-11-19, Decision 5: an account the user added and a
    /// ChatGPT login only Kilo keeps are read with their own key; the
    /// login Codex reads the limits of is not read twice.
    #[test]
    fn accounts_that_tell_their_own_numbers_are_read_with_their_own_key() {
        let key =
            otty_agents::AdminKeyRef::EnvVar(String::from("DEEPSEEK_API_KEY"));
        let added = otty_agents::Account::added(
            otty_agents::ProviderRef::of_id("deepseek"),
            key.clone(),
        );
        let saved = otty_agents::SavedProfiles::default().with_account_added(
            otty_agents::ProviderRef::of_id("deepseek"),
            key,
        );
        let mut state =
            AgentsState::new(agents(), crate::testing::registry(), env());
        let _ = reduce(&mut state, AgentsIntent::Enter);

        let loaded = reduce(
            &mut state,
            profiles_with_accounts_loaded(
                &[
                    (codex(), connected(), vec![chatgpt_login()]),
                    (kilo(), connected(), vec![kilo_login(&kilo())]),
                ],
                saved,
            ),
        );
        let again = reduce(&mut state, AgentsIntent::LimitsTick);

        assert_eq!(
            own_reads(loaded),
            [
                (
                    kilo_login(&kilo()),
                    otty_agents::KeyPlace::ProfileFiles(kilo())
                ),
                (added, otty_agents::KeyPlace::Added),
            ]
        );
        assert!(own_reads(again).is_empty(), "the reads are on their way");
    }

    /// Spec 2026-11-19, Decision 8: the system may ask to unlock the
    /// keyring, so a key found there is read only while the mode is on
    /// screen.
    #[test]
    fn a_key_of_the_keyring_is_read_only_on_screen() {
        let keyring_key = otty_agents::Account::new(
            otty_agents::ProviderRef::of_id("moonshot"),
            otty_agents::AccountIdentity::KeyFingerprint(
                otty_agents::KeyFingerprint::of("sk-made-up-moonshot"),
            ),
            otty_agents::AccountLogin::Key(None),
            otty_agents::AccountKind::Money,
            otty_agents::FoundIn::SystemKeyring,
        );
        let mut state =
            AgentsState::new(agents(), crate::testing::registry(), env());
        let _ = reduce(&mut state, AgentsIntent::Enter);
        let _ = reduce(
            &mut state,
            profiles_with_accounts_loaded(
                &[(codex(), connected(), vec![chatgpt_login()])],
                otty_agents::SavedProfiles::default().with_keyring_allowed(),
            ),
        );
        let _ = reduce(
            &mut state,
            AgentsIntent::KeyringLoaded(Ok(vec![keyring_key.clone()])),
        );
        let _ = reduce(&mut state, AgentsIntent::Leave);

        let away = load_account_numbers(&mut state, Which::Every);
        state.set_active(true);
        let on_screen = load_account_numbers(&mut state, Which::Every);

        assert!(away.is_empty(), "{away:?}");
        assert_eq!(
            own_reads(on_screen),
            [(
                keyring_key,
                otty_agents::KeyPlace::Keyring(
                    state.profiles().keyring().entries().to_vec()
                )
            )]
        );
    }
}
