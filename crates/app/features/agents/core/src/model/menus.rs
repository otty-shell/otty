use otty_agents::{AgentId, Profile};

use super::super::menus::{AgentsMenu, MenuPlace};
use super::super::project_filter::ProjectFilter;
use super::super::state::AgentsState;

/// The open menu of the left panel, with what it lists.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum MenuViewModel {
    /// The funnel menu of the projects.
    ProjectFilter(FilterMenuViewModel),
    /// The profiles a new session can start with.
    NewSession(NewSessionMenuViewModel),
}

impl MenuViewModel {
    /// The open menu; `None` while no menu is open.
    pub(in super::super) fn from_state(state: &AgentsState) -> Option<Self> {
        let open = state.menu()?;

        let vm = match open.menu() {
            AgentsMenu::ProjectFilter => {
                let agents = state
                    .agents()
                    .iter()
                    .map(|agent| {
                        let descriptor = agent.descriptor();
                        (descriptor.id(), descriptor.name())
                    })
                    .collect();

                Self::ProjectFilter(FilterMenuViewModel {
                    place: open.place(),
                    agents,
                    filter: state.search().filter().clone(),
                })
            },
            AgentsMenu::NewSession => {
                let profiles =
                    state.startable_profiles().into_iter().cloned().collect();

                Self::NewSession(NewSessionMenuViewModel {
                    place: open.place(),
                    profiles,
                })
            },
        };

        Some(vm)
    }
}

/// The funnel menu: the agents, what to show and the sort, with the
/// choices that are on.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FilterMenuViewModel {
    /// Where the menu opens.
    pub(crate) place: MenuPlace,
    /// Every agent OTTY knows: its id and its name.
    pub(crate) agents: Vec<(AgentId, &'static str)>,
    /// The choices that are on.
    pub(crate) filter: ProjectFilter,
}

/// The list of the profiles a new session in the selected project can
/// start with.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct NewSessionMenuViewModel {
    /// Where the menu opens.
    pub(crate) place: MenuPlace,
    /// The profiles, in their order.
    pub(crate) profiles: Vec<Profile>,
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use iced::{Point, Size};
    use otty_agents::ProfileStatus;

    use super::*;
    use crate::event::AgentsIntent;
    use crate::project_filter::ProjectFilterChange;
    use crate::reducer::reduce;
    use crate::testing::{
        CLAUDE, CODEX, claude, claude_work, codex, connected, read, session,
        two_agents_with_sessions, with_profiles,
    };

    fn place() -> MenuPlace {
        MenuPlace::new(Point::new(40.0, 200.0), Size::new(1000.0, 700.0))
    }

    #[test]
    fn no_open_menu_has_no_view_model() {
        let state = two_agents_with_sessions();

        assert_eq!(MenuViewModel::from_state(&state), None);
    }

    #[test]
    fn the_funnel_menu_lists_every_agent_and_the_choices_that_are_on() {
        let mut state = two_agents_with_sessions();
        let _ = reduce(
            &mut state,
            AgentsIntent::ProjectFilter(ProjectFilterChange::RecentOnly(true)),
        );
        state.set_pointer(place());
        let _ = reduce(&mut state, AgentsIntent::OpenFilterMenu);

        let vm = MenuViewModel::from_state(&state);

        let mut filter = ProjectFilter::default();
        filter.apply(ProjectFilterChange::RecentOnly(true));
        assert_eq!(
            vm,
            Some(MenuViewModel::ProjectFilter(FilterMenuViewModel {
                place: place(),
                agents: vec![(CLAUDE, "Claude Code"), (CODEX, "Codex")],
                filter,
            }))
        );
    }

    #[test]
    fn the_new_session_menu_lists_the_profiles_that_can_start() {
        let mut state = with_profiles(&[
            (claude(), connected()),
            (claude_work(), ProfileStatus::ProgramNotFound),
            (codex(), connected()),
        ]);
        read(&mut state, &claude(), vec![session("c1", "/work/otty", 1)]);
        let _ = reduce(
            &mut state,
            AgentsIntent::SelectProject(PathBuf::from("/work/otty")),
        );
        state.set_pointer(place());
        let _ = reduce(&mut state, AgentsIntent::OpenNewSessionMenu);

        let vm = MenuViewModel::from_state(&state);

        assert_eq!(
            vm,
            Some(MenuViewModel::NewSession(NewSessionMenuViewModel {
                place: place(),
                profiles: vec![claude(), codex()],
            }))
        );
    }
}
