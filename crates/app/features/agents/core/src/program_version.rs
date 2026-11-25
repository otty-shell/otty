//! The version of an agent's program for the profiles whose session files
//! name none (spec Decision 9). It starts the program (`--version`), so it
//! is asked only while Agents mode is on screen, like the limits that
//! start a program, and at most once per agent in a run: a refresh does
//! not ask again. A version from the session files always comes first.

use otty_agents::{AgentId, ProfileStatus};

use super::event::AgentsOutcome;
use super::state::AgentsState;

/// Ask for the version of the program of each agent that has a shown
/// profile with no version from its sessions and a program that was found,
/// while the mode is on screen and the agent was not asked before in this
/// run.
pub(crate) fn ask(state: &mut AgentsState) -> Vec<AgentsOutcome> {
    if !state.is_active() {
        return Vec::new();
    }

    let mut wanted: Vec<AgentId> = Vec::new();
    for shown in state.profiles().shown() {
        let agent = shown.profile().agent();
        let needs_program = shown.version().is_none()
            && *shown.status() != ProfileStatus::ProgramNotFound;
        if needs_program && !wanted.contains(agent) {
            wanted.push(agent.clone());
        }
    }

    wanted
        .into_iter()
        .filter(|agent| state.profiles_mut().start_version_read(agent))
        .map(AgentsOutcome::LoadVersion)
        .collect()
}

#[cfg(test)]
mod tests {
    use otty_agents::SavedProfiles;

    use super::*;
    use crate::event::AgentsIntent;
    use crate::reducer::reduce;
    use crate::testing::{
        agents, claude, claude_work, codex, connected, env,
        profiles_loaded_from, with_versions,
    };
    use crate::types::LoadedVersion;

    fn loaded(versions: &[Option<&str>]) -> AgentsIntent {
        with_versions(
            profiles_loaded_from(
                &[(claude(), connected()), (claude_work(), connected())],
                SavedProfiles::default().with_found_report_shown(),
            ),
            versions,
        )
    }

    fn version_reads(outcomes: &[AgentsOutcome]) -> Vec<&AgentsOutcome> {
        outcomes
            .iter()
            .filter(|outcome| matches!(outcome, AgentsOutcome::LoadVersion(_)))
            .collect()
    }

    #[test]
    fn while_the_mode_is_away_no_program_is_asked_for_its_version() {
        let mut state =
            AgentsState::new(agents(), crate::testing::registry(), env());

        let away = reduce(&mut state, loaded(&[None, None]));
        let on_screen = reduce(&mut state, AgentsIntent::Enter);

        assert!(version_reads(&away).is_empty(), "{away:?}");
        assert_eq!(
            version_reads(&on_screen),
            [&AgentsOutcome::LoadVersion(claude().agent().clone())]
        );
    }

    #[test]
    fn two_reads_of_the_profiles_ask_the_program_once() {
        let mut state =
            AgentsState::new(agents(), crate::testing::registry(), env());
        let _ = reduce(&mut state, AgentsIntent::Enter);

        let first = reduce(&mut state, loaded(&[None, None]));
        let _ = reduce(&mut state, AgentsIntent::Refresh);
        let second = reduce(&mut state, loaded(&[None, None]));

        assert_eq!(
            version_reads(&first),
            [&AgentsOutcome::LoadVersion(claude().agent().clone())]
        );
        assert!(version_reads(&second).is_empty(), "{second:?}");
    }

    #[test]
    fn profiles_with_a_version_from_their_sessions_ask_nothing() {
        let mut state =
            AgentsState::new(agents(), crate::testing::registry(), env());
        let _ = reduce(&mut state, AgentsIntent::Enter);

        let outcomes =
            reduce(&mut state, loaded(&[Some("2.1.328"), Some("2.1.300")]));

        assert!(version_reads(&outcomes).is_empty(), "{outcomes:?}");
    }

    #[test]
    fn a_program_not_found_is_not_asked() {
        let mut state =
            AgentsState::new(agents(), crate::testing::registry(), env());
        let _ = reduce(&mut state, AgentsIntent::Enter);

        let outcomes = reduce(
            &mut state,
            profiles_loaded_from(
                &[(codex(), ProfileStatus::ProgramNotFound)],
                SavedProfiles::default().with_found_report_shown(),
            ),
        );

        assert!(version_reads(&outcomes).is_empty(), "{outcomes:?}");
    }

    /// The answer stands for every profile of the agent with no version of
    /// its own, and stays through the next read; a session version wins.
    #[test]
    fn the_programs_version_fills_the_profiles_with_none_of_their_own() {
        let mut state =
            AgentsState::new(agents(), crate::testing::registry(), env());
        let _ = reduce(&mut state, AgentsIntent::Enter);
        let _ = reduce(&mut state, loaded(&[Some("2.1.300"), None]));

        let _ = reduce(
            &mut state,
            AgentsIntent::VersionLoaded(LoadedVersion {
                agent: claude().agent().clone(),
                version: Some(String::from("2.1.328")),
            }),
        );
        let _ = reduce(&mut state, AgentsIntent::Refresh);
        let _ = reduce(&mut state, loaded(&[Some("2.1.300"), None]));

        let versions: Vec<_> = state
            .profiles()
            .shown()
            .iter()
            .map(|shown| state.profiles().version_of(shown))
            .collect();
        assert_eq!(versions, [Some("2.1.300"), Some("2.1.328")]);
    }
}
