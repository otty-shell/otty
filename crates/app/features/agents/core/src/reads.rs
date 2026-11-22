//! Which reads Agents mode asks for, and when (spec Decision 10; issue 24
//! of spec 2026-11-19-agent-plugins).
//!
//! At the start of OTTY the mode is not on screen, yet it reads what it
//! shows. The profiles come first, as every read needs them; then the
//! sessions, the limits, the tokens and the numbers of the accounts that
//! tell their own are asked for together, so a slow read, such as the
//! sessions of a large Codex folder, holds up no other. Only the list
//! prices and the bills wait, for the limits (see [`StartWait`]). The
//! limits and the bills are read once per account. The list prices are
//! read only while a profile pays with an API key, at most once a day, and
//! again after a failed fetch (spec Decision 16). The limits read then
//! never starts the agent's program, neither as is nor in a hidden
//! terminal. On screen, a read of the profiles asks for every kind at once,
//! as the Refresh button does. Opening the mode reads again only what is
//! older than the period of its timer, the limits a source failed while
//! the program could not start, and the sessions of a profile the New
//! session dialog started a session of. What a read gave stays on screen
//! until the next read of its kind answers.

use std::time::SystemTime;

use super::event::AgentsOutcome;
use super::freshness::Which;
use super::state::AgentsState;

/// What the start of OTTY waits for, with the mode away, before it asks
/// for the reads that need its answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StartWait {
    /// The profiles: every other read needs the profiles and the accounts
    /// found in them.
    Profiles,
    /// The limits: the list prices and the bills are read for the profiles
    /// that pay with an API key, and a profile whose files do not tell how
    /// it pays learns it from its limits answer.
    Limits,
}

/// OTTY started: read the profiles with the mode away, unless they were
/// read or a read is on its way; the other kinds follow their answer.
pub(crate) fn start_in_background(
    state: &mut AgentsState,
) -> Vec<AgentsOutcome> {
    let profiles = state.profiles();
    if state.is_active() || profiles.is_loaded() || profiles.is_loading() {
        return Vec::new();
    }

    state.set_start_wait(Some(StartWait::Profiles));
    load_profiles(state)
}

/// Agents mode opened at `now`: the reads of the start that did not begin
/// yet begin at once, and what is older than the period is read again, as
/// are the sessions of a profile a new session was started with.
pub(crate) fn on_open(
    state: &mut AgentsState,
    now: SystemTime,
) -> Vec<AgentsOutcome> {
    state.set_start_wait(None);

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
    outcomes.extend(load_every_kind(state, Which::Stale(now), now));
    outcomes
}

/// The profiles were read at `now` and their sessions asked for: every
/// other kind is asked for now, except at the start, where the list prices
/// and the bills wait for the limits ([`StartWait::Limits`]).
pub(crate) fn after_profiles(
    state: &mut AgentsState,
    now: SystemTime,
) -> Vec<AgentsOutcome> {
    if state.start_wait().is_none() {
        return load_every_kind(state, Which::Every, now);
    }

    let mut outcomes = load_limits(state, Which::Every);
    outcomes.extend(load_spend(state, Which::Every));
    outcomes.extend(load_account_numbers(state, Which::Every));

    state.set_start_wait(Some(StartWait::Limits));
    outcomes.extend(after_start_limits(state, now));
    outcomes
}

