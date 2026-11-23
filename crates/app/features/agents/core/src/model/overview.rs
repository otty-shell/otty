//! The All accounts and All profiles pages (spec 2026-11-19, Decision 6):
//! a table of every account or profile, and under it what they need from
//! the user, the accounts' needs on the accounts' page and the profiles'
//! needs on the profiles' page.

use otty_agents::{Account, Profile};
use otty_ui_kit::components::session_row::SessionTone;

use super::super::profile_state::ProfileState;
use super::super::state::AgentsState;
use super::UNKNOWN;
use super::account_name::account_name;
use super::needs::{NeedViewModel, account_needs, profile_needs};
use super::short_number::{
    AccountNumber, account_number, hidden_word, profile_number,
};

/// The All accounts page: the table Account, Provider, Now and the needs
/// of the accounts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AllAccountsViewModel {
    /// One row for each account, in the order of the left panel.
    pub(crate) rows: Vec<AccountTableRowViewModel>,
    /// What the accounts need from the user.
    pub(crate) needs: Vec<NeedViewModel>,
}

impl AllAccountsViewModel {
    /// Build the page for the current state.
    pub(crate) fn from_state(state: &AgentsState) -> Self {
        let rows = state
            .listed_accounts()
            .into_iter()
            .map(|account| AccountTableRowViewModel::of(state, account))
            .collect();

        Self {
            rows,
            needs: account_needs(state),
        }
    }
}

/// One account of the All accounts table; a press opens it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AccountTableRowViewModel {
    /// The account, sent when the row is pressed.
    pub(crate) account: Account,
    /// The provider and the login, such as "Example Cloud · you@example.com".
    pub(crate) name: String,
    /// The name of the provider, such as "Example Cloud".
    pub(crate) provider: String,
    /// Its number with its word and color, such as "42 % used", or "—",
    /// which draws itself in the tight form; a state that needs the user
    /// is a badge.
    pub(crate) now: AccountNumber,
    /// Whether the user hid the account: Now is a "hidden" badge.
    pub(crate) is_hidden: bool,
}

impl AccountTableRowViewModel {
    fn of(state: &AgentsState, account: Account) -> Self {
        let now = account_number(state, &account).unwrap_or_else(|| {
            AccountNumber::word(UNKNOWN.to_string(), SessionTone::Muted)
        });

        Self {
            name: account_name(
                state.registry(),
                state.profiles().saved(),
                &account,
            ),
            provider: state.account_provider_name(&account).to_string(),
            now,
            is_hidden: state.profiles().is_account_hidden(&account),
            account,
        }
    }
}

/// The All profiles page: the table Profile, Agent, Version, Today and the
/// needs of the profiles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AllProfilesViewModel {
    /// One row for each profile, in the order of the left panel.
    pub(crate) rows: Vec<ProfileTableRowViewModel>,
    /// What the profiles need from the user.
    pub(crate) needs: Vec<NeedViewModel>,
}

impl AllProfilesViewModel {
    /// Build the page for the current state.
    pub(crate) fn from_state(state: &AgentsState) -> Self {
        let shown = state
            .profiles()
            .shown()
            .iter()
            .map(|shown| ProfileTableRowViewModel::of(state, shown));
        let hidden = state
            .listed_hidden_profiles()
            .iter()
            .map(|profile| ProfileTableRowViewModel::hidden(state, profile));
        let rows = shown.chain(hidden).collect();

        Self {
            rows,
            needs: profile_needs(state),
        }
    }
}

/// One profile of the All profiles table; a press opens its dashboard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProfileTableRowViewModel {
    /// The profile, sent when the row is pressed.
    pub(crate) profile: Profile,
    /// The profile name.
    pub(crate) name: String,
    /// The name of its agent, such as "Coder".
    pub(crate) agent: String,
    /// The version of the agent's program, or "—".
    pub(crate) version: String,
    /// The tokens it spent today with their word, the state that needs
    /// the user, or "—".
    pub(crate) today: String,
    /// The color of the number: a state that needs the user is a badge.
    pub(crate) tone: SessionTone,
    /// Whether the user hid the profile: Today is a "hidden" badge.
    pub(crate) is_hidden: bool,
}

