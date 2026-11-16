use std::time::SystemTime;

use otty_agents::Profile;
use otty_app_i18n::{self as i18n, Key};
use otty_ui_kit::components::session_row::SessionTone;

use super::super::state::{AgentsState, ProfileState};
use super::projects_section::ProjectsSectionViewModel;
use super::short_number::short_number;

/// The Agents mode left panel under the switcher: the Agents section over
/// the Projects section (spec Decision 6).
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

/// The Agents section: a row for each shown profile, then the Hidden
/// profiles row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentsSectionViewModel {
    /// Whether the section shows its rows.
    pub(crate) is_open: bool,
    /// The shown profiles, in their order.
    pub(crate) profiles: Vec<ProfileRowViewModel>,
    /// The Hidden profiles row and, while it is open, the hidden
    /// profiles; `None` while no profile is hidden.
    pub(crate) hidden: Option<HiddenProfilesViewModel>,
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
            .iter()
            .map(|shown| ProfileRowViewModel::from_profile(state, shown))
            .collect();

        let hidden =
            (!state.hidden().is_empty()).then(|| HiddenProfilesViewModel {
                label: i18n::with_count(
                    Key::TplAgentsHiddenProfiles,
                    state.hidden().len() as u64,
                ),
                is_open: state.is_hidden_list_open(),
                profiles: state
                    .hidden()
                    .iter()
                    .map(|profile| HiddenProfileViewModel {
                        icon: state.icon_of(profile.agent()),
                        profile: profile.clone(),
                    })
                    .collect(),
            });

        Self {
            is_open: state.is_agents_open(),
            profiles,
            hidden,
            can_add: state.is_loaded(),
            note: (!state.is_loaded()).then(|| i18n::t(Key::AgentsLoading)),
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

/// The Hidden profiles row and the hidden profiles under it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HiddenProfilesViewModel {
    /// The row text with the count.
    pub(crate) label: String,
    /// Whether the hidden profiles are listed.
    pub(crate) is_open: bool,
    /// The hidden profiles, in their order.
    pub(crate) profiles: Vec<HiddenProfileViewModel>,
}

/// One hidden profile with Show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HiddenProfileViewModel {
    /// The name of the agent's icon.
    pub(crate) icon: &'static str,
    /// The profile, sent with Show.
    pub(crate) profile: Profile,
}

#[cfg(test)]
mod tests {
    use otty_agents::ProfileStatus;

    use super::*;
    use crate::event::AgentsIntent;
    use crate::reducer::reduce;
    use crate::testing::{
        agents, at, claude, codex, codex_work, connected, env,
        five_hour_limits, limits_answer, limits_loaded, now, with_profiles,
    };

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
                limits_answer(vec![("server", Ok(five_hour_limits(42.0)))]),
                at(10),
            ),
        );

        let vm = LeftPanelViewModel::from_state(&state, now());

        assert_eq!(
            vm.agents.profiles,
            vec![
                ProfileRowViewModel {
                    profile: claude(),
                    is_selected: false,
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
    fn a_selected_profile_is_marked_and_hidden_ones_are_counted() {
        let mut state = with_profiles(&[
            (claude(), connected()),
            (codex(), connected()),
            (codex_work().with_hidden(true), connected()),
        ]);
        let _ = reduce(&mut state, AgentsIntent::SelectProfile(codex()));
        let _ = reduce(&mut state, AgentsIntent::ToggleHiddenProfiles);

        let vm = LeftPanelViewModel::from_state(&state, now());

        let selected: Vec<_> = vm
            .agents
            .profiles
            .iter()
            .map(|row| row.is_selected)
            .collect();
        assert_eq!(selected, [false, true]);
        assert!(vm.agents.can_add);
        assert_eq!(
            vm.agents.hidden,
            Some(HiddenProfilesViewModel {
                label: i18n::with_count(Key::TplAgentsHiddenProfiles, 1),
                is_open: true,
                profiles: vec![HiddenProfileViewModel {
                    icon: "codex",
                    profile: codex_work().with_hidden(true),
                }],
            })
        );
    }

    #[test]
    fn before_the_first_read_the_section_says_so_and_can_not_add() {
        let mut state = crate::state::AgentsState::new(agents(), env());
        let _ = reduce(&mut state, AgentsIntent::Enter);

        let vm = LeftPanelViewModel::from_state(&state, now());

        assert_eq!(vm.agents.hidden, None);
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
