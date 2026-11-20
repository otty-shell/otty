//! The Found by OTTY report (spec Decision 2): the profiles OTTY found,
//! each with its agent's version, its folder and how many accounts were
//! found in it; the accounts, one row each, with where they were found,
//! what OTTY reads of them and their number now; and what they need from
//! the user, the accounts apart from the profiles.

use otty_agents::{Account, AccountKind};
use otty_app_domain::folder_path::tilde_path;
use otty_app_i18n::{self as i18n, Key};

use super::super::profile_state::ProfileState;
use super::super::state::AgentsState;
use super::UNKNOWN;
use super::account_name::account_name;
use super::found_in::found_in_text;
use super::needs::{NeedViewModel, account_needs, profile_needs};
use super::short_number::account_number;

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
    /// The name of its agent, such as "Claude Code".
    pub(crate) agent: String,
    /// The name of the profile, such as "Work".
    pub(crate) profile: String,
    /// The version of the agent's program, or "—".
    pub(crate) version: String,
    /// The config folder, from `~` when it is in the home folder.
    pub(crate) folder: String,
    /// How many accounts were found in it.
    pub(crate) accounts: String,
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
            accounts: shown.accounts().len().to_string(),
        }
    }
}

/// One account of the report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FoundAccountRowViewModel {
    /// The provider and the login, such as "OpenAI · you@example.com".
    pub(crate) account: String,
    /// Every place it was found, such as "~/.codex, ~/.codex-work".
    pub(crate) found_in: String,
    /// What OTTY reads of it: limits, credits or money.
    pub(crate) reads: &'static str,
    /// Its number now with its word, as its row in the left panel shows
    /// it, or "—".
    pub(crate) now: String,
}

impl FoundAccountRowViewModel {
    fn of(state: &AgentsState, account: &Account) -> Self {
        let home = state.env().home();
        let found_in: Vec<String> = account
            .found_in()
            .iter()
            .map(|place| found_in_text(place, home))
            .collect();
        let now = account_number(state, account)
            .map_or_else(|| UNKNOWN.to_string(), |(number, _)| number);

        Self {
            account: account_name(
                state.registry(),
                state.profiles().saved(),
                account,
            ),
            found_in: found_in.join(", "),
            reads: reads_of(account.kind()),
            now,
        }
    }
}

/// What OTTY reads of an account of `kind`.
fn reads_of(kind: AccountKind) -> &'static str {
    match kind {
        AccountKind::Limits => i18n::t(Key::AgentsReadsLimits),
        AccountKind::Credits => i18n::t(Key::AgentsReadsCredits),
        _ => i18n::t(Key::AgentsReadsMoney),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use otty_agents::{
        AccountIdentity, AccountLogin, FoundIn, KeyFingerprint, ProfileStatus,
        ProviderRef, SavedProfiles, SourceId,
    };

    use super::*;
    use crate::event::AgentsIntent;
    use crate::reducer::reduce;
    use crate::testing::{
        agents, api_key, at, chatgpt_login_in, claude, codex, codex_work,
        connected, env, five_hour_limits, limits_answer, limits_loaded,
        organization, profiles_with_accounts_loaded, with_accounts,
        with_versions,
    };

    fn report(state: &AgentsState) -> FoundReportViewModel {
        FoundReportViewModel::from_state(state).expect("the report is open")
    }

    /// A made-up key at OpenAI that ends with `a1f3`.
    fn key_account() -> Account {
        Account::new(
            ProviderRef::of_id("openai"),
            AccountIdentity::KeyFingerprint(KeyFingerprint::of(
                "sk-made-up-key-for-tests-a1f3",
            )),
            AccountLogin::Key(Some(String::from("a1f3"))),
            AccountKind::Money,
            FoundIn::ProfileFolder(PathBuf::from("/home/user/.codex")),
        )
    }

    #[test]
    fn each_profile_shows_its_agent_version_folder_and_accounts() {
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
                    accounts: String::from("0"),
                },
                FoundProfileRowViewModel {
                    agent: String::from("Codex"),
                    profile: String::from("Codex · Work"),
                    version: String::from(UNKNOWN),
                    folder: String::from("~/.codex-work"),
                    accounts: String::from("1"),
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

    /// Spec Decision 2: one account, one row, however many profiles it
    /// was found in; its number is the one of its first profile.
    #[test]
    fn an_account_found_in_two_profiles_is_one_row_with_both_folders() {
        let mut state = with_accounts(
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
        let _ = reduce(
            &mut state,
            limits_loaded(
                &codex(),
                limits_answer(vec![(
                    SourceId::ChatGptUsageApi,
                    Ok(five_hour_limits(42.0)),
                )]),
                at(10),
            ),
        );

        let vm = report(&state);

        assert_eq!(
            vm.accounts,
            vec![FoundAccountRowViewModel {
                account: String::from("OpenAI · you@example.com"),
                found_in: String::from("~/.codex, ~/.codex-work"),
                reads: i18n::t(Key::AgentsReadsLimits),
                now: i18n::with_count(Key::TplAgentsPercentUsed, 42),
            }]
        );
    }

    #[test]
    fn a_key_is_shown_by_its_last_characters_and_nothing_known_is_a_dash() {
        let state = with_accounts(
            &[(codex(), api_key(), vec![key_account()])],
            SavedProfiles::default(),
        );

        let vm = report(&state);

        assert_eq!(
            vm.accounts,
            vec![FoundAccountRowViewModel {
                account: format!(
                    "OpenAI · {}",
                    i18n::filled(Key::TplAgentsAccountKey, &[("last", "a1f3")])
                ),
                found_in: String::from("~/.codex"),
                reads: i18n::t(Key::AgentsReadsMoney),
                now: String::from(UNKNOWN),
            }]
        );
    }

    /// The report lists the needs of the accounts and of the profiles
    /// apart, as the All accounts and All profiles pages do.
    #[test]
    fn the_report_lists_the_needs_of_the_accounts_and_of_the_profiles_apart() {
        let state = with_accounts(
            &[
                (claude(), api_key(), vec![organization()]),
                (codex(), ProfileStatus::NotLoggedIn, Vec::new()),
            ],
            SavedProfiles::default(),
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
