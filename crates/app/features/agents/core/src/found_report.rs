//! The Found by OTTY report (spec Decision 2): what OTTY found by itself,
//! shown by itself once, after the first read of the profiles that found
//! any with an agent part on. OTTY ships no plugin, so a start with none
//! leaves it for a later start. Closing it remembers in `agents.json`
//! that it was shown, so it never opens by itself again. The button of a need closes it and does
//! what the need asks.

use super::event::AgentsOutcome;
use super::need::{self, NeedAction};
use super::state::AgentsState;

/// Open the report after a read of the profiles when it was never shown:
/// an agent part of a plugin is on, `agents.json` was read and does not
/// say it was shown, the read found a profile, and the report did not open
/// before in this run.
pub(crate) fn open_on_first_start(state: &mut AgentsState) {
    let has_agent_part = !state.agents().is_empty();
    let profiles = state.profiles();
    let is_never_shown = profiles
        .saved()
        .is_some_and(|saved| !saved.is_found_report_shown());
    let has_profiles = !profiles.shown().is_empty();

    if has_agent_part && is_never_shown && has_profiles {
        state.open_found_report();
    }
}

/// Close the report and save in `agents.json` that it was shown, unless
/// the file says so already: the user opened it again. Nothing happens
/// while it is closed.
pub(crate) fn close(state: &mut AgentsState) -> Vec<AgentsOutcome> {
    if !state.close_found_report() {
        return Vec::new();
    }
    let was_shown = state
        .profiles()
        .saved()
        .is_some_and(|saved| saved.is_found_report_shown());
    if was_shown {
        return Vec::new();
    }

    let Some(saved) = state.profiles_mut().mark_found_report_shown() else {
        return Vec::new();
    };

    vec![AgentsOutcome::SaveProfiles {
        saved,
        create_dir: None,
    }]
}

/// Close the report, then do what the need's button asks.
pub(crate) fn act(
    state: &mut AgentsState,
    action: NeedAction,
) -> Vec<AgentsOutcome> {
    let mut outcomes = close(state);

    outcomes.extend(need::act(state, action));
    outcomes
}

#[cfg(test)]
mod tests {
    use otty_agents::{AgentCommand, SavedProfiles};

    use super::*;
    use crate::event::AgentsIntent;
    use crate::reducer::reduce;
    use crate::testing::{
        api_key, claude, codex, connected, open_fix_tab, organization,
        profiles_loaded_from, profiles_loaded_with_problem, with_accounts,
        with_saved_profiles,
    };

    fn shown_saved() -> SavedProfiles {
        SavedProfiles::default().with_found_report_shown()
    }

    fn save_shown() -> AgentsOutcome {
        AgentsOutcome::SaveProfiles {
            saved: shown_saved(),
            create_dir: None,
        }
    }

    #[test]
    fn the_report_opens_after_the_first_read_that_found_profiles() {
        let state = with_saved_profiles(
            &[(claude(), connected())],
            SavedProfiles::default(),
        );

        assert!(state.is_found_report_open());
    }

    /// Issue 28: on a start with no agent part there is nothing to read,
    /// so the report waits for a start that has one, and `agents.json`
    /// does not say it was shown.
    #[test]
    fn the_report_waits_for_an_agent_part() {
        let mut state = AgentsState::new(
            crate::testing::no_agents(),
            crate::testing::registry(),
            crate::testing::env(),
        );
        let _ = reduce(&mut state, AgentsIntent::Enter);

        let _ = reduce(
            &mut state,
            profiles_loaded_from(
                &[(claude(), connected())],
                SavedProfiles::default(),
            ),
        );
        let closed = reduce(&mut state, AgentsIntent::CloseFoundReport);

        assert!(!state.is_found_report_open());
        assert!(closed.is_empty());
        assert_eq!(state.profiles().saved(), Some(&SavedProfiles::default()));
    }

    #[test]
    fn the_report_does_not_open_when_agents_json_says_it_was_shown() {
        let state =
            with_saved_profiles(&[(claude(), connected())], shown_saved());

        assert!(!state.is_found_report_open());
    }

    #[test]
    fn the_report_waits_for_a_profile_and_a_readable_agents_json() {
        let mut state = with_saved_profiles(&[], SavedProfiles::default());
        let none_found = state.is_found_report_open();

        // The read of a broken `agents.json` hands on no content.
        let AgentsIntent::ProfilesLoaded(mut broken) =
            profiles_loaded_with_problem(
                &[(claude(), connected())],
                Some("not json"),
            )
        else {
            panic!("a profiles read is given");
        };
        broken.saved = None;
        let _ = reduce(&mut state, AgentsIntent::ProfilesLoaded(broken));
        let unreadable = state.is_found_report_open();

        assert!(!none_found);
        assert!(!unreadable);
    }

    #[test]
    fn closing_the_report_saves_that_it_was_shown() {
        let mut state = with_saved_profiles(
            &[(claude(), connected())],
            SavedProfiles::default(),
        );

        let closed = reduce(&mut state, AgentsIntent::CloseFoundReport);
        let again = reduce(&mut state, AgentsIntent::CloseFoundReport);

        assert_eq!(closed, vec![save_shown()]);
        assert!(again.is_empty());
        assert!(!state.is_found_report_open());
        assert_eq!(state.profiles().saved(), Some(&shown_saved()));
    }

    /// A read that started before the save may still find the old file:
    /// the report does not open twice in one run.
    #[test]
    fn a_closed_report_does_not_open_again_in_the_same_run() {
        let mut state = with_saved_profiles(
            &[(claude(), connected())],
            SavedProfiles::default(),
        );
        let _ = reduce(&mut state, AgentsIntent::CloseFoundReport);

        let _ = reduce(
            &mut state,
            profiles_loaded_from(
                &[(claude(), connected())],
                SavedProfiles::default(),
            ),
        );

        assert!(!state.is_found_report_open());
    }

    #[test]
    fn run_of_a_need_closes_the_report_and_opens_the_login_tab() {
        let mut state = with_saved_profiles(
            &[(codex(), crate::testing::not_logged_in(&codex()))],
            SavedProfiles::default(),
        );

        let outcomes = reduce(
            &mut state,
            AgentsIntent::FoundReportNeed(NeedAction::Run {
                title: String::from("Codex"),
                command: AgentCommand::new("codex").with_arg("login"),
            }),
        );

        assert_eq!(
            outcomes,
            vec![
                save_shown(),
                open_fix_tab(
                    "Codex",
                    "/home/user",
                    AgentCommand::new("codex").with_arg("login")
                ),
            ]
        );
        assert!(!state.is_found_report_open());
    }

    #[test]
    fn ask_key_of_a_need_closes_the_report_and_opens_the_key_dialog() {
        let mut state = with_accounts(
            &[(claude(), api_key(), vec![organization()])],
            SavedProfiles::default(),
        );

        let outcomes = reduce(
            &mut state,
            AgentsIntent::FoundReportNeed(NeedAction::AskKey {
                account: organization(),
                plugin: String::from("Anthropic"),
                ask: crate::testing::key_ask(),
            }),
        );

        assert_eq!(outcomes, vec![save_shown()]);
        assert!(!state.is_found_report_open());
        assert!(state.key_dialog().is_some());
    }
}
