use std::time::SystemTime;

use otty_agents::{Account, Profile};
use otty_app_i18n::{self as i18n, Key};
use otty_ui_kit::components::session_row::SessionTone;

use super::super::own_login_actions::has_own_login;
use super::super::profile_state::ProfileState;
use super::super::state::AgentsState;
use super::account_name::short_account_name;
use super::entry_icon::EntryIcon;
use super::projects_section::ProjectsSectionViewModel;
use super::short_number::{
    AccountNumber, account_number, hidden_word, profile_number,
};

/// The Agents mode left panel under the switcher: the Accounts section,
/// the Profiles section and the Projects section (spec 2026-11-19,
/// Decision 1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LeftPanelViewModel<'a> {
    /// All accounts and the accounts.
    pub(crate) accounts: AccountsSectionViewModel,
    /// All profiles and the profiles.
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
            accounts: AccountsSectionViewModel::from_state(state),
            agents: AgentsSectionViewModel::from_state(state),
            projects: ProjectsSectionViewModel::from_state(state, now),
        }
    }
}

/// The first row of the Accounts or the Profiles section, All accounts or
/// All profiles: how many rows follow it, and whether its page is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AllRowViewModel {
    /// How many accounts or profiles there are.
    pub(crate) count: usize,
    /// Whether its page is the one shown.
    pub(crate) is_selected: bool,
}

/// The Accounts section: All accounts, then a row for each account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AccountsSectionViewModel {
    /// Whether the section shows its rows.
    pub(crate) is_open: bool,
    /// All accounts, once the profiles were read.
    pub(crate) all: Option<AllRowViewModel>,
    /// The accounts, in the order of the profiles they were found in.
    pub(crate) accounts: Vec<AccountRowViewModel>,
    /// Whether `+` (Add account) acts: `agents.json` was read, so OTTY's
    /// own login can be saved.
    pub(crate) can_add: bool,
    /// The text in place of the rows while the profiles are read for the
    /// first time.
    pub(crate) note: Option<&'static str>,
}

impl AccountsSectionViewModel {
    fn from_state(state: &AgentsState) -> Self {
        let is_loaded = state.profiles().is_loaded();
        let selected = state.selected_account();
        let accounts: Vec<AccountRowViewModel> = state
            .listed_accounts()
            .into_iter()
            .map(|account| {
                AccountRowViewModel::from_account(
                    state,
                    account,
                    selected.as_ref(),
                )
            })
            .collect();

        Self {
            is_open: state.is_accounts_open(),
            all: is_loaded.then(|| AllRowViewModel {
                count: accounts.len(),
                is_selected: state.is_all_accounts_selected(),
            }),
            accounts,
            can_add: state.profiles().saved().is_some(),
            note: (!is_loaded).then(|| i18n::t(Key::AgentsLoading)),
        }
    }
}

/// One account: the logo of its provider, its name and its number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AccountRowViewModel {
    /// The account, sent when the row is pressed.
    pub(crate) account: Account,
    /// Whether the account is the one selected.
    pub(crate) is_selected: bool,
    /// The icon of its provider.
    pub(crate) icon: EntryIcon,
    /// The login without the provider its logo shows, such as
    /// "you@example.com", or the name the user gave the account.
    pub(crate) name: String,
    /// Its number with its word and color, such as "42 % used", which
    /// draws itself in the tight form; `None` while nothing is known.
    pub(crate) number: Option<AccountNumber>,
    /// Whether the account has OTTY's own login: the row has the tag
    /// "OTTY" (issue 09 of spec 2026-11-23-accounts-apart-from-profiles).
    pub(crate) has_own_login: bool,
}

impl AccountRowViewModel {
    fn from_account(
        state: &AgentsState,
        account: Account,
        selected: Option<&Account>,
    ) -> Self {
        let is_selected =
            selected.is_some_and(|selected| selected.is_same_account(&account));
        Self {
            is_selected,
            icon: state.account_icon(&account),
            name: short_account_name(state.profiles().saved(), &account),
            number: account_number(state, &account),
            has_own_login: has_own_login(state, &account),
            account,
        }
    }
}

