//! The Found by OTTY report (spec Decision 2): the profiles OTTY found,
//! each with its agent's version and its folder; the accounts, each with
//! its provider, name, kind and Source; and what they need from the user.
//! The profiles and the accounts are two groups that do not know about
//! each other: no count of the accounts of a profile, no "found in" link
//! from one group to the other (spec
//! 2026-11-23-accounts-apart-from-profiles, Decision 1 and "Needs").

use otty_agents::Account;
use otty_app_domain::folder_path::tilde_path;
use otty_app_i18n::{self as i18n, Key};

use super::super::profile_state::ProfileState;
use super::super::state::AgentsState;
use super::UNKNOWN;
use super::account_page::kind_text;
use super::needs::{NeedViewModel, account_needs, profile_needs};
use super::source::source_text;

/// The Found by OTTY report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FoundReportViewModel {
    /// "Found by OTTY".
    pub(crate) title: &'static str,
    /// How many profiles and accounts were found, and that OTTY only read.
    pub(crate) intro: String,
    /// One row for each profile.
    pub(crate) profiles: Vec<FoundProfileRowViewModel>,
    /// One row for each account.
    pub(crate) accounts: Vec<FoundAccountRowViewModel>,
    /// What the accounts need from the user.
    pub(crate) account_needs: Vec<NeedViewModel>,
    /// What the profiles need from the user.
    pub(crate) profile_needs: Vec<NeedViewModel>,
}

impl FoundReportViewModel {
    /// Build the report; `None` while it is closed.
    pub(crate) fn from_state(state: &AgentsState) -> Option<Self> {
        if !state.is_found_report_open() {
            return None;
        }

        let shown = state.profiles().shown();
        let accounts = state.profiles().accounts();

        let profiles: Vec<FoundProfileRowViewModel> = shown
            .iter()
            .map(|profile| FoundProfileRowViewModel::of(state, profile))
            .collect();
        let account_rows: Vec<FoundAccountRowViewModel> = accounts
            .iter()
            .map(|account| FoundAccountRowViewModel::of(state, account))
            .collect();

        let intro = i18n::filled(
            Key::TplAgentsFoundIntro,
            &[
                ("profiles", &profiles.len().to_string()),
                ("accounts", &account_rows.len().to_string()),
            ],
        );

        Some(Self {
            title: i18n::t(Key::AgentsFoundTitle),
            intro,
            profiles,
            accounts: account_rows,
            account_needs: account_needs(state),
            profile_needs: profile_needs(state),
        })
    }
}

/// One profile of the report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FoundProfileRowViewModel {
    /// The name of its agent, such as "Coder".
    pub(crate) agent: String,
    /// The name of the profile, such as "Work".
    pub(crate) profile: String,
    /// The version of the agent's program, or "—".
    pub(crate) version: String,
    /// The config folder, from `~` when it is in the home folder.
    pub(crate) folder: String,
}

impl FoundProfileRowViewModel {
    fn of(state: &AgentsState, shown: &ProfileState) -> Self {
        let profile = shown.profile();

        Self {
            agent: state.agent_name(profile.agent()).to_string(),
            profile: profile.name().to_string(),
            version: state
                .profiles()
                .version_of(shown)
                .unwrap_or(UNKNOWN)
                .to_string(),
            folder: tilde_path(profile.config_dir(), state.env().home()),
        }
    }
}

/// One account of the report, as Account info of its page writes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FoundAccountRowViewModel {
    /// The name of its provider, such as "Example Cloud".
    pub(crate) provider: String,
    /// The name its provider part gives, such as "you@example.com".
    pub(crate) name: String,
    /// Its kind in words, such as "Subscription".
    pub(crate) kind: &'static str,
    /// Where its provider part found it, such as "Found in ~/.coder",
    /// one place after another.
    pub(crate) source: String,
}

impl FoundAccountRowViewModel {
    fn of(state: &AgentsState, account: &Account) -> Self {
        let home = state.env().home();
        let source: Vec<String> = account
            .found_in()
            .iter()
            .map(|place| source_text(place, home))
            .collect();

        Self {
            provider: state.account_provider_name(account).to_string(),
            name: account.name().to_string(),
            kind: kind_text(account.kind()),
            source: source.join(", "),
        }
    }
}

#[cfg(test)]
mod tests {
    use otty_agents::{AccountKind, SavedProfiles};

    use super::*;
    use crate::event::AgentsIntent;
    use crate::reducer::reduce;
    use crate::testing::{
        agents, chatgpt_login_in, claude, codex, codex_work, connected, env,
        organization, profiles_with_accounts_loaded, with_accounts,
        with_versions,
    };

    fn report(state: &AgentsState) -> FoundReportViewModel {
        FoundReportViewModel::from_state(state).expect("the report is open")
    }

