//! What the button of a need does (spec "Words"): a need is a task only
//! the user can do, with one button. The Found by OTTY report and the All
//! accounts and All profiles pages list the needs and send the same
//! action.

use otty_agents::Profile;

use super::event::AgentsOutcome;
use super::state::AgentsState;
use super::{profile_actions, profile_dialog};

/// What the button of a need does: the one thing the user can do about it
/// (spec "Words").
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NeedAction {
    /// Log the profile in: its agent logs in in a terminal tab.
    LogIn(Profile),
    /// Give the admin key of the organization found in the profile, in the
    /// profile dialog.
    AddAdminKey(Profile),
}

/// Do what the button of a need asks.
pub(crate) fn act(
    state: &mut AgentsState,
    action: NeedAction,
) -> Vec<AgentsOutcome> {
    match action {
        NeedAction::LogIn(profile) => profile_actions::log_in(state, &profile),
        NeedAction::AddAdminKey(profile) => {
            profile_dialog::open_edit(state, &profile)
        },
    }
}

#[cfg(test)]
mod tests {
    use otty_agents::{AgentCommand, ProfileStatus, SavedProfiles};

    use super::*;
    use crate::event::AgentsIntent;
    use crate::reducer::reduce;
    use crate::testing::{
        api_key, claude, codex, open_tab, organization, with_accounts,
        with_saved_profiles,
    };

    fn report_shown() -> SavedProfiles {
        SavedProfiles::default().with_found_report_shown()
    }

    #[test]
    fn log_in_of_a_need_on_a_page_opens_the_login_tab() {
        let mut state = with_saved_profiles(
            &[(codex(), ProfileStatus::NotLoggedIn)],
            report_shown(),
        );

        let outcomes = reduce(
            &mut state,
            AgentsIntent::FixNeed(NeedAction::LogIn(codex())),
        );

        assert_eq!(
            outcomes,
            vec![open_tab(
                "Codex",
                "/home/user",
                AgentCommand::new("codex").with_arg("login")
            )]
        );
    }

    #[test]
    fn add_admin_key_of_a_need_on_a_page_opens_the_profile_dialog() {
        let mut state = with_accounts(
            &[(claude(), api_key(), vec![organization()])],
            report_shown(),
        );

        let outcomes = reduce(
            &mut state,
            AgentsIntent::FixNeed(NeedAction::AddAdminKey(claude())),
        );

        assert!(outcomes.is_empty());
        assert!(state.dialog().is_some());
    }

    /// The report never opens or closes from a page: closing it saves
    /// `agents.json`, and a page has nothing to save.
    #[test]
    fn a_need_on_a_page_leaves_the_found_report_as_it_is() {
        let mut state = with_saved_profiles(
            &[(codex(), ProfileStatus::NotLoggedIn)],
            SavedProfiles::default(),
        );

        let _ = reduce(
            &mut state,
            AgentsIntent::FixNeed(NeedAction::LogIn(codex())),
        );

        assert!(state.is_found_report_open());
    }
}