/// The Profiles section: All profiles, then a row for each profile, the
/// hidden ones last while they are listed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentsSectionViewModel {
    /// Whether the section shows its rows.
    pub(crate) is_open: bool,
    /// All profiles, once the profiles were read.
    pub(crate) all: Option<AllRowViewModel>,
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
        let is_loaded = state.profiles().is_loaded();
        let shown = state
            .profiles()
            .shown()
            .iter()
            .map(|shown| ProfileRowViewModel::from_profile(state, shown));
        let hidden = state
            .listed_hidden_profiles()
            .iter()
            .map(|profile| ProfileRowViewModel::hidden(state, profile));
        let profiles: Vec<ProfileRowViewModel> = shown.chain(hidden).collect();

        Self {
            is_open: state.is_agents_open(),
            all: is_loaded.then(|| AllRowViewModel {
                count: profiles.len(),
                is_selected: state.is_all_profiles_selected(),
            }),
            profiles,
            can_add: is_loaded,
            note: (!is_loaded).then(|| i18n::t(Key::AgentsLoading)),
        }
    }
}

/// One profile: its agent's logo, its name and its number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProfileRowViewModel {
    /// The profile, sent when the row is pressed.
    pub(crate) profile: Profile,
    /// Whether the profile is the one selected.
    pub(crate) is_selected: bool,
    /// The icon of its agent.
    pub(crate) icon: EntryIcon,
    /// The profile name.
    pub(crate) name: String,
    /// Its number with its word, such as "1.8M today", or the state that
    /// needs the user; `None` while nothing is known.
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
        let (number, tone) = match profile_number(shown) {
            Some((number, tone)) => (Some(number), tone),
            None => (None, SessionTone::Muted),
        };

        Self {
            profile: profile.clone(),
            is_selected,
            icon: state.agent_icon(profile.agent()),
            name: profile.name().to_string(),
            number,
            tone,
        }
    }

    /// The row of the hidden `profile`: "hidden", as nothing of it is read.
    fn hidden(state: &AgentsState, profile: &Profile) -> Self {
        let is_selected = state
            .selected_hidden_profile()
            .is_some_and(|selected| selected == profile);

        Self {
            profile: profile.clone(),
            is_selected,
            icon: state.agent_icon(profile.agent()),
            name: profile.name().to_string(),
            number: Some(hidden_word()),
            tone: SessionTone::Muted,
        }
    }
}

#[cfg(test)]
mod tests {
    use otty_agents::SavedProfiles;

    use super::*;
    use crate::event::AgentsIntent;
    use crate::reducer::reduce;
    use crate::testing::{
        agents, at, chatgpt_login, chatgpt_login_in, claude, codex, connected,
        env, five_hour_limits, limits_answer, limits_loaded, now, organization,
        spend_loaded, spend_of, with_accounts, with_profiles,
    };
    use crate::types::SpendCount;

    /// Claude Code paying through the [`organization`] and Codex logged in
    /// to the [`chatgpt_login`], with its limits read: 42 % used.
    fn two_accounts() -> AgentsState {
        let mut state = with_accounts(
            &[
                (claude(), connected(), vec![organization()]),
                (
                    codex(),
                    connected(),
                    vec![chatgpt_login_in("/home/user/.codex")],
                ),
            ],
            SavedProfiles::default(),
        );
        let _ = reduce(
            &mut state,
            limits_loaded(
                &codex(),
                limits_answer(vec![(
                    otty_agents::SourceId::ProviderPart,
                    Ok(five_hour_limits(42.0)),
                )]),
                at(10),
            ),
        );

        state
    }

    /// Spec Decision 1: All accounts first, then each account with its
    /// number and its word.
    #[test]
    fn the_accounts_section_starts_with_all_accounts_then_each_account() {
        let state = two_accounts();

        let vm = LeftPanelViewModel::from_state(&state, now());

        assert_eq!(
            vm.accounts.all,
            Some(AllRowViewModel {
                count: 2,
                is_selected: false,
            })
        );
        assert_eq!(
            vm.accounts.accounts,
            vec![
                AccountRowViewModel {
                    account: organization(),
                    is_selected: false,
                    icon: crate::testing::provider_icon("anthropic"),
                    name: String::from("dev@example.com"),
                    number: None,
                    has_own_login: false,
                },
                AccountRowViewModel {
                    account: chatgpt_login_in("/home/user/.codex"),
                    is_selected: false,
                    icon: crate::testing::provider_icon("openai"),
                    name: String::from("you@example.com"),
                    number: Some(AccountNumber::word(
                        i18n::with_count(Key::TplAgentsPercentUsed, 42),
                        SessionTone::Muted
                    )),
                    has_own_login: false,
                },
            ]
        );
        assert!(vm.accounts.is_open);
        assert_eq!(vm.accounts.note, None);
    }

