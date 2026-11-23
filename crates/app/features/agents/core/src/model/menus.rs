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
    /// The menu of a profile: Rename…, Open folder, Edit, Log in, Hide or
    /// Show again, and Remove.
    Profile(ProfileMenuViewModel),
    /// The menu of an account: Rename…, Open usage page, Hide or Show
    /// again.
    Account(AccountMenuViewModel),
    /// The "⋯" of All accounts: the hidden accounts and the Found by OTTY
    /// report.
    AllAccounts(HiddenListMenuViewModel),
    /// The "⋯" of All profiles: the hidden profiles.
    AllProfiles(HiddenListMenuViewModel),
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
                        let id = agent.entry().id();
                        (id.clone(), state.agent_name(id).to_string())
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
                    can_remove: !profile.is_default(),
                    is_hidden: state.profiles().is_profile_hidden(profile),
                })
            },
            AgentsMenu::Account(account) => {
                Self::Account(AccountMenuViewModel {
                    place: open.place(),
                    can_save: account.can_be_saved(),
                    is_hidden: state.profiles().is_account_hidden(account),
                })
            },
            AgentsMenu::AllAccounts => {
                let profiles = state.profiles();
                let hidden = profiles
                    .all_accounts()
                    .iter()
                    .filter(|account| profiles.is_account_hidden(account))
                    .count();

                Self::AllAccounts(HiddenListMenuViewModel {
                    place: open.place(),
                    hidden,
                    is_showing: state.is_showing_hidden_accounts(),
                })
            },
            AgentsMenu::AllProfiles => {
                Self::AllProfiles(HiddenListMenuViewModel {
                    place: open.place(),
                    hidden: state.profiles().hidden_profiles().len(),
                    is_showing: state.is_showing_hidden_profiles(),
                })
            },
            AgentsMenu::UsagePeriod => {
                Self::UsagePeriod(UsagePeriodMenuViewModel {
                    place: open.place(),
                    picked: state.usage().period(),
                })
            },
            AgentsMenu::UsageMeasure => {
                // The menu of the usage block of a profile page.
                let money_reason =
                    state.selected_profile().and_then(|_| money_reason(state));
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
    pub(crate) agents: Vec<(AgentId, String)>,
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

/// The menu of a profile (spec 2026-11-19, Decision 7): Rename…, Open
/// folder, Edit, Log in while the profile is not logged in, Hide or Show
/// again, and Remove for an added profile.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ProfileMenuViewModel {
    /// Where the menu opens.
    pub(crate) place: MenuPlace,
    /// The profile, sent with Edit, Log in and Remove.
    pub(crate) profile: Profile,
    /// Whether Log in shows: the profile is not logged in and its program
    /// was found.
    pub(crate) can_log_in: bool,
    /// Whether Remove shows: only a profile the user added can be
    /// removed.
    pub(crate) can_remove: bool,
    /// Whether the profile is hidden: Show again in place of Hide.
    pub(crate) is_hidden: bool,
}

/// The menu of an account (spec 2026-11-19, Decision 7): Rename…, Open
/// usage page, and Hide or Show again. No account is added by hand, so
/// none has Edit key… or Remove (spec 2026-11-23, Decision 10).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AccountMenuViewModel {
    /// Where the menu opens.
    pub(crate) place: MenuPlace,
    /// Whether Rename… and Hide act: the account is known by an id, an
    /// organization or an email. A key is not kept, so they are off for
    /// it, with why.
    pub(crate) can_save: bool,
    /// Whether the account is hidden: Show again in place of Hide.
    pub(crate) is_hidden: bool,
}

