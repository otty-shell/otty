//! Which reads Agents mode asks for, and when (spec Decision 10).
//!
//! At the start of OTTY the mode is not on screen, yet it reads what it
//! shows: the profiles, then the sessions of each profile, then the limits,
//! then the tokens, then the money, one kind after another so the machine
//! is not loaded with all of them at once. The limits read then never
//! starts the agent's program: neither `codex app-server` nor `claude` in a
//! hidden terminal. On screen, a read of the profiles asks for every kind
//! at once, as the Refresh button does. Opening the mode reads again only
//! what is older than the period of its timer, and the limits a source
//! failed while the program could not start.

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
    /// The limits of each connected profile.
    Limits,
    /// The tokens each profile spent.
    Spend,
    /// The bill of each API key profile with an admin key.
    Bills,
}

impl ReadKind {
    /// The kind read after this one at the start; `None` after the last.
    fn next(self) -> Option<Self> {
        match self {
            Self::Profiles => Some(Self::Sessions),
            Self::Sessions => Some(Self::Limits),
            Self::Limits => Some(Self::Spend),
            Self::Spend => Some(Self::Bills),
            Self::Bills => None,
        }
    }
}

/// OTTY started: read the profiles with the mode away, unless they were
/// read or a read is on its way; the other kinds follow one after another.
pub(crate) fn start_in_background(
    state: &mut AgentsState,
) -> Vec<AgentsOutcome> {
    if state.is_active() || state.is_loaded() || state.is_loading() {
        return Vec::new();
    }

    state.set_startup_reads(Some(ReadKind::Profiles));
    load_profiles(state)
}

/// Agents mode opened at `now`: the reads of the start that did not begin
/// yet begin at once, and what is older than the period is read again.
pub(crate) fn on_open(
    state: &mut AgentsState,
    now: SystemTime,
) -> Vec<AgentsOutcome> {
    state.set_startup_reads(None);

    if state.is_loading() {
        // The profiles read on its way asks for every kind when it ends.
        return Vec::new();
    }
    if !state.are_profiles_fresh(now) {
        return load_profiles(state);
    }

    let stale = Which::Stale(now);
    let mut outcomes = load_limits(state, stale);
    outcomes.extend(load_spend(state, stale));
    outcomes.extend(load_bills(state, stale));
    outcomes
}

/// The profiles were read and their sessions asked for: at the start the
/// next kind follows once the sessions are read, else every kind is read
/// now.
pub(crate) fn after_profiles(state: &mut AgentsState) -> Vec<AgentsOutcome> {
    if state.startup_reads().is_some() {
        state.set_startup_reads(Some(ReadKind::Sessions));
        return go_on_at_start(state);
    }

    let mut outcomes = load_limits(state, Which::Every);
    outcomes.extend(load_spend(state, Which::Every));
    outcomes.extend(load_bills(state, Which::Every));
    outcomes
}

/// The limits of a profile were read: on screen, limits that wait for the
/// agent's program are read again with it; at the start the next kind
/// follows once every limit is read.
pub(crate) fn after_limits(state: &mut AgentsState) -> Vec<AgentsOutcome> {
    if state.is_active() {
        return load_limits(state, Which::WaitingForPrograms);
    }

    go_on_at_start(state)
}

/// A read of the start ended: once no read of the kind of the start is on
/// its way, ask for the next kind that has something to read.
pub(crate) fn go_on_at_start(state: &mut AgentsState) -> Vec<AgentsOutcome> {
    while let Some(kind) = state.startup_reads() {
        if state.is_reading_any(kind) {
            return Vec::new();
        }

        let next = kind.next();
        state.set_startup_reads(next);

        let outcomes = match next {
            Some(ReadKind::Limits) => load_limits(state, Which::Every),
            Some(ReadKind::Spend) => load_spend(state, Which::Every),
            Some(ReadKind::Bills) => load_bills(state, Which::Every),
            _ => Vec::new(),
        };
        if !outcomes.is_empty() {
            return outcomes;
        }
    }

    Vec::new()
}

