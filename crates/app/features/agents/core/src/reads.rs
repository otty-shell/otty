//! Which reads Agents mode asks for, and when (spec Decision 10; issue 24
//! of spec 2026-11-19-agent-plugins).
//!
//! At the start of OTTY the mode is not on screen, yet it reads what it
//! shows. The profiles come first, as every read needs them and the
//! accounts found in them; then the sessions, the list prices, the numbers
//! of the accounts and the bills of the organizations are asked for
//! together, so a slow read, such as the sessions of a large Codex folder,
//! holds up no other. The first count of the tokens waits for the first
//! read of the list prices, kept or fetched, so a start reads the session
//! files once; with no prices at all it still counts, and counts again
//! once prices arrive (issue 11 of spec
//! 2026-11-23-accounts-apart-from-profiles). The numbers and the bills are
//! read once per account. The list prices are read at most once a day,
//! and again after a failed fetch (spec Decision 16). The reads of the
//! start never start the agent's program, neither as is nor in a hidden
//! terminal. On screen, a read of the profiles asks for every kind at
//! once, as the Refresh button does. Opening the mode reads again only
//! what is older than the period of its timer, the numbers a read without
//! the program could not give, and the sessions of a profile the New
//! session dialog started a session of. What a read gave stays on screen
//! until the next read of its kind answers.

use std::time::SystemTime;

use super::event::AgentsOutcome;
use super::freshness::Which;
use super::state::AgentsState;

/// OTTY started: read the profiles with the mode away, unless they were
/// read or a read is on its way; the other kinds follow their answer.
pub(crate) fn start_in_background(
    state: &mut AgentsState,
) -> Vec<AgentsOutcome> {
    let profiles = state.profiles();
    if state.is_active() || profiles.is_loaded() || profiles.is_loading() {
        return Vec::new();
    }

    load_profiles(state)
}

