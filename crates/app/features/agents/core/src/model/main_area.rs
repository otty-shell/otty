use std::time::SystemTime;

use super::super::state::AgentsState;
use super::account_page::AccountPageViewModel;
use super::dashboard::DashboardViewModel;
use super::overview::{AllAccountsViewModel, AllProfilesViewModel};
use super::project::ProjectViewModel;

/// What fills the main area of Agents mode (spec Decision 6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MainAreaViewModel<'a> {
    /// The profiles are read for the first time.
    Loading,
    /// No profile was found: no agent is installed.
    NoAgent,
    /// No project is selected: a card that says what the mode is for.
    Start,
    /// The selected project with its sessions table, boxed because it is
    /// much larger than the cards.
    Project(Box<ProjectViewModel<'a>>),
    /// The dashboard of the selected profile, boxed because it is much
    /// larger than the other screens.
    Profile(Box<DashboardViewModel>),
    /// The page of the selected account.
    Account(Box<AccountPageViewModel>),
    /// Every account and what the accounts need from the user.
    AllAccounts(Box<AllAccountsViewModel>),
    /// Every profile and what the profiles need from the user.
    AllProfiles(Box<AllProfilesViewModel>),
}

impl<'a> MainAreaViewModel<'a> {
    /// Build the main area for the current state, with the ages of the
    /// sessions counted up to `now`.
    pub(in super::super) fn from_state(
        state: &'a AgentsState,
        now: SystemTime,
    ) -> Self {
        if !state.profiles().is_loaded() {
            return Self::Loading;
        }
        if state.profiles().shown().is_empty() {
            return Self::NoAgent;
        }

        if state.is_all_accounts_selected() {
            return Self::AllAccounts(Box::new(
                AllAccountsViewModel::from_state(state),
            ));
        }
        if state.is_all_profiles_selected() {
            return Self::AllProfiles(Box::new(
                AllProfilesViewModel::from_state(state),
            ));
        }

        if let Some(account) = state.selected_account() {
            return Self::Account(Box::new(AccountPageViewModel::of(
                state, account, now,
            )));
        }
        if let Some(shown) = state.selected_profile() {
            return Self::Profile(Box::new(DashboardViewModel::from_profile(
                state, shown, now,
            )));
        }

        match state.selected_project() {
            Some(project) => Self::Project(Box::new(
                ProjectViewModel::from_project(state, project, now),
            )),
            None => Self::Start,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::event::AgentsIntent;
    use crate::reducer::reduce;
    use crate::testing::{
        agents, chatgpt_login, claude, codex, connected, env, now,
        two_agents_with_sessions, with_accounts, with_profiles,
    };

    fn select(state: &mut AgentsState, folder: &str) {
        let _ =
            reduce(state, AgentsIntent::SelectProject(PathBuf::from(folder)));
    }

    #[test]
    fn the_first_read_shows_that_it_loads() {
        let mut state = AgentsState::new(agents(), env());
        let _ = reduce(&mut state, AgentsIntent::Enter);

        assert_eq!(
            MainAreaViewModel::from_state(&state, now()),
            MainAreaViewModel::Loading
        );
    }

    #[test]
    fn with_no_profile_the_card_says_no_agent_was_found() {
        let state = with_profiles(&[]);

        assert_eq!(
            MainAreaViewModel::from_state(&state, now()),
            MainAreaViewModel::NoAgent
        );
    }

    /// The first read selects the first profile of the list, so the
    /// mode opens on its dashboard and not on a card.
    #[test]
    fn after_the_first_read_the_first_profile_shows_its_dashboard() {
        let state =
            with_profiles(&[(codex(), connected()), (claude(), connected())]);

        let vm = MainAreaViewModel::from_state(&state, now());

        assert!(matches!(
            vm,
            MainAreaViewModel::Profile(dashboard) if dashboard.profile == codex()
        ));
    }

    #[test]
    fn with_a_selected_project_that_is_gone_the_card_says_what_the_mode_is_for()
    {
        let mut state = two_agents_with_sessions();

        select(&mut state, "/work/none");
        assert_eq!(
            MainAreaViewModel::from_state(&state, now()),
            MainAreaViewModel::Start
        );
    }

    #[test]
    fn a_selected_profile_shows_its_dashboard() {
        let mut state = two_agents_with_sessions();
        let _ = reduce(&mut state, AgentsIntent::SelectProfile(codex()));

        let vm = MainAreaViewModel::from_state(&state, now());

        assert!(matches!(
            vm,
            MainAreaViewModel::Profile(dashboard) if dashboard.profile == codex()
        ));
    }

    #[test]
    fn all_accounts_and_all_profiles_show_their_pages() {
        let mut state = two_agents_with_sessions();

        let _ = reduce(&mut state, AgentsIntent::SelectAllAccounts);
        let accounts = matches!(
            MainAreaViewModel::from_state(&state, now()),
            MainAreaViewModel::AllAccounts(_)
        );
        let _ = reduce(&mut state, AgentsIntent::SelectAllProfiles);
        let profiles = MainAreaViewModel::from_state(&state, now());

        assert!(accounts);
        assert!(matches!(profiles, MainAreaViewModel::AllProfiles(_)));
    }

    #[test]
    fn a_selected_account_shows_its_page() {
        let mut state = with_accounts(
            &[
                (claude(), connected(), Vec::new()),
                (codex(), connected(), vec![chatgpt_login()]),
            ],
            otty_agents::SavedProfiles::default(),
        );
        let _ =
            reduce(&mut state, AgentsIntent::SelectAccount(chatgpt_login()));

        let vm = MainAreaViewModel::from_state(&state, now());

        assert!(matches!(
            vm,
            MainAreaViewModel::Account(page) if page.account.is_same_account(&chatgpt_login())
        ));
    }

    #[test]
    fn a_selected_project_shows_its_sessions_table() {
        let mut state = two_agents_with_sessions();
        select(&mut state, "/work/otty");

        let vm = MainAreaViewModel::from_state(&state, now());

        assert!(matches!(
            vm,
            MainAreaViewModel::Project(project) if project.name == "otty"
        ));
    }
}