/// The limits of a profile were read at `now`: on screen, limits that
/// wait for the agent's program are read again with it; at the start the
/// list prices and the bills follow once every limit is read.
pub(crate) fn after_limits(
    state: &mut AgentsState,
    now: SystemTime,
) -> Vec<AgentsOutcome> {
    if state.is_active() {
        return load_limits(state, Which::WaitingForPrograms);
    }

    after_start_limits(state, now)
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
/// key and that `which` picks, unless a read of it is on its way.
pub(crate) fn load_account_numbers(
    state: &mut AgentsState,
    which: Which,
) -> Vec<AgentsOutcome> {
    let may_start_programs = state.is_active();

    state
        .profiles_mut()
        .start_own_reads(which)
        .into_iter()
        .map(
            |(account, place, given)| AgentsOutcome::LoadAccountNumbers {
                account,
                place,
                given,
                may_start_programs,
            },
        )
        .collect()
}

/// Ask for the profiles, unless a read is on its way.
pub(crate) fn load_profiles(state: &mut AgentsState) -> Vec<AgentsOutcome> {
    if !state.profiles_mut().start_loading() {
        return Vec::new();
    }

    vec![AgentsOutcome::LoadProfiles]
}

/// Ask at once for every kind besides the profiles and the sessions that
/// `which` wants at `now`: the limits, the tokens, the list prices, the
/// bills and the numbers of the accounts that tell their own.
fn load_every_kind(
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

/// At the start, once no limits read is on its way, ask for the list
/// prices at `now` and the bills, which wait for the limits
/// ([`StartWait::Limits`]); the start then ends.
fn after_start_limits(
    state: &mut AgentsState,
    now: SystemTime,
) -> Vec<AgentsOutcome> {
    let is_waiting = state.start_wait() == Some(StartWait::Limits);
    if !is_waiting || state.profiles().is_reading_limits() {
        return Vec::new();
    }

    state.set_start_wait(None);

    let mut outcomes = load_prices(state, Which::Every, now);
    outcomes.extend(load_bills(state, Which::Every));
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

/// Ask for the bill of each organization a provider part serves that an
/// API key profile `which` picks pays through, once per organization.
fn load_bills(state: &mut AgentsState, which: Which) -> Vec<AgentsOutcome> {
    state
        .profiles_mut()
        .start_bill_reads(which)
        .into_iter()
        .map(|(profile, organization, given)| AgentsOutcome::LoadBill {
            profile,
            organization,
            given,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use otty_agents::{SourceError, SourceId, Unavailable};

    use super::*;
    use crate::event::AgentsIntent;
    use crate::reducer::reduce;
    use crate::testing::{
        agents, api_key, at, bill_loaded, bill_of, billed_claude_loaded,
        chatgpt_login, claude, claude_work, codex, codex_work, connected, env,
        fetched_prices, five_hour_limits, fresh_profiles_loaded, limits_answer,
        limits_loaded, load_bill, organization, prices_answer, prices_loaded,
        profiles_loaded, profiles_with_accounts_loaded, saved_with_given_key,
        sessions_loaded, spend_loaded, spend_of,
    };

    /// The limits of `profile` answered at `read_at`.
    fn limits_answered(
        profile: &otty_agents::Profile,
        read_at: SystemTime,
    ) -> AgentsIntent {
        let answer = limits_answer(vec![(
            SourceId::ProviderPart,
            Ok(five_hour_limits(10.0)),
        )]);

        limits_loaded(profile, answer, read_at)
    }

    /// The limits of `profile` that no source gave, at `read_at`.
    fn limits_failed(
        profile: &otty_agents::Profile,
        read_at: SystemTime,
    ) -> AgentsIntent {
        let failed = limits_answer(vec![(
            SourceId::ProviderPart,
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

    /// Issue 24: the sessions of Codex take seconds to read, and the
    /// limits and the tokens do not need them, so they are asked for with
    /// the sessions. The limits read at the start never starts the agent's
    /// program.
    #[test]
    fn at_start_the_limits_read_does_not_wait_for_the_sessions_read() {
        let mut state =
            AgentsState::new(agents(), crate::testing::registry(), env());
        let _ = reduce(&mut state, AgentsIntent::LoadInBackground);

        let profiles = reduce(
            &mut state,
            fresh_profiles_loaded(&[
                (claude(), connected()),
                (codex(), connected()),
            ]),
        );
        let sessions =
            reduce(&mut state, sessions_loaded(&codex(), Vec::new()));

        assert_eq!(
            profiles,
            vec![
                AgentsOutcome::LoadSessions(claude()),
                AgentsOutcome::LoadSessions(codex()),
                AgentsOutcome::LoadLimitsInBackground(claude()),
                AgentsOutcome::LoadLimitsInBackground(codex()),
                AgentsOutcome::LoadSpend(claude()),
                AgentsOutcome::LoadSpend(codex()),
            ]
        );
        assert!(sessions.is_empty(), "{sessions:?}");
    }

    /// A second read of the profiles answers while the reads the first
    /// one asked for run: none of them is asked for twice.
    #[test]
    fn a_second_profiles_answer_asks_for_no_read_twice() {
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
        assert!(sessions.is_empty(), "{sessions:?}");
    }

    /// An API key profile has no limits to wait for: its tokens, the list
    /// prices, its bill and the own numbers of its organization are asked
    /// for with its sessions.
    #[test]
    fn at_start_with_no_limits_to_read_everything_is_asked_for_at_once() {
        let mut state =
            AgentsState::new(agents(), crate::testing::registry(), env());
        let _ = reduce(&mut state, AgentsIntent::LoadInBackground);
        let profile = claude();
        let mut loaded = billed_claude_loaded();
        if let AgentsIntent::ProfilesLoaded(profiles) = &mut loaded {
            profiles.read_at = SystemTime::now();
        }

        let profiles = reduce(&mut state, loaded);
        let after_count = reduce(&mut state, counted(&profile, at(10)));

        let (own, others): (Vec<_>, Vec<_>) =
            profiles.into_iter().partition(|outcome| {
                matches!(outcome, AgentsOutcome::LoadAccountNumbers { .. })
            });
        assert_eq!(
            others,
            [
                AgentsOutcome::LoadSessions(profile.clone()),
                AgentsOutcome::LoadSpend(profile.clone()),
                load_prices(false),
                load_bill(&profile),
            ]
        );
        assert!(!own.is_empty(), "the organization's own numbers are read");
        assert!(
            own.iter().all(|outcome| matches!(
                outcome,
                AgentsOutcome::LoadAccountNumbers { account, .. }
                    if account.is_same_account(&organization())
            )),
            "{own:?}"
        );
        assert!(after_count.is_empty(), "{after_count:?}");
    }

    /// Codex learns from its limits answer that it pays with an API key:
    /// the list prices of its money and the bills are asked for once the
    /// limits are read.
    #[test]
    fn at_start_the_prices_and_the_bills_wait_for_the_limits() {
        let mut state =
            AgentsState::new(agents(), crate::testing::registry(), env());
        let _ = reduce(&mut state, AgentsIntent::LoadInBackground);
        let mut loaded = profiles_with_accounts_loaded(
            &[
                (codex(), connected(), Vec::new()),
                (claude(), api_key(), vec![organization()]),
            ],
            saved_with_given_key(),
        );
        if let AgentsIntent::ProfilesLoaded(profiles) = &mut loaded {
            profiles.read_at = SystemTime::now();
        }
        let api_key_answer = limits_answer(vec![(
            SourceId::ProviderPart,
            Ok(otty_agents::Limits::new(otty_agents::Billing::ApiKey, None)),
        )]);

        let profiles = reduce(&mut state, loaded);
        let limits = reduce(
            &mut state,
            limits_loaded(&codex(), api_key_answer, SystemTime::now()),
        );

        assert!(
            profiles.contains(&AgentsOutcome::LoadLimitsInBackground(codex()))
        );
        assert!(
            !profiles.iter().any(|outcome| matches!(
                outcome,
                AgentsOutcome::LoadPrices { .. }
                    | AgentsOutcome::LoadBill { .. }
            )),
            "{profiles:?}"
        );
        assert_eq!(limits, vec![load_prices(false), load_bill(&claude())]);
        assert!(state.profiles().is_api_key(&codex()));
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
                saved_with_given_key(),
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

    /// The start was still reading the sessions, the limits and the
    /// tokens: opening the mode asks for none of them twice, and their
    /// answers ask for nothing more.
    #[test]
    fn opening_the_mode_during_the_start_asks_for_no_read_twice() {
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
        let count = reduce(&mut state, counted(&claude(), SystemTime::now()));

        assert!(opened.is_empty(), "{opened:?}");
        assert!(sessions.is_empty(), "{sessions:?}");
        assert!(count.is_empty(), "{count:?}");
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

    /// Issue 29: at the start `claude /usage` did not start, so the
    /// provider part of a login with no token read nothing: on screen the
    /// limits are read again with the program.
    #[test]
    fn limits_read_as_nothing_at_the_start_are_read_again_on_screen() {
        let mut state = started(true, |profile, read_at| {
            limits_loaded(profile, crate::types::LimitsRead::NoSources, read_at)
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

        assert!(opened.is_empty(), "the start asked for the tokens");
        assert_eq!(failed, vec![AgentsOutcome::LoadLimits(claude())]);
        assert!(failed_on_screen.is_empty(), "the terminal had its chance");
    }

    #[test]
    fn no_read_starts_on_the_timer_while_the_mode_is_away() {
        let mut state = started(true, limits_answered);

        let ticked = reduce(&mut state, AgentsIntent::LimitsTick);

        assert!(ticked.is_empty());
    }

    /// A subscription login only a made-up agent, Coder, keeps, found in
    /// its folder.
    fn coder_login(coder: &otty_agents::Profile) -> otty_agents::Account {
        otty_agents::Account::new(
            otty_agents::ProviderRef::of_id("openai"),
            otty_agents::AccountIdentity::AccountId(String::from(
                "made-up-coder-login",
            )),
            otty_agents::AccountLogin::Email(String::from("coder@example.com")),
            otty_agents::AccountKind::Limits,
            otty_agents::FoundIn::ProfileFolder(
                coder.config_dir().to_path_buf(),
            ),
        )
        .with_read_by(Some(String::from("openai")))
    }

    fn coder() -> otty_agents::Profile {
        otty_agents::Profile::default_for(
            otty_agents::AgentId::new("coder"),
            "Coder",
            std::path::PathBuf::from("/home/user/.coder"),
        )
    }

    fn own_reads(
        outcomes: Vec<AgentsOutcome>,
    ) -> Vec<(otty_agents::Account, otty_agents::KeyPlace)> {
        outcomes
            .into_iter()
            .filter_map(|outcome| match outcome {
                AgentsOutcome::LoadAccountNumbers {
                    account, place, ..
                } => Some((account, place)),
                _ => None,
            })
            .collect()
    }

    /// Spec 2026-11-19, Decision 5: an account the user added and a
    /// subscription login only Coder keeps are read with their own key; the
    /// login Codex reads the limits of is not read twice.
    #[test]
    fn accounts_that_tell_their_own_numbers_are_read_with_their_own_key() {
        let key = otty_agents::KeyRef::EnvVar(String::from("DEEPSEEK_API_KEY"));
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
                    (coder(), connected(), vec![coder_login(&coder())]),
                ],
                saved,
            ),
        );
        let again = reduce(&mut state, AgentsIntent::LimitsTick);

        assert_eq!(
            own_reads(loaded),
            [
                (
                    coder_login(&coder()),
                    otty_agents::KeyPlace::ProfileFiles(vec![coder()])
                ),
                (
                    crate::testing::read_by_its_plugin(added),
                    otty_agents::KeyPlace::Added
                ),
            ]
        );
        assert!(own_reads(again).is_empty(), "the reads are on their way");
    }

    /// Issue 17: an account added by hand for a provider no plugin part
    /// serves is listed with the others, and its numbers are not read.
    #[test]
    fn an_added_account_no_plugin_serves_is_listed_and_not_read() {
        let key = otty_agents::KeyRef::EnvVar(String::from("EXAMPLE_API_KEY"));
        let added = otty_agents::Account::added(
            otty_agents::ProviderRef::of_id("example-cloud"),
            key.clone(),
        );
        let saved = otty_agents::SavedProfiles::default().with_account_added(
            otty_agents::ProviderRef::of_id("example-cloud"),
            key,
        );
        let mut state =
            AgentsState::new(agents(), crate::testing::registry(), env());
        let _ = reduce(&mut state, AgentsIntent::Enter);

        let loaded = reduce(
            &mut state,
            profiles_with_accounts_loaded(
                &[(codex(), connected(), Vec::new())],
                saved,
            ),
        );

        assert!(state.profiles().accounts().contains(&added));
        assert!(
            own_reads(loaded)
                .iter()
                .all(|(account, _)| *account != added),
            "an account no part serves is not read"
        );
    }
}