/// Agents mode opened at `now`: what is older than the period is read
/// again, as are the numbers that wait for the agent's program and the
/// sessions of a profile a new session was started with.
pub(crate) fn on_open(
    state: &mut AgentsState,
    now: SystemTime,
) -> Vec<AgentsOutcome> {
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
/// other kind is asked for now.
pub(crate) fn after_profiles(
    state: &mut AgentsState,
    now: SystemTime,
) -> Vec<AgentsOutcome> {
    load_every_kind(state, Which::Every, now)
}

/// The numbers of an account were read: on screen, numbers that wait for
/// the agent's program, read while the mode was away, are read again with
/// it.
pub(crate) fn after_account_numbers(
    state: &mut AgentsState,
) -> Vec<AgentsOutcome> {
    let mut outcomes = read_accounts_at_new_prices(state);
    if !state.is_active() {
        return outcomes;
    }

    outcomes.extend(load_account_numbers(state, Which::WaitingForPrograms));
    outcomes
}

/// The timer ticked: while the mode is on screen, read the numbers of the
/// accounts and the bills of the organizations again.
pub(crate) fn on_tick(state: &mut AgentsState) -> Vec<AgentsOutcome> {
    if !state.is_active() {
        return Vec::new();
    }

    let mut outcomes = load_account_numbers(state, Which::Every);
    outcomes.extend(load_bills(state, Which::Every));
    outcomes
}

/// Ask for the numbers of each account `which` picks, unless a read of it
/// is on its way: with the agent's program while the mode is on screen,
/// without it while the mode is away.
pub(crate) fn load_account_numbers(
    state: &mut AgentsState,
    which: Which,
) -> Vec<AgentsOutcome> {
    let may_start_programs = state.is_active();

    state
        .profiles_mut()
        .start_account_reads(which, may_start_programs)
        .into_iter()
        .map(|(account, given)| AgentsOutcome::LoadAccountNumbers {
            account,
            given,
            may_start_programs,
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

/// The list prices were read or changed, or a count ended after they
/// did: ask for the tokens of each shown profile that waits for them,
/// unless a count is on its way. The agent parts price the tokens (spec
/// 2026-11-23-accounts-apart-from-profiles, Decision 3), so only a new
/// count follows new prices.
pub(crate) fn count_at_new_prices(
    state: &mut AgentsState,
) -> Vec<AgentsOutcome> {
    let mut outcomes: Vec<AgentsOutcome> = state
        .profiles_mut()
        .start_counts_at_new_prices()
        .into_iter()
        .map(AgentsOutcome::LoadSpend)
        .collect();

    outcomes.extend(read_accounts_at_new_prices(state));
    outcomes
}

/// Ask for the numbers of each account paid per token that waits for new
/// list prices, unless a read of it is on its way, which asks again once
/// it ends: its provider part prices what the agent's sessions spent
/// through it at them (issue 22 of spec
/// 2026-11-23-accounts-apart-from-profiles).
fn read_accounts_at_new_prices(state: &mut AgentsState) -> Vec<AgentsOutcome> {
    let may_start_programs = state.is_active();

    state
        .profiles_mut()
        .start_account_reads_at_new_prices(may_start_programs)
        .into_iter()
        .map(|(account, given)| AgentsOutcome::LoadAccountNumbers {
            account,
            given,
            may_start_programs,
        })
        .collect()
}

/// Ask at once for every kind besides the profiles and the sessions that
/// `which` wants at `now`: the list prices, then the tokens, which wait
/// for the first read of the prices, the numbers of the accounts and the
/// bills.
fn load_every_kind(
    state: &mut AgentsState,
    which: Which,
    now: SystemTime,
) -> Vec<AgentsOutcome> {
    let mut outcomes = load_prices(state, which, now);
    outcomes.extend(load_spend(state, which));
    outcomes.extend(load_account_numbers(state, which));
    outcomes.extend(load_bills(state, which));
    outcomes
}

/// Ask for the tokens of each shown profile `which` picks that has no
/// count on its way; none while the first read of the list prices is on
/// its way, which starts them once it answers.
fn load_spend(state: &mut AgentsState, which: Which) -> Vec<AgentsOutcome> {
    state
        .profiles_mut()
        .start_spend_reads(which)
        .into_iter()
        .map(AgentsOutcome::LoadSpend)
        .collect()
}

/// Ask for the list prices when a profile is shown and `which` wants them
/// at `now`, unless a read is on its way: once a day, and again after a
/// failed fetch (spec Decision 16).
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

/// Ask for the bill of each organization a provider part reads that
/// `which` picks, once per organization.
fn load_bills(state: &mut AgentsState, which: Which) -> Vec<AgentsOutcome> {
    state
        .profiles_mut()
        .start_bill_reads(which)
        .into_iter()
        .map(|(organization, given)| AgentsOutcome::LoadBill {
            organization,
            given,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use otty_agents::{ApiPrices, SourceError, SourceId, Unavailable};

    use super::*;
    use crate::event::AgentsIntent;
    use crate::reducer::reduce;
    use crate::testing::{
        agents, at, billed_claude_loaded, chatgpt_login, claude, claude_login,
        claude_work, codex, codex_work, connected, env, fetched_prices,
        five_hour_limits, fresh_profiles_loaded, limits_numbers, load_bill,
        numbers_loaded, organization, price_table, prices_answer,
        prices_loaded, profiles_with_accounts_loaded, registry,
        saved_with_given_key, sessions_loaded, spend_loaded, spend_of,
        with_profiles,
    };

    /// The limits of the [`claude_login`] answered at `read_at`.
    fn login_answered(read_at: SystemTime) -> AgentsIntent {
        numbers_loaded(
            &claude_login(),
            Ok(limits_numbers(five_hour_limits(10.0))),
            read_at,
        )
    }

    /// The read of the [`claude_login`] its part failed, at `read_at`.
    fn login_failed(read_at: SystemTime) -> AgentsIntent {
        let down = SourceError::Network(String::from("down"));

        numbers_loaded(&claude_login(), Err(down), read_at)
    }

    fn counted(profile: &otty_agents::Profile, at: SystemTime) -> AgentsIntent {
        spend_loaded(
            profile,
            crate::types::SpendCount::Counted(spend_of(1, 2)),
            at,
        )
    }

    /// The answer of a profiles read just now that found `profiles`, each
    /// with the accounts found in it, and `saved` in `agents.json`.
    fn fresh_accounts_loaded(
        profiles: &[(
            otty_agents::Profile,
            otty_agents::ProfileStatus,
            Vec<otty_agents::Account>,
        )],
        saved: otty_agents::SavedProfiles,
    ) -> AgentsIntent {
        let mut loaded = profiles_with_accounts_loaded(profiles, saved);
        if let AgentsIntent::ProfilesLoaded(profiles) = &mut loaded {
            profiles.read_at = SystemTime::now();
        }

        loaded
    }

    /// The read of the numbers of `account`, which may start the agent's
    /// program when `may_start_programs`.
    fn numbers_read(
        account: otty_agents::Account,
        may_start_programs: bool,
    ) -> AgentsOutcome {
        AgentsOutcome::LoadAccountNumbers {
            account,
            given: Vec::new(),
            may_start_programs,
        }
    }

    /// Agents mode away, with the start read through to the end for the
    /// connected Claude Code profile with the [`claude_login`]: everything
    /// read just now when `is_fresh`, else long ago; the numbers of the
    /// login as `numbers` says.
    fn started(
        is_fresh: bool,
        numbers: fn(SystemTime) -> AgentsIntent,
    ) -> AgentsState {
        let mut state =
            AgentsState::new(agents(), crate::testing::registry(), env());
        let shown = [(claude(), connected(), vec![claude_login()])];
        let saved = otty_agents::SavedProfiles::default();
        let (profiles, read_at) = if is_fresh {
            (fresh_accounts_loaded(&shown, saved), SystemTime::now())
        } else {
            (profiles_with_accounts_loaded(&shown, saved), at(10))
        };
        for intent in [
            AgentsIntent::LoadInBackground,
            profiles,
            sessions_loaded(&claude(), Vec::new()),
            good_prices(read_at),
            numbers(read_at),
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
    /// prices and the numbers of the accounts do not need them, so they
    /// are asked for with the sessions. The reads of the start never start
    /// the agent's program.
    #[test]
    fn at_start_nothing_waits_for_the_sessions_read() {
        let mut state =
            AgentsState::new(agents(), crate::testing::registry(), env());
        let _ = reduce(&mut state, AgentsIntent::LoadInBackground);

        let profiles = reduce(
            &mut state,
            fresh_accounts_loaded(
                &[
                    (claude(), connected(), vec![claude_login()]),
                    (codex(), connected(), vec![chatgpt_login()]),
                ],
                otty_agents::SavedProfiles::default(),
            ),
        );
        let sessions =
            reduce(&mut state, sessions_loaded(&codex(), Vec::new()));

        assert_eq!(
            profiles,
            vec![
                AgentsOutcome::LoadSessions(claude()),
                AgentsOutcome::LoadSessions(codex()),
                load_prices(false),
                numbers_read(claude_login(), false),
                numbers_read(chatgpt_login(), false),
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

    /// An organization is read at the start with everything else: the
    /// list prices, its bill and its own numbers are asked for with the
    /// sessions.
    #[test]
    fn at_start_an_organization_is_read_with_everything_else() {
        let mut state =
            AgentsState::new(agents(), crate::testing::registry(), env());
        let _ = reduce(&mut state, AgentsIntent::LoadInBackground);
        let mut loaded = billed_claude_loaded();
        if let AgentsIntent::ProfilesLoaded(profiles) = &mut loaded {
            profiles.read_at = SystemTime::now();
        }

        let profiles = reduce(&mut state, loaded);

        let (own, others): (Vec<_>, Vec<_>) =
            profiles.into_iter().partition(|outcome| {
                matches!(outcome, AgentsOutcome::LoadAccountNumbers { .. })
            });
        assert_eq!(
            others,
            [
                AgentsOutcome::LoadSessions(claude()),
                load_prices(false),
                load_bill(),
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
    }

    /// How many reads of the numbers of `account` `outcomes` ask for.
    fn reads_of(
        outcomes: &[AgentsOutcome],
        account: &otty_agents::Account,
    ) -> usize {
        outcomes
            .iter()
            .filter(|outcome| {
                matches!(outcome,
                AgentsOutcome::LoadAccountNumbers { account: read, .. }
                    if read.is_same_account(account))
            })
            .count()
    }

    /// When the limits of `account` were read, as they show.
    fn shown_limits(
        state: &AgentsState,
        account: &otty_agents::Account,
    ) -> Option<SystemTime> {
        state
            .profiles()
            .account_limits(account)
            .and_then(|limits| limits.last_good().map(|(_, read_at)| read_at))
    }

    /// Spec Decision 5, owner's machine: Codex and a second Codex folder
    /// logged in to one ChatGPT account cause one read of it, and the
    /// account shows its answer with one "Updated" time.
    #[test]
    fn an_account_found_in_two_profiles_is_read_once() {
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
        let _ = reduce(
            &mut state,
            numbers_loaded(
                &chatgpt_login(),
                Ok(limits_numbers(five_hour_limits(10.0))),
                at(20),
            ),
        );
        let ticked = reduce(&mut state, AgentsIntent::LimitsTick);

        assert_eq!(reads_of(&loaded, &chatgpt_login()), 1, "{loaded:?}");
        assert_eq!(shown_limits(&state, &chatgpt_login()), Some(at(20)));
        assert_eq!(reads_of(&ticked, &chatgpt_login()), 1, "{ticked:?}");
    }

    /// An account found in one more profile while its read is on its way
    /// waits for that read instead of asking again.
    #[test]
    fn an_account_found_again_while_it_is_read_waits_for_that_read() {
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

        assert_eq!(reads_of(&reloaded, &chatgpt_login()), 0, "{reloaded:?}");
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

    fn counts(outcomes: &[AgentsOutcome]) -> Vec<&AgentsOutcome> {
        outcomes
            .iter()
            .filter(|outcome| matches!(outcome, AgentsOutcome::LoadSpend(_)))
            .collect()
    }

    /// Spec 2026-11-23-accounts-apart-from-profiles, Decision 3: the agent
    /// parts price the tokens, so new list prices count the tokens of each
    /// profile again, and the same prices read again count nothing.
    #[test]
    fn new_prices_count_the_tokens_again() {
        let mut state = with_profiles(&[(claude(), connected())]);
        let _ = reduce(&mut state, counted(&claude(), at(10)));
        let before = Arc::clone(state.profiles().api_prices());

        let new = reduce(&mut state, good_prices(at(20)));
        let _ = reduce(&mut state, counted(&claude(), at(30)));
        let same = reduce(&mut state, good_prices(at(20)));

        assert_eq!(counts(&new), [&AgentsOutcome::LoadSpend(claude())]);
        assert!(counts(&same).is_empty(), "{same:?}");
        assert_ne!(state.profiles().api_prices(), &before);
        assert_eq!(
            state.profiles().api_prices().as_ref(),
            &ApiPrices::new(price_table(), registry())
        );
    }

    /// Issue 22 of spec 2026-11-23-accounts-apart-from-profiles: a part
    /// prices the sessions of an account paid per token while it reads its
    /// numbers, so new list prices read those numbers again; a
    /// subscription's limits have no price, and the same prices read
    /// nothing.
    #[test]
    fn new_prices_read_the_numbers_of_accounts_paid_per_token_again() {
        let mut state = crate::testing::with_accounts(
            &[(claude(), connected(), vec![organization(), claude_login()])],
            saved_with_given_key(),
        );
        let money = otty_agents::AccountNumbers::Balance(
            otty_agents::Balance::new(None, None),
        );
        let _ = reduce(
            &mut state,
            numbers_loaded(&organization(), Ok(money.clone()), at(10)),
        );
        let _ = reduce(&mut state, login_answered(at(10)));

        let new = reduce(&mut state, good_prices(at(20)));
        let _ = reduce(
            &mut state,
            numbers_loaded(&organization(), Ok(money), at(30)),
        );
        let same = reduce(&mut state, good_prices(at(20)));

        assert_eq!(reads_of(&new, &organization()), 1, "{new:?}");
        assert_eq!(reads_of(&new, &claude_login()), 0, "{new:?}");
        assert_eq!(reads_of(&same, &organization()), 0, "{same:?}");
    }

    /// Issue 11 of spec 2026-11-23-accounts-apart-from-profiles: at the
    /// start the list prices are read before the first count, so the
    /// session files are read once. Each shown profile is counted once,
    /// when the kept prices answer.
    #[test]
    fn a_start_with_kept_prices_counts_each_shown_profile_once() {
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
        let prices = reduce(
            &mut state,
            failed_fetch(SystemTime::now(), Some(SystemTime::now())),
        );
        let first = reduce(&mut state, counted(&claude(), SystemTime::now()));
        let second = reduce(&mut state, counted(&codex(), SystemTime::now()));

        assert!(counts(&profiles).is_empty(), "{profiles:?}");
        assert!(profiles.contains(&load_prices(false)), "{profiles:?}");
        assert_eq!(
            counts(&prices),
            [
                &AgentsOutcome::LoadSpend(claude()),
                &AgentsOutcome::LoadSpend(codex()),
            ]
        );
        assert!(counts(&first).is_empty(), "{first:?}");
        assert!(counts(&second).is_empty(), "{second:?}");
    }

    /// Issue 11: a start with no prices at all, a first run offline,
    /// counts once when the read of the prices gives none, and once more
    /// when prices arrive.
    #[test]
    fn a_start_with_no_prices_counts_once_and_again_when_prices_arrive() {
        let mut state =
            AgentsState::new(agents(), crate::testing::registry(), env());
        let _ = reduce(&mut state, AgentsIntent::LoadInBackground);
        let profiles = reduce(
            &mut state,
            fresh_profiles_loaded(&[(claude(), connected())]),
        );

        let no_prices =
            reduce(&mut state, failed_fetch(SystemTime::now(), None));
        let counted_without =
            reduce(&mut state, counted(&claude(), SystemTime::now()));
        let arrived = reduce(&mut state, good_prices(SystemTime::now()));
        let counted_with =
            reduce(&mut state, counted(&claude(), SystemTime::now()));

        assert!(counts(&profiles).is_empty(), "{profiles:?}");
        assert_eq!(counts(&no_prices), [&AgentsOutcome::LoadSpend(claude())]);
        assert!(counts(&counted_without).is_empty(), "{counted_without:?}");
        assert_eq!(counts(&arrived), [&AgentsOutcome::LoadSpend(claude())]);
        assert!(counts(&counted_with).is_empty(), "{counted_with:?}");
    }

    /// A count on its way when the prices change was asked at the old
    /// prices, here none: the tokens are counted again once it ends.
    #[test]
    fn a_count_on_its_way_when_the_prices_change_is_made_again() {
        let mut state = with_profiles(&[(claude(), connected())]);
        let _ = reduce(&mut state, failed_fetch(at(10), None));
        let was_counting =
            state.profiles().shown()[0].spend().reads().is_reading();

        let during = reduce(&mut state, good_prices(at(20)));
        let after = reduce(&mut state, counted(&claude(), at(30)));
        let settled = reduce(&mut state, counted(&claude(), at(40)));

        assert!(was_counting, "the answer of the prices asked for a count");
        assert!(counts(&during).is_empty(), "{during:?}");
        assert_eq!(after, [AgentsOutcome::LoadSpend(claude())]);
        assert!(settled.is_empty());
    }

    /// The prices read when the API key profile was read on screen.
    fn prices_asked(state: &mut AgentsState) -> Vec<AgentsOutcome> {
        let outcomes =
            reduce(state, fresh_profiles_loaded(&[(claude(), connected())]));

        outcomes
            .into_iter()
            .filter(|outcome| {
                matches!(outcome, AgentsOutcome::LoadPrices { .. })
            })
            .collect()
    }

    /// Spec 2026-11-23, Decision 3: the money of every profile is at API
    /// prices, so the prices are read with a profile paid by a
    /// subscription too, and with no profile they are not read.
    #[test]
    fn on_screen_the_prices_are_read_with_any_profile() {
        let entered = || {
            let mut state =
                AgentsState::new(agents(), crate::testing::registry(), env());
            let _ = reduce(&mut state, AgentsIntent::Enter);
            state
        };
        let prices_of = |outcomes: Vec<AgentsOutcome>| -> Vec<AgentsOutcome> {
            outcomes
                .into_iter()
                .filter(|outcome| {
                    matches!(outcome, AgentsOutcome::LoadPrices { .. })
                })
                .collect()
        };

        let api_key_reads = prices_asked(&mut entered());
        let subscription_reads = prices_of(reduce(
            &mut entered(),
            fresh_profiles_loaded(&[
                (claude(), connected()),
                (codex(), connected()),
            ]),
        ));
        let no_profile_reads =
            prices_of(reduce(&mut entered(), fresh_profiles_loaded(&[])));

        assert_eq!(api_key_reads, vec![load_prices(false)]);
        assert_eq!(subscription_reads, vec![load_prices(false)]);
        assert!(no_profile_reads.is_empty(), "{no_profile_reads:?}");
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
            fresh_profiles_loaded(&[(claude(), connected())]),
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
        let mut state = started(true, login_answered);

        let opened = reduce(&mut state, AgentsIntent::Enter);

        assert!(opened.is_empty(), "{opened:?}");
        assert!(state.is_active());
    }

    #[test]
    fn opening_the_mode_reads_the_profiles_again_when_they_are_old() {
        let mut state = started(false, login_answered);

        let opened = reduce(&mut state, AgentsIntent::Enter);

        assert_eq!(opened, vec![AgentsOutcome::LoadProfiles]);
    }

    /// The profiles and the prices are fresh, the numbers and the tokens
    /// were read long ago.
    #[test]
    fn opening_the_mode_reads_again_what_is_older_than_the_period() {
        let mut state =
            AgentsState::new(agents(), crate::testing::registry(), env());
        for intent in [
            AgentsIntent::LoadInBackground,
            fresh_accounts_loaded(
                &[(claude(), connected(), vec![claude_login()])],
                otty_agents::SavedProfiles::default(),
            ),
            sessions_loaded(&claude(), Vec::new()),
            good_prices(SystemTime::now()),
            login_answered(at(10)),
            counted(&claude(), at(10)),
        ] {
            let _ = reduce(&mut state, intent);
        }

        let opened = reduce(&mut state, AgentsIntent::Enter);

        assert_eq!(
            opened,
            vec![
                AgentsOutcome::LoadSpend(claude()),
                numbers_read(claude_login(), true),
            ]
        );
    }

    /// The start was still reading the sessions and the prices: opening
    /// the mode asks for none of them twice, and the tokens are counted
    /// once the prices answer.
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
        let prices = reduce(&mut state, good_prices(SystemTime::now()));
        let count = reduce(&mut state, counted(&claude(), SystemTime::now()));

        assert!(opened.is_empty(), "{opened:?}");
        assert!(sessions.is_empty(), "{sessions:?}");
        assert_eq!(counts(&prices), [&AgentsOutcome::LoadSpend(claude())]);
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
            fresh_accounts_loaded(
                &[(claude(), connected(), vec![claude_login()])],
                otty_agents::SavedProfiles::default(),
            ),
        );

        assert!(opened.is_empty());
        assert_eq!(
            profiles,
            vec![
                AgentsOutcome::LoadSessions(claude()),
                load_prices(false),
                numbers_read(claude_login(), true),
            ]
        );
    }

    /// The hidden terminal did not start at the start, and the part gave
    /// nothing: on screen the account is read again with it.
    #[test]
    fn an_account_that_failed_at_the_start_is_read_again_on_screen() {
        let mut state = started(true, login_failed);

        let opened = reduce(&mut state, AgentsIntent::Enter);

        assert_eq!(opened, vec![numbers_read(claude_login(), true)]);
    }

    /// Issue 29: at the start `claude /usage` did not start, so the
    /// provider part of a login with no token read nothing: on screen the
    /// account is read again with the program.
    #[test]
    fn an_account_read_as_nothing_at_the_start_is_read_again_on_screen() {
        let mut state = started(true, |read_at| {
            let nothing = otty_agents::Limits::new(
                otty_agents::Billing::UsageWindows(
                    otty_agents::UsageWindows::default(),
                ),
                None,
            );
            numbers_loaded(
                &claude_login(),
                Ok(limits_numbers(nothing)),
                read_at,
            )
        });

        let opened = reduce(&mut state, AgentsIntent::Enter);

        assert_eq!(opened, vec![numbers_read(claude_login(), true)]);
    }

    #[test]
    fn a_read_of_the_start_that_fails_on_screen_is_read_again_at_once() {
        let mut state =
            AgentsState::new(agents(), crate::testing::registry(), env());
        for intent in [
            AgentsIntent::LoadInBackground,
            fresh_accounts_loaded(
                &[(claude(), connected(), vec![claude_login()])],
                otty_agents::SavedProfiles::default(),
            ),
            sessions_loaded(&claude(), Vec::new()),
        ] {
            let _ = reduce(&mut state, intent);
        }
        let opened = reduce(&mut state, AgentsIntent::Enter);

        let failed = reduce(&mut state, login_failed(SystemTime::now()));
        let failed_on_screen =
            reduce(&mut state, login_failed(SystemTime::now()));

        assert!(opened.is_empty(), "the start's reads are on their way");
        assert_eq!(failed, vec![numbers_read(claude_login(), true)]);
        assert!(failed_on_screen.is_empty(), "the terminal had its chance");
    }

    #[test]
    fn no_read_starts_on_the_timer_while_the_mode_is_away() {
        let mut state = started(true, login_answered);

        let ticked = reduce(&mut state, AgentsIntent::LimitsTick);

        assert!(ticked.is_empty());
    }

    /// A subscription the OpenAI part found in the folder of a made-up
    /// agent, Coder.
    fn coder_login(coder: &otty_agents::Profile) -> otty_agents::Account {
        otty_agents::Account::new(
            "openai",
            otty_agents::ProviderId::new("openai"),
            "chatgpt:made-up-coder-login",
            "coder@example.com",
            otty_agents::AccountKind::Subscription,
            vec![otty_agents::FoundIn::profile(
                coder.agent().clone(),
                coder.config_dir().to_path_buf(),
            )],
        )
    }

    fn coder() -> otty_agents::Profile {
        otty_agents::Profile::default_for(
            otty_agents::AgentId::new("coder"),
            "Coder",
            std::path::PathBuf::from("/home/user/.coder"),
        )
    }

    fn own_reads(outcomes: Vec<AgentsOutcome>) -> Vec<otty_agents::Account> {
        outcomes
            .into_iter()
            .filter_map(|outcome| match outcome {
                AgentsOutcome::LoadAccountNumbers { account, .. } => {
                    Some(account)
                },
                _ => None,
            })
            .collect()
    }

    /// Spec 2026-11-23-accounts-apart-from-profiles, Decision 5: every
    /// account a part reads is read by itself, whatever agent its folder
    /// belongs to.
    #[test]
    fn every_account_a_part_reads_is_read_by_itself() {
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
                otty_agents::SavedProfiles::default(),
            ),
        );
        let again = reduce(&mut state, AgentsIntent::LimitsTick);

        assert_eq!(own_reads(loaded), [chatgpt_login(), coder_login(&coder())]);
        assert!(own_reads(again).is_empty(), "the reads are on their way");
    }

    /// Spec 2026-11-23-accounts-apart-from-profiles, Decision 1: two
    /// plugins that find one login give two accounts, each read by its own
    /// part; OTTY joins none.
    #[test]
    fn one_login_two_plugins_find_is_two_accounts_each_read_by_its_part() {
        let fork = otty_agents::ProviderEntry::new(
            otty_agents::ProviderId::new("openai"),
            "OpenAI Fork",
            otty_plugin_host::PluginPart::new(
                "openai-fork",
                "OpenAI Fork",
                "0.1.0",
                "",
                "openai",
            ),
        );
        let of_fork = otty_agents::Account::new(
            "openai-fork",
            otty_agents::ProviderId::new("openai"),
            "chatgpt:made-up-account",
            "you@example.com",
            otty_agents::AccountKind::Key,
            Vec::new(),
        );
        let mut state = AgentsState::new(
            agents(),
            crate::testing::registry_with(fork),
            env(),
        );
        let _ = reduce(&mut state, AgentsIntent::Enter);

        let loaded = reduce(
            &mut state,
            profiles_with_accounts_loaded(
                &[(
                    codex(),
                    connected(),
                    vec![crate::testing::key_account(), of_fork],
                )],
                otty_agents::SavedProfiles::default(),
            ),
        );

        let read: Vec<String> = own_reads(loaded)
            .iter()
            .map(|account| account.plugin().to_string())
            .collect();
        assert_eq!(state.profiles().accounts().len(), 2);
        assert_eq!(read, ["openai", "openai-fork"]);
    }

    /// Issue 11 of spec 2026-11-23-accounts-apart-from-profiles: the
    /// limits of a subscription found in a profile are the numbers of its
    /// account, read by its provider part in the folders it was found in,
    /// at the start without the agent's program.
    #[test]
    fn a_subscription_is_read_as_the_numbers_of_its_account() {
        let mut state =
            AgentsState::new(agents(), crate::testing::registry(), env());
        let _ = reduce(&mut state, AgentsIntent::LoadInBackground);

        let loaded = reduce(
            &mut state,
            fresh_accounts_loaded(
                &[
                    (codex(), connected(), vec![chatgpt_login()]),
                    (claude(), connected(), Vec::new()),
                ],
                otty_agents::SavedProfiles::default(),
            ),
        );

        let read = AgentsOutcome::LoadAccountNumbers {
            account: chatgpt_login(),
            given: Vec::new(),
            may_start_programs: false,
        };
        assert!(loaded.contains(&read), "{loaded:?}");
        assert_eq!(own_reads(loaded), [chatgpt_login()]);
    }

    /// Issue 29, kept with the reads of the accounts: at the start the
    /// agent's program did not start, so the read of a subscription gave
    /// nothing; on screen it is read again with the program, and an
    /// account read well at the start is not.
    #[test]
    fn an_account_that_waits_for_its_program_is_read_again_on_screen() {
        let mut state =
            AgentsState::new(agents(), crate::testing::registry(), env());
        let down = SourceError::Network(String::from("down"));
        for intent in [
            AgentsIntent::LoadInBackground,
            fresh_accounts_loaded(
                &[
                    (codex(), connected(), vec![chatgpt_login()]),
                    (claude(), connected(), vec![claude_login()]),
                ],
                otty_agents::SavedProfiles::default(),
            ),
            numbers_loaded(&chatgpt_login(), Err(down), SystemTime::now()),
            numbers_loaded(
                &claude_login(),
                Ok(limits_numbers(five_hour_limits(10.0))),
                SystemTime::now(),
            ),
        ] {
            let _ = reduce(&mut state, intent);
        }

        let opened = reduce(&mut state, AgentsIntent::Enter);

        let read = AgentsOutcome::LoadAccountNumbers {
            account: chatgpt_login(),
            given: Vec::new(),
            may_start_programs: true,
        };
        assert!(opened.contains(&read), "{opened:?}");
        assert_eq!(own_reads(opened), [chatgpt_login()]);
    }

    /// Issue 11: an organization has a bill, which its provider part reads
    /// once, whatever else was found in its folders: a Claude
    /// subscription next to it changes nothing.
    #[test]
    fn the_bill_of_an_organization_is_read_once_whatever_its_folders_hold() {
        let mut state =
            AgentsState::new(agents(), crate::testing::registry(), env());
        let _ = reduce(&mut state, AgentsIntent::Enter);

        let loaded = reduce(
            &mut state,
            profiles_with_accounts_loaded(
                &[
                    (
                        claude(),
                        connected(),
                        vec![claude_login(), organization()],
                    ),
                    (
                        claude_work(),
                        connected(),
                        vec![claude_login(), organization()],
                    ),
                ],
                saved_with_given_key(),
            ),
        );
        let ticked = reduce(&mut state, AgentsIntent::LimitsTick);

        let bills: Vec<_> = loaded
            .iter()
            .filter_map(|outcome| match outcome {
                AgentsOutcome::LoadBill {
                    organization,
                    given,
                } => Some((organization.key().clone(), given.clone())),
                _ => None,
            })
            .collect();
        assert_eq!(
            bills,
            [(
                organization().key().clone(),
                vec![crate::testing::given_key()]
            )]
        );
        assert!(
            !ticked.iter().any(|outcome| matches!(
                outcome,
                AgentsOutcome::LoadBill { .. }
            )),
            "the read is on its way"
        );
    }

    /// Issue 17: an account of a provider no plugin part serves is listed
    /// with the others, and its numbers are not read.
    #[test]
    fn an_account_no_plugin_serves_is_listed_and_not_read() {
        let unserved = otty_agents::Account::new(
            "example",
            otty_agents::ProviderId::new("example-cloud"),
            "key:made-up-example",
            "key ••••mple",
            otty_agents::AccountKind::Key,
            vec![crate::testing::found_in(
                crate::testing::codex_id(),
                "/home/user/.codex",
            )],
        );
        let mut state =
            AgentsState::new(agents(), crate::testing::registry(), env());
        let _ = reduce(&mut state, AgentsIntent::Enter);

        let loaded = reduce(
            &mut state,
            profiles_with_accounts_loaded(
                &[(codex(), connected(), vec![unserved.clone()])],
                otty_agents::SavedProfiles::default(),
            ),
        );

        assert!(state.profiles().accounts().contains(&unserved));
        assert!(
            own_reads(loaded).iter().all(|account| *account != unserved),
            "an account no part serves is not read"
        );
    }
}
