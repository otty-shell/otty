use otty_agents::{AgentId, Profile};
use otty_ui_kit::components::context_menu::MenuPlace;

use super::super::menus::AgentsMenu;
use super::super::project_filter::ProjectFilter;
use super::super::sessions_table::SessionKey;
use super::super::state::AgentsState;
use super::super::usage::{UsageMeasure, UsagePeriod};
use super::profile_page::money_reason;

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
    /// The periods of the usage block.
    UsagePeriod(UsagePeriodMenuViewModel),
    /// The measures of the usage block.
    UsageMeasure(UsageMeasureMenuViewModel),
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
                    can_log_in: state.profiles().can_log_in(profile),
                    can_remove: is_added(profile),
                })
            },
            AgentsMenu::UsagePeriod => {
                Self::UsagePeriod(UsagePeriodMenuViewModel {
                    place: open.place(),
                    picked: state.usage().period(),
                })
            },
            AgentsMenu::UsageMeasure => {
                let money_reason = state
                    .selected_profile()
                    .and_then(|shown| money_reason(state, shown));
                let picked = if money_reason.is_some() {
                    UsageMeasure::Tokens
                } else {
                    state.usage().measure()
                };

                Self::UsageMeasure(UsageMeasureMenuViewModel {
                    place: open.place(),
                    picked,
                    money_reason,
                })
            },
            AgentsMenu::NewSession => {
                let profiles =
                    state.profiles().startable().into_iter().cloned().collect();

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
        let entry = state.selected_project().and_then(|project| {
            project.sessions().iter().find(|entry| key.is_of(entry))
        });
        let is_known_agent = entry.is_some_and(|entry| {
            state.agent(entry.profile().agent()).is_some()
        });
        let can_show_stats =
            entry.is_some_and(|entry| state.has_stats(entry.profile().agent()));

        Self {
            place,
            can_continue: entry.is_some_and(|entry| state.can_continue(entry)),
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
    /// Whether Remove shows: only a profile the user added can be
    /// removed.
    pub(crate) can_remove: bool,
}

/// The periods of the usage block, the picked one lit.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct UsagePeriodMenuViewModel {
    /// Where the menu opens.
    pub(crate) place: MenuPlace,
    /// The period shown.
    pub(crate) picked: UsagePeriod,
}

/// The measures of the usage block, the shown one lit; Money is off, with
/// its reason under it, when the profile on screen can not show it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct UsageMeasureMenuViewModel {
    /// Where the menu opens.
    pub(crate) place: MenuPlace,
    /// The measure shown.
    pub(crate) picked: UsageMeasure,
    /// Why Money is off; `None` when it is on.
    pub(crate) money_reason: Option<String>,
}

/// Whether the user added `profile`: it is neither the agent's default
/// profile nor one OTTY found.
fn is_added(profile: &Profile) -> bool {
    !profile.is_default() && !profile.is_found()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use iced::{Point, Size};
    use otty_agents::ProfileStatus;
    use otty_app_i18n::{self as i18n, Key};

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
                sessions: crate::types::SessionsRead::Read(vec![session(
                    "c1",
                    "/work/gone",
                    1,
                )]),
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

    /// A profile OTTY found is not in `agents.json`: removing it would
    /// change nothing, as it is found again.
    #[test]
    fn the_menu_of_a_found_profile_has_no_remove() {
        let found = Profile::found(
            claude().agent(),
            "Work",
            std::path::PathBuf::from("/home/user/.claude-work"),
        );
        let mut state = with_profiles(&[(found.clone(), connected())]);

        let menu = profile_menu(&mut state, found.clone());

        assert_eq!(
            menu,
            Some(MenuViewModel::Profile(ProfileMenuViewModel {
                place: place(),
                profile: found,
                can_log_in: false,
                can_remove: false,
            }))
        );
    }

    /// Spec 2026-11-19, Decision 6: Money is off for a subscription, with
    /// its reason, and the menu lights Tokens, the measure shown.
    #[test]
    fn the_measure_menu_of_a_subscription_has_money_off_with_its_reason() {
        let mut state = with_profiles(&[(codex(), connected())]);
        let _ = reduce(&mut state, AgentsIntent::SelectProfile(codex()));
        let _ =
            reduce(&mut state, AgentsIntent::PickMeasure(UsageMeasure::Money));
        state.set_pointer(place());

        let _ = reduce(&mut state, AgentsIntent::OpenMeasureMenu);
        let vm = MenuViewModel::from_state(&state);

        assert_eq!(
            vm,
            Some(MenuViewModel::UsageMeasure(UsageMeasureMenuViewModel {
                place: place(),
                picked: UsageMeasure::Tokens,
                money_reason: Some(
                    i18n::t(Key::AgentsMoneySubscription).to_string()
                ),
            }))
        );
    }

    #[test]
    fn a_picked_period_closes_its_menu_and_shows() {
        let mut state = with_profiles(&[(codex(), connected())]);
        state.set_pointer(place());
        let _ = reduce(&mut state, AgentsIntent::OpenPeriodMenu);
        let open = MenuViewModel::from_state(&state);

        let _ =
            reduce(&mut state, AgentsIntent::PickPeriod(UsagePeriod::Today));

        assert_eq!(
            open,
            Some(MenuViewModel::UsagePeriod(UsagePeriodMenuViewModel {
                place: place(),
                picked: UsagePeriod::SevenDays,
            }))
        );
        assert_eq!(MenuViewModel::from_state(&state), None);
        assert_eq!(state.usage().period(), UsagePeriod::Today);
    }
}