    /// Issue 34: a row names its account by the login alone, as its logo
    /// already says the provider; a name the user gave is shown as given.
    #[test]
    fn an_account_row_is_its_login_or_the_name_the_user_gave() {
        let saved = SavedProfiles::default()
            .with_account_name(&organization(), Some(String::from("Work")));
        let state = with_accounts(
            &[
                (claude(), connected(), vec![organization()]),
                (codex(), connected(), vec![chatgpt_login()]),
            ],
            saved,
        );

        let vm = LeftPanelViewModel::from_state(&state, now());

        let names: Vec<&str> = vm
            .accounts
            .accounts
            .iter()
            .map(|row| row.name.as_str())
            .collect();
        assert_eq!(names, ["Work", "you@example.com"]);
    }

    /// Spec 2026-11-23-accounts-apart-from-profiles, Decision 1: a login
    /// two plugins find is two accounts, one row each, logo and name as
    /// any account: no plugin after the name.
    #[test]
    fn a_login_two_plugins_find_is_two_rows() {
        let fork = otty_agents::ProviderEntry::new(
            otty_agents::ProviderId::new("openai"),
            "OpenAI",
            otty_plugin_host::PluginPart::new(
                "openai-fork",
                "OpenAI Fork",
                "0.1.0",
                "",
                "openai",
            ),
        );
        let of_fork = otty_agents::Account::new(
            "openai-fork",
            otty_agents::ProviderId::new("openai"),
            "chatgpt:made-up-account",
            "you@example.com",
            otty_agents::AccountKind::Subscription,
            Vec::new(),
        );
        let state = crate::testing::with_accounts_in(
            crate::testing::registry_with(fork),
            &[(codex(), connected(), vec![chatgpt_login(), of_fork])],
            SavedProfiles::default(),
        );

        let vm = LeftPanelViewModel::from_state(&state, now());

        let names: Vec<&str> = vm
            .accounts
            .accounts
            .iter()
            .map(|row| row.name.as_str())
            .collect();
        assert_eq!(names, ["you@example.com", "you@example.com"]);
    }

    /// Issue 09 of spec 2026-11-23-accounts-apart-from-profiles: the row
    /// of an account with OTTY's own login is marked so; a found one is
    /// not.
    #[test]
    fn the_row_of_an_own_login_is_marked() {
        let own = crate::testing::with_own_login(vec![
            crate::testing::chatgpt_own_login(),
        ]);
        let found = two_accounts();

        let own_rows = LeftPanelViewModel::from_state(&own, now()).accounts;
        let found_rows = LeftPanelViewModel::from_state(&found, now()).accounts;

        assert!(own_rows.accounts[0].has_own_login);
        assert!(found_rows.accounts.iter().all(|row| !row.has_own_login));
    }

    #[test]
    fn a_selected_account_and_all_accounts_are_marked() {
        let mut state = two_accounts();
        let _ =
            reduce(&mut state, AgentsIntent::SelectAccount(chatgpt_login()));
        let account = LeftPanelViewModel::from_state(&state, now()).accounts;
        let marked: Vec<bool> =
            account.accounts.iter().map(|row| row.is_selected).collect();
        let all_marked = account.all.map(|row| row.is_selected);

        let _ = reduce(&mut state, AgentsIntent::SelectAllAccounts);
        let all = LeftPanelViewModel::from_state(&state, now());

        assert_eq!(marked, [false, true]);
        assert_eq!(all_marked, Some(false));
        assert_eq!(all.accounts.all.map(|row| row.is_selected), Some(true));
        assert!(all.accounts.accounts.iter().all(|row| !row.is_selected));
        assert!(all.agents.profiles.iter().all(|row| !row.is_selected));
    }

