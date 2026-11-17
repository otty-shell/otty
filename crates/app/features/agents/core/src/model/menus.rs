use otty_agents::{AgentId, Profile};

use super::super::menus::{AgentsMenu, MenuPlace};
use super::super::project_filter::ProjectFilter;
use super::super::sessions_table::SessionKey;
use super::super::state::AgentsState;

/// The open menu of Agents mode, with what it lists.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum MenuViewModel {
    /// The funnel menu of the projects.
    ProjectFilter(FilterMenuViewModel),
    /// The profiles a new session can start with.
    NewSession(NewSessionMenuViewModel),
    /// The actions of one session of the sessions table.
    Session(SessionMenuViewModel),
    /// Edit, Log in and Remove of the profile on screen.
    Profile(ProfileMenuViewModel),
}

impl MenuViewModel {
    /// The open menu; `None` while no menu is open.
    pub(in super::super) fn from_state(state: &AgentsState) -> Option<Self> {
        let open = state.menu()?;

        let vm = match open.menu() {
            AgentsMenu::Session(key) => Self::Session(
                SessionMenuViewModel::from_key(state, key, open.place()),
            ),
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
            AgentsMenu::Profile(profile) => {
                Self::Profile(ProfileMenuViewModel {
                    place: open.place(),
                    profile: profile.clone(),
                    can_log_in: state.can_log_in(profile),
                    can_remove: !profile.is_default(),
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

/// The menu of one session: Continue, Continue in a split, Copy resume
/// command, Copy session ID and Show statistics, each enabled when it can
/// do something.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SessionMenuViewModel {
    /// Where the menu opens.
    pub(crate) place: MenuPlace,
    /// Whether Continue and Continue in a split can open the session: its
    /// folder exists and its profile can start.
    pub(crate) can_continue: bool,
    /// Whether the command that continues it is known: its agent is.
    pub(crate) can_copy_command: bool,
    /// Whether Show statistics can open: its agent reads statistics.
    pub(crate) can_show_stats: bool,
}

impl SessionMenuViewModel {
    /// The menu of the session of `key` in the selected project, open at
    /// `place`. A session that is no longer listed can only have its id
    /// copied.
    fn from_key(
        state: &AgentsState,
        key: &SessionKey,
        place: MenuPlace,
    ) -> Self {
        let project = state.selected_project();
        let entry = project.and_then(|project| {
            project.sessions().iter().find(|entry| key.is_of(entry))
        });
        let is_folder_there =
            project.is_some_and(|project| !state.is_missing(project.folder()));
        let is_known_agent = entry.is_some_and(|entry| {
            state.agent(entry.profile().agent()).is_some()
        });
        let can_start =
            entry.is_some_and(|entry| state.can_start(entry.profile()));
        let can_show_stats =
            entry.is_some_and(|entry| state.has_stats(entry.profile().agent()));

        Self {
            place,
            can_continue: is_folder_there && can_start && is_known_agent,
            can_copy_command: is_known_agent,
            can_show_stats,
        }
    }
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

/// The menu of a profile's dashboard: Edit, Log in while the profile is
/// not logged in, and Remove for an added profile.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ProfileMenuViewModel {
    /// Where the menu opens.
    pub(crate) place: MenuPlace,
    /// The profile, sent with the items.
    pub(crate) profile: Profile,
    /// Whether Log in shows: the profile is not logged in and its program
    /// was found.
    pub(crate) can_log_in: bool,
    /// Whether Remove shows: only an added profile can be removed.
    pub(crate) can_remove: bool,
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use iced::{Point, Size};
    use otty_agents::ProfileStatus;

    use super::*;
    use crate::event::{AgentsIntent, SessionAction};
    use crate::project_filter::ProjectFilterChange;
    use crate::reducer::reduce;
    use crate::testing::{
        CLAUDE, CODEX, claude, claude_work, codex, connected, read, session,
        two_agents_with_sessions, with_profiles,
    };
    use crate::types::LoadedSessions;

    /// The menu of the first row of the project of `folder`, opened at
    /// [`place`].
    fn session_menu(state: &mut AgentsState, folder: &str) -> MenuViewModel {
        let _ =
            reduce(state, AgentsIntent::SelectProject(PathBuf::from(folder)));
        state.set_pointer(place());
        let _ = reduce(state, AgentsIntent::OpenSessionMenu(0));

        MenuViewModel::from_state(state).expect("the menu is open")
    }

    #[test]
    fn the_menu_of_a_session_that_can_open_has_every_item_on() {
        let mut state = two_agents_with_sessions();

        let vm = session_menu(&mut state, "/work/otty");

        assert_eq!(
            vm,
            MenuViewModel::Session(SessionMenuViewModel {
                place: place(),
                can_continue: true,
                can_copy_command: true,
                can_show_stats: true,
            })
        );
    }

    #[test]
    fn a_session_whose_program_is_not_found_can_only_be_copied() {
        let mut state =
            with_profiles(&[(claude(), ProfileStatus::ProgramNotFound)]);
        read(&mut state, &claude(), vec![session("c1", "/work/otty", 1)]);

        let vm = session_menu(&mut state, "/work/otty");

        assert_eq!(
            vm,
            MenuViewModel::Session(SessionMenuViewModel {
                place: place(),
                can_continue: false,
                can_copy_command: true,
                can_show_stats: true,
            })
        );
    }

    #[test]
    fn a_session_of_a_gone_folder_can_not_continue() {
        let mut state = with_profiles(&[(claude(), connected())]);
        let _ = reduce(
            &mut state,
            AgentsIntent::SessionsLoaded(LoadedSessions {
                profile: claude(),
                sessions: Ok(vec![session("c1", "/work/gone", 1)]),
                missing_folders: vec![PathBuf::from("/work/gone")],
            }),
        );

        let vm = session_menu(&mut state, "/work/gone");

        assert_eq!(
            vm,
            MenuViewModel::Session(SessionMenuViewModel {
                place: place(),
                can_continue: false,
                can_copy_command: true,
                can_show_stats: true,
            })
        );
    }

    #[test]
    fn a_session_gone_after_a_read_keeps_a_menu_with_nothing_to_open() {
        let mut state = two_agents_with_sessions();
        let _ = session_menu(&mut state, "/work/otty");
        read(&mut state, &codex(), Vec::new());

        let vm = MenuViewModel::from_state(&state);
        let outcomes = reduce(
            &mut state,
            AgentsIntent::SessionMenu(SessionAction::Continue),
        );

        assert_eq!(
            vm,
            Some(MenuViewModel::Session(SessionMenuViewModel {
                place: place(),
                can_continue: false,
                can_copy_command: false,
                can_show_stats: false,
            }))
        );
        assert!(outcomes.is_empty());
    }

    #[test]
    fn a_session_of_an_agent_that_reads_no_statistics_has_them_off() {
        let stranger = otty_agents::Profile::default_for(
            otty_agents::AgentId::new("stranger"),
            "Stranger",
            PathBuf::from("/home/user/.stranger"),
        );
        let mut state = with_profiles(&[(stranger.clone(), connected())]);
        read(&mut state, &stranger, vec![session("s1", "/work/otty", 1)]);

        let vm = session_menu(&mut state, "/work/otty");

        assert_eq!(
            vm,
            MenuViewModel::Session(SessionMenuViewModel {
                place: place(),
                can_continue: false,
                can_copy_command: false,
                can_show_stats: false,
            })
        );
    }

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

    /// The view model of the menu of `profile`, opened at [`place`].
    fn profile_menu(
        state: &mut AgentsState,
        profile: Profile,
    ) -> Option<MenuViewModel> {
        state.set_pointer(place());
        let _ = reduce(state, AgentsIntent::OpenProfileMenu(profile));

        MenuViewModel::from_state(state)
    }

    #[test]
    fn the_profile_menu_offers_log_in_only_while_not_logged_in_and_remove_only_for_an_added_profile()
     {
        let mut state = with_profiles(&[
            (claude(), ProfileStatus::NotLoggedIn),
            (claude_work(), connected()),
            (codex(), ProfileStatus::ProgramNotFound),
        ]);

        let default = profile_menu(&mut state, claude());
        let added = profile_menu(&mut state, claude_work());
        let not_found = profile_menu(&mut state, codex());

        assert_eq!(
            default,
            Some(MenuViewModel::Profile(ProfileMenuViewModel {
                place: place(),
                profile: claude(),
                can_log_in: true,
                can_remove: false,
            }))
        );
        assert_eq!(
            added,
            Some(MenuViewModel::Profile(ProfileMenuViewModel {
                place: place(),
                profile: claude_work(),
                can_log_in: false,
                can_remove: true,
            }))
        );
        assert_eq!(
            not_found,
            Some(MenuViewModel::Profile(ProfileMenuViewModel {
                place: place(),
                profile: codex(),
                can_log_in: false,
                can_remove: false,
            })),
            "a program that is not found can not log in"
        );
    }
}