impl ProfileTableRowViewModel {
    fn of(state: &AgentsState, shown: &ProfileState) -> Self {
        let profile = shown.profile();
        let version = state.profiles().version_of(shown).unwrap_or(UNKNOWN);
        let (today, tone) = profile_number(shown)
            .unwrap_or_else(|| (UNKNOWN.to_string(), SessionTone::Muted));

        Self {
            profile: profile.clone(),
            name: profile.name().to_string(),
            agent: state.agent_name(profile.agent()).to_string(),
            version: version.to_string(),
            today,
            tone,
            is_hidden: false,
        }
    }

    /// The row of the hidden `profile`: nothing of it is read, so its
    /// version is not known and Today is "hidden".
    fn hidden(state: &AgentsState, profile: &Profile) -> Self {
        Self {
            profile: profile.clone(),
            name: profile.name().to_string(),
            agent: state.agent_name(profile.agent()).to_string(),
            version: UNKNOWN.to_string(),
            today: hidden_word(),
            tone: SessionTone::Muted,
            is_hidden: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use otty_agents::SavedProfiles;
    use otty_app_i18n::{self as i18n, Key};

    use super::*;
    use crate::event::AgentsIntent;
    use crate::model::UNKNOWN;
    use crate::need::NeedAction;
    use crate::reducer::reduce;
    use crate::testing::{
        agents, api_key, at, chatgpt_login_in, claude, codex, codex_work,
        connected, env, five_hour_limits, limits_answer, limits_loaded,
        organization, profiles_with_accounts_loaded, spend_loaded, spend_of,
        with_versions,
    };
    use crate::types::SpendCount;

    /// Claude Code paying through the [`organization`] with no admin key,
    /// Codex logged in to the [`chatgpt_login`] with 42 % used, and Codex ·
    /// Work not logged in; Claude Code 2.1.328 spent 1.8M tokens today.
    fn found() -> AgentsState {
        let mut state =
            AgentsState::new(agents(), crate::testing::registry(), env());
        let _ = reduce(&mut state, AgentsIntent::Enter);
        let loaded = profiles_with_accounts_loaded(
            &[
                (claude(), api_key(), vec![organization()]),
                (
                    codex(),
                    connected(),
                    vec![chatgpt_login_in("/home/user/.codex")],
                ),
                (
                    codex_work(),
                    crate::testing::not_logged_in(&codex_work()),
                    Vec::new(),
                ),
            ],
            SavedProfiles::default(),
        );
        let _ = reduce(
            &mut state,
            with_versions(loaded, &[Some("2.1.328"), None, None]),
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
        let _ = reduce(
            &mut state,
            spend_loaded(
                &claude(),
                SpendCount::Counted(spend_of(1_800_000, 1)),
                at(10),
            ),
        );

        state
    }

    #[test]
    fn all_accounts_lists_each_account_with_its_provider_and_number() {
        let state = found();

        let vm = AllAccountsViewModel::from_state(&state);

        assert_eq!(
            vm.rows,
            vec![
                AccountTableRowViewModel {
                    account: organization(),
                    name: String::from("Anthropic · dev@example.com"),
                    provider: String::from("Anthropic"),
                    now: AccountNumber::word(
                        String::from(UNKNOWN),
                        SessionTone::Muted
                    ),
                    is_hidden: false,
                },
                AccountTableRowViewModel {
                    account: chatgpt_login_in("/home/user/.codex"),
                    name: String::from("OpenAI · you@example.com"),
                    provider: String::from("OpenAI"),
                    now: AccountNumber::word(
                        i18n::with_count(Key::TplAgentsPercentUsed, 42),
                        SessionTone::Muted
                    ),
                    is_hidden: false,
                },
            ]
        );
    }

    /// Spec 2026-11-23-accounts-apart-from-profiles, Decision 1: two
    /// plugins find one ChatGPT login; each is an account row of its own,
    /// so the user can hide the one they do not want.
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
        )
        .with_hosts(["openai.com"]);
        let registry = crate::testing::registry();
        let mut providers = registry.providers().to_vec();
        providers.push(fork);
        let registry = std::sync::Arc::new(otty_agents::Registry::new(
            registry.agents().to_vec(),
            providers,
        ));
        let of_fork = otty_agents::Account::new(
            "openai-fork",
            otty_agents::ProviderId::new("openai"),
            "chatgpt:made-up-account",
            "you@example.com",
            otty_agents::AccountKind::Subscription,
            Vec::new(),
        );
        let state = crate::testing::with_accounts_in(
            registry,
            &[(
                codex(),
                connected(),
                vec![crate::testing::chatgpt_login(), of_fork],
            )],
            SavedProfiles::default(),
        );

        let vm = AllAccountsViewModel::from_state(&state);

        let rows: Vec<(&str, &str)> = vm
            .rows
            .iter()
            .map(|row| (row.account.plugin(), row.name.as_str()))
            .collect();
        assert_eq!(
            rows,
            [
                ("openai", "OpenAI · you@example.com"),
                ("openai-fork", "OpenAI · you@example.com"),
            ]
        );
    }

    /// Spec Decision 6: All accounts lists the needs of the accounts only.
    #[test]
    fn all_accounts_lists_only_the_needs_of_the_accounts() {
        let mut state = found();
        let _ = reduce(
            &mut state,
            AgentsIntent::BillLoaded(crate::types::LoadedBill {
                profile: claude(),
                given: Vec::new(),
                bill: Err(crate::testing::bill_key_needed()),
                read_at: crate::testing::now(),
            }),
        );

        let vm = AllAccountsViewModel::from_state(&state);

        let actions: Vec<_> = vm
            .needs
            .iter()
            .filter_map(|need| need.fix.as_ref())
            .filter_map(|fix| fix.action.clone())
            .collect();
        assert_eq!(
            actions,
            vec![NeedAction::AskKey {
                account: organization(),
                plugin: String::from("Anthropic"),
                ask: crate::testing::key_ask(),
            }]
        );
        assert_eq!(vm.needs.len(), 1);
    }

    #[test]
    fn all_profiles_lists_each_profile_with_its_agent_version_and_today() {
        let state = found();

        let vm = AllProfilesViewModel::from_state(&state);

        assert_eq!(
            vm.rows,
            vec![
                ProfileTableRowViewModel {
                    profile: claude(),
                    name: String::from("Claude Code"),
                    agent: String::from("Claude Code"),
                    version: String::from("2.1.328"),
                    today: i18n::filled(
                        Key::TplAgentsAmountToday,
                        &[("amount", "1.8M")]
                    ),
                    tone: SessionTone::Muted,
                    is_hidden: false,
                },
                ProfileTableRowViewModel {
                    profile: codex(),
                    name: String::from("Codex"),
                    agent: String::from("Codex"),
                    version: String::from(UNKNOWN),
                    today: String::from(UNKNOWN),
                    tone: SessionTone::Muted,
                    is_hidden: false,
                },
                ProfileTableRowViewModel {
                    profile: codex_work(),
                    name: String::from("Codex · Work"),
                    agent: String::from("Codex"),
                    version: String::from(UNKNOWN),
                    today: i18n::t(Key::AgentsStateNeedsLogin).to_string(),
                    tone: SessionTone::Failed,
                    is_hidden: false,
                },
            ]
        );
    }

    /// Spec Decision 6: All profiles lists the needs of the profiles only.
    #[test]
    fn all_profiles_lists_only_the_needs_of_the_profiles() {
        let state = found();

        let vm = AllProfilesViewModel::from_state(&state);

        let places: Vec<&str> =
            vm.needs.iter().map(|need| need.place.as_str()).collect();
        assert_eq!(places, ["Codex · Work"]);
    }
}