    /// A made-up key at OpenAI the OpenAI part named by its ending.
    fn key_account() -> Account {
        Account::new(
            "openai",
            otty_agents::ProviderId::new("openai"),
            "key:made-up-a1f3",
            "key ••••a1f3",
            AccountKind::Key,
            vec![crate::testing::found_in(
                crate::testing::codex_id(),
                "/home/user/.codex",
            )],
        )
    }

    /// Spec 2026-11-23-accounts-apart-from-profiles, "Needs": the
    /// profiles are a group of their own, each with its agent, version and
    /// folder, and with nothing of the accounts found in it.
    #[test]
    fn each_profile_shows_its_agent_version_and_folder() {
        let mut state =
            AgentsState::new(agents(), crate::testing::registry(), env());
        let _ = reduce(&mut state, AgentsIntent::Enter);
        let loaded = profiles_with_accounts_loaded(
            &[
                (claude(), connected(), Vec::new()),
                (
                    codex_work(),
                    connected(),
                    vec![chatgpt_login_in("/home/user/.codex-work")],
                ),
            ],
            SavedProfiles::default(),
        );
        let _ =
            reduce(&mut state, with_versions(loaded, &[Some("2.1.328"), None]));

        let vm = report(&state);

        assert_eq!(
            vm.profiles,
            vec![
                FoundProfileRowViewModel {
                    agent: String::from("Claude Code"),
                    profile: String::from("Claude Code"),
                    version: String::from("2.1.328"),
                    folder: String::from("~/.claude"),
                },
                FoundProfileRowViewModel {
                    agent: String::from("Codex"),
                    profile: String::from("Codex · Work"),
                    version: String::from(UNKNOWN),
                    folder: String::from("~/.codex-work"),
                },
            ]
        );
        assert_eq!(vm.title, i18n::t(Key::AgentsFoundTitle));
        assert_eq!(
            vm.intro,
            i18n::filled(
                Key::TplAgentsFoundIntro,
                &[("profiles", "2"), ("accounts", "1")]
            )
        );
    }

    /// The accounts are a group of their own, each with its provider,
    /// name, kind and Source, as its page writes them. One account found
    /// in two profiles is one row, and no row links to a profile.
    #[test]
    fn an_account_shows_its_provider_name_kind_and_source() {
        let state = with_accounts(
            &[
                (
                    codex(),
                    connected(),
                    vec![chatgpt_login_in("/home/user/.codex")],
                ),
                (
                    codex_work(),
                    connected(),
                    vec![chatgpt_login_in("/home/user/.codex-work")],
                ),
            ],
            SavedProfiles::default(),
        );

        let vm = report(&state);

        let source = |folder: &str| {
            i18n::filled(Key::TplAgentsSourceFoundIn, &[("folder", folder)])
        };
        assert_eq!(
            vm.accounts,
            vec![FoundAccountRowViewModel {
                provider: String::from("OpenAI"),
                name: String::from("you@example.com"),
                kind: i18n::t(Key::AgentsKindSubscription),
                source: format!(
                    "{}, {}",
                    source("~/.codex"),
                    source("~/.codex-work")
                ),
            }]
        );
    }

    #[test]
    fn a_key_is_shown_by_the_name_its_part_gives() {
        let state = with_accounts(
            &[(codex(), connected(), vec![key_account()])],
            SavedProfiles::default(),
        );

        let vm = report(&state);

        assert_eq!(
            vm.accounts,
            vec![FoundAccountRowViewModel {
                provider: String::from("OpenAI"),
                name: String::from("key ••••a1f3"),
                kind: i18n::t(Key::AgentsKindKey),
                source: i18n::filled(
                    Key::TplAgentsSourceFoundIn,
                    &[("folder", "~/.codex")]
                ),
            }]
        );
    }

    /// The report lists the needs of the accounts and of the profiles
    /// apart, as the All accounts and All profiles pages do.
    #[test]
    fn the_report_lists_the_needs_of_the_accounts_and_of_the_profiles_apart() {
        let mut state = with_accounts(
            &[
                (claude(), connected(), vec![organization()]),
                (codex(), crate::testing::not_logged_in(&codex()), Vec::new()),
            ],
            crate::testing::saved_with_given_key(),
        );
        let _ = reduce(
            &mut state,
            crate::testing::bill_loaded(Err(crate::testing::bill_key_needed())),
        );

        let vm = report(&state);

        assert_eq!(vm.account_needs, account_needs(&state));
        assert_eq!(vm.profile_needs, profile_needs(&state));
        assert_eq!(vm.account_needs.len(), 1);
        assert_eq!(vm.profile_needs.len(), 1);
    }

    #[test]
    fn a_closed_report_has_no_view_model() {
        let mut state = with_accounts(
            &[(claude(), connected(), Vec::new())],
            SavedProfiles::default(),
        );
        let _ = reduce(&mut state, AgentsIntent::CloseFoundReport);

        assert_eq!(FoundReportViewModel::from_state(&state), None);
    }
}
