use std::time::SystemTime;

use otty_agents::Profile;
use otty_app_i18n::{self as i18n, Key};
use otty_ui_kit::components::session_row::SessionTone;

use super::super::profile_state::ProfileState;
use super::super::state::AgentsState;
use super::projects_section::ProjectsSectionViewModel;
use super::short_number::short_number;

/// The Agents mode left panel under the switcher: the Profiles section
/// over the Projects section (spec Decisions 6 and 10).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LeftPanelViewModel<'a> {
    /// The profiles.
    pub(crate) agents: AgentsSectionViewModel,
    /// The projects with the search field.
    pub(crate) projects: ProjectsSectionViewModel<'a>,
}

impl<'a> LeftPanelViewModel<'a> {
    /// Build the left panel for the current state at `now`, the time the
    /// last 30 days of the project filter count back from.
    pub(in super::super) fn from_state(
        state: &'a AgentsState,
        now: SystemTime,
    ) -> Self {
        Self {
            agents: AgentsSectionViewModel::from_state(state),
            projects: ProjectsSectionViewModel::from_state(state, now),
        }
    }
}

/// The Profiles section: a row for each profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentsSectionViewModel {
    /// Whether the section shows its rows.
    pub(crate) is_open: bool,
    /// The profiles, in their order.
    pub(crate) profiles: Vec<ProfileRowViewModel>,
    /// Whether the `+` of the header adds a profile: once the profiles
    /// were read.
    pub(crate) can_add: bool,
    /// The text in place of the rows while the profiles are read for the
    /// first time.
    pub(crate) note: Option<&'static str>,
}

impl AgentsSectionViewModel {
    fn from_state(state: &AgentsState) -> Self {
        let profiles = state
            .profiles()
            .shown()
            .iter()
            .map(|shown| ProfileRowViewModel::from_profile(state, shown))
            .collect();

        Self {
            is_open: state.is_agents_open(),
            profiles,
            can_add: state.profiles().is_loaded(),
            note: (!state.profiles().is_loaded())
                .then(|| i18n::t(Key::AgentsLoading)),
        }
    }
}

/// One profile: its agent's logo, its name and its short number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProfileRowViewModel {
    /// The profile, sent when the row is pressed.
    pub(crate) profile: Profile,
    /// Whether the profile is the one selected.
    pub(crate) is_selected: bool,
    /// The name of the agent's icon.
    pub(crate) icon: &'static str,
    /// The profile name.
    pub(crate) name: String,
    /// The short number of its billing, or the state that needs the user;
    /// `None` while nothing is known.
    pub(crate) number: Option<String>,
    /// The color of the number.
    pub(crate) tone: SessionTone,
}

impl ProfileRowViewModel {
    fn from_profile(state: &AgentsState, shown: &ProfileState) -> Self {
        let profile = shown.profile();
        let is_selected = state
            .selected_profile()
            .is_some_and(|selected| selected.profile() == profile);
        let (number, tone) = match short_number(shown) {
            Some((number, tone)) => (Some(number), tone),
            None => (None, SessionTone::Muted),
        };

        Self {
            profile: profile.clone(),
            is_selected,
            icon: state.icon_of(profile.agent()),
            name: profile.name().to_string(),
            number,
            tone,
        }
    }
}

#[cfg(test)]
mod tests {
    use otty_agents::ProfileStatus;

    use super::*;
    use crate::event::AgentsIntent;
    use crate::reducer::reduce;
    use crate::testing::{
        agents, at, claude, codex, connected, env, five_hour_limits,
        limits_answer, limits_loaded, now, with_profiles,
    };

    /// The first read selects the first profile, so its row is marked.
    #[test]
    fn each_profile_shows_its_logo_name_and_short_number() {
        let mut state = with_profiles(&[
            (claude(), connected()),
            (codex(), ProfileStatus::NotLoggedIn),
        ]);
        let _ = reduce(
            &mut state,
            limits_loaded(
                &claude(),
                limits_answer(vec![(
                    otty_agents::SourceId::ChatGptUsageApi,
                    Ok(five_hour_limits(42.0)),
                )]),
                at(10),
            ),
        );

        let vm = LeftPanelViewModel::from_state(&state, now());

        assert_eq!(
            vm.agents.profiles,
            vec![
                ProfileRowViewModel {
                    profile: claude(),
                    is_selected: true,
                    icon: "claude-code",
                    name: String::from("Claude Code"),
                    number: Some(i18n::with_count(Key::TplAgentsMostUsed, 42)),
                    tone: SessionTone::Muted,
                },
                ProfileRowViewModel {
                    profile: codex(),
                    is_selected: false,
                    icon: "codex",
                    name: String::from("Codex"),
                    number: Some(
                        i18n::t(Key::AgentsStateNeedsLogin).to_string()
                    ),
                    tone: SessionTone::Failed,
                },
            ]
        );
        assert!(vm.agents.is_open);
        assert_eq!(vm.agents.note, None);
    }

    #[test]
    fn a_profile_with_nothing_known_shows_no_number() {
        let state = with_profiles(&[(claude(), connected())]);

        let vm = LeftPanelViewModel::from_state(&state, now());

        assert_eq!(vm.agents.profiles[0].number, None);
    }

    #[test]
    fn a_selected_profile_is_marked() {
        let mut state =
            with_profiles(&[(claude(), connected()), (codex(), connected())]);
        let _ = reduce(&mut state, AgentsIntent::SelectProfile(codex()));

        let vm = LeftPanelViewModel::from_state(&state, now());

        let selected: Vec<_> = vm
            .agents
            .profiles
            .iter()
            .map(|row| row.is_selected)
            .collect();
        assert_eq!(selected, [false, true]);
        assert!(vm.agents.can_add);
    }

    #[test]
    fn before_the_first_read_the_section_says_so_and_can_not_add() {
        let mut state = crate::state::AgentsState::new(agents(), env());
        let _ = reduce(&mut state, AgentsIntent::Enter);

        let vm = LeftPanelViewModel::from_state(&state, now());

        assert!(vm.agents.profiles.is_empty());
        assert!(!vm.agents.can_add);
        assert_eq!(vm.agents.note, Some(i18n::t(Key::AgentsLoading)));
    }

    #[test]
    fn a_closed_section_says_so() {
        let mut state = with_profiles(&[(claude(), connected())]);
        let _ = reduce(&mut state, AgentsIntent::ToggleAgentsSection);

        let vm = LeftPanelViewModel::from_state(&state, now());

        assert!(!vm.agents.is_open);
        assert!(vm.projects.is_open);
    }
}