/// The "⋯" of All accounts or All profiles: Show hidden … (N), off while
/// none is hidden, or Hide hidden … while they are shown.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct HiddenListMenuViewModel {
    /// Where the menu opens.
    pub(crate) place: MenuPlace,
    /// How many are hidden.
    pub(crate) hidden: usize,
    /// Whether the hidden ones are listed.
    pub(crate) is_showing: bool,
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use iced::{Point, Size};
    use otty_agents::{Account, ProfileStatus, SavedProfiles};
    use otty_app_i18n::{self as i18n, Key};

    use super::*;
    use crate::event::{AgentsIntent, SessionAction};
    use crate::project_filter::ProjectFilterChange;
    use crate::reducer::reduce;
    use crate::testing::{
        agents, api_key, chatgpt_login, claude, claude_id, claude_work, codex,
        codex_id, codex_work, connected, env, organization,
        profiles_with_hidden_loaded, read, session, two_agents_with_sessions,
        with_accounts, with_profiles,
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
                agents: vec![
                    (claude_id(), String::from("Claude Code")),
                    (codex_id(), String::from("Codex")),
                ],
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
            (claude(), crate::testing::not_logged_in(&claude())),
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
                is_hidden: false,
            }))
        );
        assert_eq!(
            added,
            Some(MenuViewModel::Profile(ProfileMenuViewModel {
                place: place(),
                profile: claude_work(),
                can_log_in: false,
                can_remove: true,
                is_hidden: false,
            }))
        );
        assert_eq!(
            not_found,
            Some(MenuViewModel::Profile(ProfileMenuViewModel {
                place: place(),
                profile: codex(),
                can_log_in: false,
                can_remove: false,
                is_hidden: false,
            })),
            "a program that is not found can not log in"
        );
    }

    fn account_menu(
        state: &mut AgentsState,
        account: Account,
    ) -> MenuViewModel {
        crate::testing::open_account_menu(state, account);

        MenuViewModel::from_state(state).expect("the menu is open")
    }

    /// Spec 2026-11-19, Decision 7: a login and an organization have
    /// Rename…, Open usage page and Hide; a key a part asks for is given
    /// from its need, not from the menu (issue 20).
    #[test]
    fn the_account_menu_of_a_login_and_of_an_organization_is_the_same() {
        let mut state = with_accounts(
            &[
                (claude(), api_key(), vec![organization()]),
                (codex(), connected(), vec![chatgpt_login()]),
            ],
            SavedProfiles::default(),
        );

        let login = account_menu(&mut state, chatgpt_login());
        let company = account_menu(&mut state, organization());

        assert_eq!(
            login,
            MenuViewModel::Account(AccountMenuViewModel {
                place: place(),
                can_save: true,
                is_hidden: false,
            })
        );
        assert_eq!(
            company,
            MenuViewModel::Account(AccountMenuViewModel {
                place: place(),
                can_save: true,
                is_hidden: false,
            })
        );
    }

    /// A key is known only by its fingerprint, which is not kept: its name
    /// and its hiding can not be saved, so Rename… and Hide are off.
    #[test]
    fn the_menu_of_a_key_can_not_rename_or_hide_it() {
        let key = Account::new(
            otty_agents::ProviderRef::of_id("deepseek"),
            otty_agents::AccountIdentity::KeyFingerprint(
                otty_agents::KeyFingerprint::of("sk-made-up-deepseek-key"),
            ),
            otty_agents::AccountLogin::Key(None),
            otty_agents::AccountKind::Money,
            otty_agents::FoundIn::ProfileFolder(PathBuf::from(
                "/home/user/.claude",
            )),
        );
        let mut state = with_accounts(
            &[(claude(), api_key(), vec![key.clone()])],
            SavedProfiles::default(),
        );

        let menu = account_menu(&mut state, key);

        assert_eq!(
            menu,
            MenuViewModel::Account(AccountMenuViewModel {
                place: place(),
                can_save: false,
                is_hidden: false,
            })
        );
    }

    #[test]
    fn the_menu_of_a_hidden_account_or_profile_shows_it_again() {
        let saved = SavedProfiles::default()
            .with_account_hidden(&chatgpt_login(), true)
            .with_profile_hidden(&codex_work(), true);
        let mut state =
            AgentsState::new(agents(), crate::testing::registry(), env());
        let _ = reduce(&mut state, AgentsIntent::Enter);
        let _ = reduce(
            &mut state,
            profiles_with_hidden_loaded(
                &[(codex(), connected(), vec![chatgpt_login()])],
                &[codex_work()],
                saved,
            ),
        );

        let account = account_menu(&mut state, chatgpt_login());
        let profile = profile_menu(&mut state, codex_work());

        assert_eq!(
            account,
            MenuViewModel::Account(AccountMenuViewModel {
                place: place(),
                can_save: true,
                is_hidden: true,
            })
        );
        assert_eq!(
            profile,
            Some(MenuViewModel::Profile(ProfileMenuViewModel {
                place: place(),
                profile: codex_work(),
                can_log_in: false,
                can_remove: true,
                is_hidden: true,
            }))
        );
    }

    /// The "⋯" of All accounts counts the hidden accounts, and has Show
    /// Found by OTTY; Show hidden is off while none is hidden.
    #[test]
    fn the_overview_menus_count_what_is_hidden() {
        let saved = SavedProfiles::default()
            .with_account_hidden(&chatgpt_login(), true);
        let mut state = with_accounts(
            &[(codex(), connected(), vec![chatgpt_login()])],
            saved,
        );
        state.set_pointer(place());

        let _ = reduce(&mut state, AgentsIntent::OpenAllAccountsMenu);
        let accounts = MenuViewModel::from_state(&state);
        let _ = reduce(&mut state, AgentsIntent::ShowHiddenAccounts(true));
        let _ = reduce(&mut state, AgentsIntent::OpenAllAccountsMenu);
        let showing = MenuViewModel::from_state(&state);
        let _ = reduce(&mut state, AgentsIntent::OpenAllProfilesMenu);
        let profiles = MenuViewModel::from_state(&state);

        assert_eq!(
            accounts,
            Some(MenuViewModel::AllAccounts(HiddenListMenuViewModel {
                place: place(),
                hidden: 1,
                is_showing: false,
            }))
        );
        assert_eq!(
            showing,
            Some(MenuViewModel::AllAccounts(HiddenListMenuViewModel {
                place: place(),
                hidden: 1,
                is_showing: true,
            }))
        );
        assert_eq!(
            profiles,
            Some(MenuViewModel::AllProfiles(HiddenListMenuViewModel {
                place: place(),
                hidden: 0,
                is_showing: false,
            }))
        );
    }

    /// Spec 2026-11-23, Decision 3: Money is off only while the prices
    /// are not known, with its reason, and the menu lights Tokens, the
    /// measure shown; then a subscription has Money too.
    #[test]
    fn the_measure_menu_has_money_off_only_until_the_prices_are_known() {
        let mut state = with_profiles(&[(codex(), connected())]);
        let _ = reduce(&mut state, AgentsIntent::SelectProfile(codex()));
        let _ =
            reduce(&mut state, AgentsIntent::PickMeasure(UsageMeasure::Money));
        state.set_pointer(place());

        let _ = reduce(&mut state, AgentsIntent::OpenMeasureMenu);
        let no_prices = MenuViewModel::from_state(&state);
        crate::testing::set_prices(&mut state, crate::testing::price_table());
        let priced = MenuViewModel::from_state(&state);

        assert_eq!(
            no_prices,
            Some(MenuViewModel::UsageMeasure(UsageMeasureMenuViewModel {
                place: place(),
                picked: UsageMeasure::Tokens,
                money_reason: Some(
                    i18n::t(Key::AgentsMoneyNoPrices).to_string()
                ),
            }))
        );
        assert_eq!(
            priced,
            Some(MenuViewModel::UsageMeasure(UsageMeasureMenuViewModel {
                place: place(),
                picked: UsageMeasure::Money,
                money_reason: None,
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