    /// The first read selects the first profile, so its row is marked.
    #[test]
    fn each_profile_shows_its_logo_name_and_number_with_its_word() {
        let mut state = with_profiles(&[
            (claude(), connected()),
            (codex(), crate::testing::not_logged_in(&codex())),
        ]);
        let _ = reduce(
            &mut state,
            spend_loaded(
                &claude(),
                SpendCount::Counted(spend_of(1_800_000, 1)),
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
                    icon: crate::testing::agent_icon("claude-code"),
                    name: String::from("Claude Code"),
                    number: Some(i18n::filled(
                        Key::TplAgentsAmountToday,
                        &[("amount", "1.8M")]
                    )),
                    tone: SessionTone::Muted,
                },
                ProfileRowViewModel {
                    profile: codex(),
                    is_selected: false,
                    icon: crate::testing::agent_icon("codex"),
                    name: String::from("Codex"),
                    number: Some(
                        i18n::t(Key::AgentsStateNeedsLogin).to_string()
                    ),
                    tone: SessionTone::Failed,
                },
            ]
        );
        assert_eq!(
            vm.agents.all,
            Some(AllRowViewModel {
                count: 2,
                is_selected: false,
            })
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
    fn a_selected_profile_and_all_profiles_are_marked() {
        let mut state =
            with_profiles(&[(claude(), connected()), (codex(), connected())]);
        let _ = reduce(&mut state, AgentsIntent::SelectProfile(codex()));
        let profile = LeftPanelViewModel::from_state(&state, now()).agents;
        let selected: Vec<bool> =
            profile.profiles.iter().map(|row| row.is_selected).collect();
        let can_add = profile.can_add;

        let _ = reduce(&mut state, AgentsIntent::SelectAllProfiles);
        let all = LeftPanelViewModel::from_state(&state, now());

        assert_eq!(selected, [false, true]);
        assert!(can_add);
        assert_eq!(all.agents.all.map(|row| row.is_selected), Some(true));
        assert!(all.agents.profiles.iter().all(|row| !row.is_selected));
    }

    #[test]
    fn before_the_first_read_the_sections_say_so_and_can_not_add() {
        let mut state = crate::state::AgentsState::new(
            agents(),
            crate::testing::registry(),
            env(),
        );
        let _ = reduce(&mut state, AgentsIntent::Enter);

        let vm = LeftPanelViewModel::from_state(&state, now());

        assert!(vm.agents.profiles.is_empty());
        assert!(!vm.agents.can_add);
        assert!(!vm.accounts.can_add);
        assert_eq!(vm.agents.all, None);
        assert_eq!(vm.agents.note, Some(i18n::t(Key::AgentsLoading)));
        assert_eq!(vm.accounts.all, None);
        assert!(vm.accounts.accounts.is_empty());
        assert_eq!(vm.accounts.note, Some(i18n::t(Key::AgentsLoading)));
    }

    /// Issue 24: "Loading" shows only before the first read; a Refresh
    /// keeps the sections as they are until the new answers come.
    #[test]
    fn a_refresh_keeps_the_sections_and_says_nothing_loads() {
        let read = with_profiles(&[(claude(), connected())]);
        let mut state = with_profiles(&[(claude(), connected())]);

        let _ = reduce(&mut state, AgentsIntent::Refresh);
        let while_reading = LeftPanelViewModel::from_state(&state, now());

        assert_eq!(while_reading, LeftPanelViewModel::from_state(&read, now()));
        assert_eq!(while_reading.agents.note, None);
        assert_eq!(while_reading.accounts.note, None);
    }

    #[test]
    fn a_closed_section_says_so() {
        let mut state = with_profiles(&[(claude(), connected())]);
        let _ = reduce(&mut state, AgentsIntent::ToggleAgentsSection);
        let _ = reduce(&mut state, AgentsIntent::ToggleAccountsSection);

        let vm = LeftPanelViewModel::from_state(&state, now());

        assert!(!vm.agents.is_open);
        assert!(!vm.accounts.is_open);
        assert!(vm.projects.is_open);
    }
}
