use std::time::SystemTime;

use super::super::state::AgentsState;
use super::profile_details::ProfileDetailsViewModel;
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
    /// The details of the selected profile, boxed because they are much
    /// larger than the other screens.
    Profile(Box<ProfileDetailsViewModel>),
}

impl<'a> MainAreaViewModel<'a> {
    /// Build the main area for the current state, with the ages of the
    /// sessions counted up to `now`.
    pub(in super::super) fn from_state(
        state: &'a AgentsState,
        now: SystemTime,
    ) -> Self {
        if !state.is_loaded() {
            return Self::Loading;
        }
        if state.profiles().is_empty() {
            return Self::NoAgent;
        }

        if let Some(shown) = state.selected_profile() {
            return Self::Profile(Box::new(
                ProfileDetailsViewModel::from_profile(state, shown, now),
            ));
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
        agents, codex, env, now, two_agents_with_sessions, with_profiles,
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

    #[test]
    fn with_no_project_selected_the_card_says_what_the_mode_is_for() {
        let mut state = two_agents_with_sessions();

        assert_eq!(
            MainAreaViewModel::from_state(&state, now()),
            MainAreaViewModel::Start
        );

        select(&mut state, "/work/none");
        assert_eq!(
            MainAreaViewModel::from_state(&state, now()),
            MainAreaViewModel::Start
        );
    }

    #[test]
    fn a_selected_profile_shows_its_details() {
        let mut state = two_agents_with_sessions();
        let _ = reduce(&mut state, AgentsIntent::SelectProfile(codex()));

        let vm = MainAreaViewModel::from_state(&state, now());

        assert!(matches!(
            vm,
            MainAreaViewModel::Profile(details) if details.profile == codex()
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
