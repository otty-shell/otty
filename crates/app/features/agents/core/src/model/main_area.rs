use std::time::SystemTime;

use super::super::state::AgentsState;
use super::account_page::AccountPageViewModel;
use super::overview::{AllAccountsViewModel, AllProfilesViewModel};
use super::profile_page::ProfilePageViewModel;
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
    /// The page of the selected profile, boxed because it is much larger
    /// than the other screens.
    Profile(Box<ProfilePageViewModel>),
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
        let profiles = state.profiles();
        if profiles.shown().is_empty() && profiles.hidden_profiles().is_empty()
        {
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
            return Self::Profile(Box::new(ProfilePageViewModel::of(
                state, shown, now,
            )));
        }
        if let Some(hidden) = state.selected_hidden_profile() {
            return Self::Profile(Box::new(ProfilePageViewModel::hidden(
                state, hidden,
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
        agents, at, chatgpt_login, claude, codex, connected, env,
        five_hour_limits, limits_answer, limits_loaded, now,
        profiles_with_accounts_loaded, two_agents_with_sessions, with_accounts,
        with_profiles,
    };

    fn select(state: &mut AgentsState, folder: &str) {
        let _ =
            reduce(state, AgentsIntent::SelectProject(PathBuf::from(folder)));
    }

    #[test]
    fn the_first_read_shows_that_it_loads() {
        let mut state =
            AgentsState::new(agents(), crate::testing::registry(), env());
        let _ = reduce(&mut state, AgentsIntent::Enter);

        assert_eq!(
            MainAreaViewModel::from_state(&state, now()),
            MainAreaViewModel::Loading
        );
    }

    /// Issue 24: a Refresh never brings the spinner back or empties the
    /// page. The page of the ChatGPT login shows the limits it showed until
    /// the new answer comes, then that answer.
    #[test]
    fn a_refresh_keeps_the_numbers_on_screen_until_new_ones_come() {
        let shown = [(codex(), connected(), vec![chatgpt_login()])];
        let mut state =
            with_accounts(&shown, otty_agents::SavedProfiles::default());
        let _ = reduce(&mut state, limits_at(&codex(), 42.0, at(10)));
        let _ =
            reduce(&mut state, AgentsIntent::SelectAccount(chatgpt_login()));
        let before = account_page(&state);

        let _ = reduce(&mut state, AgentsIntent::Refresh);
        let while_reading = account_page(&state);
        let _ = reduce(
            &mut state,
            profiles_with_accounts_loaded(
                &shown,
                otty_agents::SavedProfiles::default(),
            ),
        );
        let profiles_read = account_page(&state);
        let _ = reduce(&mut state, limits_at(&codex(), 80.0, at(20)));
        let limits_read = account_page(&state);

        assert!(before.is_some(), "the page of the login shows");
        assert_eq!(while_reading, before);
        assert_eq!(profiles_read, before);
        assert!(limits_read.is_some());
        assert_ne!(limits_read, before, "the new limits show");
    }

    /// The page of an account the main area shows; `None` for any other
    /// screen, the spinner too.
    fn account_page(state: &AgentsState) -> Option<AccountPageViewModel> {
        match MainAreaViewModel::from_state(state, now()) {
            MainAreaViewModel::Account(page) => Some(*page),
            _ => None,
        }
    }

    /// The limits of `profile` with `percent` of five hours used, read at
    /// `read_at`.
    fn limits_at(
        profile: &otty_agents::Profile,
        percent: f64,
        read_at: SystemTime,
    ) -> AgentsIntent {
        let answer = limits_answer(vec![(
            otty_agents::SourceId::ProviderPart,
            Ok(five_hour_limits(percent)),
        )]);

        limits_loaded(profile, answer, read_at)
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
            MainAreaViewModel::Profile(page) if page.profile == codex()
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
            MainAreaViewModel::Profile(page) if page.profile == codex()
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

    /// Spec 2026-11-19, Decision 7: a hidden profile is not read, so its
    /// page says it is hidden and shows only what OTTY knows without a
    /// read.
    #[test]
    fn a_hidden_profile_opens_a_page_that_says_it_is_hidden() {
        use otty_app_i18n::{self as i18n, Key};

        let mut state =
            AgentsState::new(agents(), crate::testing::registry(), env());
        let _ = reduce(&mut state, AgentsIntent::Enter);
        let _ = reduce(
            &mut state,
            crate::testing::profiles_with_hidden_loaded(
                &[(codex(), connected(), Vec::new())],
                &[crate::testing::codex_work()],
                otty_agents::SavedProfiles::default()
                    .with_profile_hidden(&crate::testing::codex_work(), true),
            ),
        );
        let _ = reduce(&mut state, AgentsIntent::ShowHiddenProfiles(true));
        let _ = reduce(
            &mut state,
            AgentsIntent::SelectProfile(crate::testing::codex_work()),
        );

        let vm = MainAreaViewModel::from_state(&state, now());

        let MainAreaViewModel::Profile(page) = vm else {
            panic!("the page of the hidden profile shows: {vm:?}");
        };
        assert_eq!(page.name, "Codex · Work");
        assert_eq!(
            page.info.as_deref(),
            Some(i18n::t(Key::AgentsHiddenProfileNote))
        );
        assert!(page.usage.is_none());
        assert!(page.needs.is_empty());
    }

    #[test]
    fn a_hidden_account_page_has_no_numbers_no_needs_and_says_so() {
        use otty_app_i18n::{self as i18n, Key};

        let saved = otty_agents::SavedProfiles::default()
            .with_account_hidden(&crate::testing::organization(), true);
        let mut state = with_accounts(
            &[(
                claude(),
                crate::testing::api_key(),
                vec![crate::testing::organization()],
            )],
            saved,
        );
        let _ = reduce(
            &mut state,
            AgentsIntent::SelectAccount(crate::testing::organization()),
        );

        let vm = MainAreaViewModel::from_state(&state, now());

        let MainAreaViewModel::Account(page) = vm else {
            panic!("the page of the hidden account shows: {vm:?}");
        };
        assert_eq!(
            page.hidden_note.as_deref(),
            Some(i18n::t(Key::AgentsHiddenAccountNote))
        );
        assert!(page.numbers.is_none());
        assert!(page.needs.is_empty());
    }

    /// A hidden account leaves the "Pays through" chips of its profiles.
    #[test]
    fn a_hidden_account_leaves_the_chips_of_its_profile() {
        let saved = otty_agents::SavedProfiles::default()
            .with_account_hidden(&chatgpt_login(), true);
        let mut state = with_accounts(
            &[(codex(), connected(), vec![chatgpt_login()])],
            saved,
        );
        let _ = reduce(&mut state, AgentsIntent::SelectProfile(codex()));

        let vm = MainAreaViewModel::from_state(&state, now());

        let MainAreaViewModel::Profile(page) = vm else {
            panic!("the page of the profile shows: {vm:?}");
        };
        assert!(page.pays_through.is_empty());
    }
}