/// The timer ticked: while the mode is on screen, read the limits and the
/// bills of every profile again.
pub(crate) fn on_tick(state: &mut AgentsState) -> Vec<AgentsOutcome> {
    if !state.is_active() {
        return Vec::new();
    }

    let mut outcomes = load_limits(state, Which::Every);
    outcomes.extend(load_bills(state, Which::Every));
    outcomes
}

/// Ask for the profiles, unless a read is on its way.
pub(crate) fn load_profiles(state: &mut AgentsState) -> Vec<AgentsOutcome> {
    if !state.start_loading() {
        return Vec::new();
    }

    vec![AgentsOutcome::LoadProfiles]
}

/// Ask for the limits of each connected profile `which` picks that has no
/// read on its way: with the agent's program while the mode is on screen,
/// without it while the mode is away.
fn load_limits(state: &mut AgentsState, which: Which) -> Vec<AgentsOutcome> {
    let is_on_screen = state.is_active();

    state
        .start_limit_reads(which)
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
        .start_spend_reads(which)
        .into_iter()
        .map(AgentsOutcome::LoadSpend)
        .collect()
}

/// Ask for the bill of each API key profile with an admin key `which`
/// picks that has no read on its way.
fn load_bills(state: &mut AgentsState, which: Which) -> Vec<AgentsOutcome> {
    state
        .start_bill_reads(which)
        .into_iter()
        .map(AgentsOutcome::LoadBill)
        .collect()
}

#[cfg(test)]
mod tests {
    use otty_agents::SourceError;
    use otty_agents::SourceId::{
        AnthropicUsageApi, ChatGptUsageApi, CodexAppServer, LatestCodexSession,
    };

    use super::*;
    use crate::event::AgentsIntent;
    use crate::reducer::reduce;
    use crate::testing::{
        agents, api_key, at, bill_loaded, bill_of, claude,
        claude_with_admin_key, codex, connected, env, five_hour_limits,
        fresh_profiles_loaded, limits_answer, limits_loaded, profiles_loaded,
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
        spend_loaded(profile, Some(Ok(spend_of(1, 2))), at)
    }

    /// Agents mode away, with the start read through to the end for the
    /// connected Claude Code profile: everything read just now when
    /// `is_fresh`, else long ago; the limits as `limits` says.
    fn started(
        is_fresh: bool,
        limits: fn(&otty_agents::Profile, SystemTime) -> AgentsIntent,
    ) -> AgentsState {
        let mut state = AgentsState::new(agents(), env());
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
        let mut state = AgentsState::new(agents(), env());

        let started = reduce(&mut state, AgentsIntent::LoadInBackground);
        let again = reduce(&mut state, AgentsIntent::LoadInBackground);

        assert_eq!(started, vec![AgentsOutcome::LoadProfiles]);
        assert!(again.is_empty(), "a read is on its way");
        assert!(!state.is_active());
    }

    /// The limits read at the start never starts the agent's program.
    #[test]
    fn at_start_each_kind_is_read_after_the_one_before_ended() {
        let mut state = AgentsState::new(agents(), env());
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

    /// An API key profile has no limits: the start goes on to its tokens,
    /// then its bill.
    #[test]
    fn at_start_a_kind_with_nothing_to_read_is_passed() {
        let mut state = AgentsState::new(agents(), env());
        let _ = reduce(&mut state, AgentsIntent::LoadInBackground);
        let profile = claude_with_admin_key();
        let _ = reduce(
            &mut state,
            fresh_profiles_loaded(&[(profile.clone(), api_key())]),
        );

        let after_sessions =
            reduce(&mut state, sessions_loaded(&profile, Vec::new()));
        let after_count = reduce(&mut state, counted(&profile, at(10)));
        let after_bill =
            reduce(&mut state, bill_loaded(&profile, Ok(bill_of(1, 2))));

        assert_eq!(
            after_sessions,
            vec![AgentsOutcome::LoadSpend(profile.clone())]
        );
        assert_eq!(after_count, vec![AgentsOutcome::LoadBill(profile)]);
        assert!(after_bill.is_empty());
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
        let mut state = AgentsState::new(agents(), env());
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
        let mut state = AgentsState::new(agents(), env());
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
        let mut state = AgentsState::new(agents(), env());
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
                    Err(SourceError::NotAvailable(String::from("not now"))),
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
        let mut state = AgentsState::new(agents(), env());
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
}
