//! The Found by OTTY report (spec Decision 2): the profiles OTTY found,
//! each with its agent's version, its folder and how many accounts were
//! found in it; the accounts, one row each, with where they were found,
//! what OTTY reads of them and their number now; and what they need from
//! the user, the accounts apart from the profiles.

use otty_agents::{
    Account, AccountKind, AccountLogin, FoundIn, merge_accounts,
};
use otty_app_domain::folder_path::tilde_path;
use otty_app_i18n::{self as i18n, Key};

use super::super::found_report::NeedAction;
use super::super::profile_state::ProfileState;
use super::super::state::AgentsState;
use super::UNKNOWN;
use super::short_number::short_number;
use super::user_need::UserNeed;

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
        let found: Vec<Account> = shown
            .iter()
            .flat_map(|profile| profile.accounts().iter().cloned())
            .collect();
        let accounts = merge_accounts(found);

        let profiles: Vec<FoundProfileRowViewModel> = shown
            .iter()
            .map(|profile| FoundProfileRowViewModel::of(state, profile))
            .collect();
        let account_rows: Vec<FoundAccountRowViewModel> = accounts
            .iter()
            .map(|account| FoundAccountRowViewModel::of(state, account))
            .collect();
        let account_needs = accounts
            .iter()
            .filter_map(|account| admin_key_need(state, account))
            .collect();
        let profile_needs = shown
            .iter()
            .filter_map(|profile| profile_need(state, profile))
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
            account_needs,
            profile_needs,
        })
    }
}

/// One profile of the report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FoundProfileRowViewModel {
    /// The name of its agent, such as "Claude Code".
    pub(crate) agent: &'static str,
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
            agent: state.agent_name(profile.agent()),
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
    /// Its number now, as the row of its first profile shows it, or "—".
    pub(crate) now: String,
}

impl FoundAccountRowViewModel {
    fn of(state: &AgentsState, account: &Account) -> Self {
        let home = state.env().home();
        let found_in: Vec<String> = account
            .found_in()
            .iter()
            .map(|place| match place {
                FoundIn::ProfileFolder(folder) => tilde_path(folder, home),
                FoundIn::SystemKeyring => {
                    i18n::t(Key::AgentsFoundInKeyring).to_string()
                },
                _ => i18n::t(Key::AgentsFoundInUser).to_string(),
            })
            .collect();
        let now = first_profile_of(state, account)
            .and_then(short_number)
            .map_or_else(|| UNKNOWN.to_string(), |(number, _)| number);

        Self {
            account: account_name(account),
            found_in: found_in.join(", "),
            reads: reads_of(account.kind()),
            now,
        }
    }
}

/// One thing the user is asked to do: where, what, why, and its one
/// button when the report can do it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NeedViewModel {
    /// The account or the profile it is about.
    pub(crate) place: String,
    /// What is needed, in a few words.
    pub(crate) title: String,
    /// Why, and what the button does.
    pub(crate) text: String,
    /// The button, when there is one.
    pub(crate) fix: Option<NeedFixViewModel>,
}

/// The one button of a need.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NeedFixViewModel {
    /// The label of the button, such as "Log in".
    pub(crate) label: &'static str,
    /// What it does.
    pub(crate) action: NeedAction,
}

/// The provider and the login of `account`, such as "OpenAI ·
/// you@example.com" or "Anthropic · key ••••a1f3"; a login with no email
/// shows its plan when the files name one.
fn account_name(account: &Account) -> String {
    let login = match account.login() {
        AccountLogin::Email(email) => email.clone(),
        AccountLogin::Key(Some(last)) => {
            i18n::filled(Key::TplAgentsAccountKey, &[("last", last)])
        },
        AccountLogin::Key(None) => i18n::t(Key::AgentsAccountKey).to_string(),
        _ => account
            .plan()
            .unwrap_or(i18n::t(Key::AgentsAccountLogin))
            .to_string(),
    };

    format!("{} · {login}", account.provider().name())
}

/// What OTTY reads of an account of `kind`.
fn reads_of(kind: AccountKind) -> &'static str {
    match kind {
        AccountKind::Limits => i18n::t(Key::AgentsReadsLimits),
        AccountKind::Credits => i18n::t(Key::AgentsReadsCredits),
        _ => i18n::t(Key::AgentsReadsMoney),
    }
}

/// The first shown profile `account` was found in.
fn first_profile_of<'a>(
    state: &'a AgentsState,
    account: &Account,
) -> Option<&'a ProfileState> {
    state.profiles().shown().iter().find(|shown| {
        shown
            .accounts()
            .iter()
            .any(|found| found.is_same_account(account))
    })
}

/// The need of an organization whose bill OTTY could read but has no admin
/// key for: Add admin key opens the profile dialog of its first profile,
/// which has the field.
fn admin_key_need(
    state: &AgentsState,
    account: &Account,
) -> Option<NeedViewModel> {
    account.organization_id()?;
    let shown = first_profile_of(state, account)?;
    let profiles = state.profiles();
    let profile = shown.profile();

    let can_add = shown.is_api_key() && profiles.has_bill(profile.agent());
    if !can_add || profiles.admin_key_of(shown).is_some() {
        return None;
    }

    Some(NeedViewModel {
        place: account_name(account),
        title: i18n::t(Key::AgentsNeedAdminKeyTitle).to_string(),
        text: i18n::t(Key::AgentsNeedAdminKeyText).to_string(),
        fix: Some(NeedFixViewModel {
            label: i18n::t(Key::AgentsAddAdminKey),
            action: NeedAction::AddAdminKey(profile.clone()),
        }),
    })
}

/// The need of a profile that can not be used, with Log in while it can
/// log in.
fn profile_need(
    state: &AgentsState,
    shown: &ProfileState,
) -> Option<NeedViewModel> {
    let need = UserNeed::of(shown.status())?;
    let profile = shown.profile();

    let fix = state
        .profiles()
        .can_log_in(profile)
        .then(|| NeedFixViewModel {
            label: i18n::t(Key::AgentsLogIn),
            action: NeedAction::LogIn(profile.clone()),
        });

    Some(NeedViewModel {
        place: profile.name().to_string(),
        title: need.state().to_string(),
        text: need.hint().to_string(),
        fix,
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use otty_agents::{
        AccountIdentity, KeyFingerprint, ProfileStatus, Provider,
        SavedProfiles, SourceId,
    };

    use super::*;
    use crate::event::AgentsIntent;
    use crate::reducer::reduce;
    use crate::testing::{
        agents, api_key, at, chatgpt_login, claude, codex, codex_work,
        connected, env, five_hour_limits, limits_answer, limits_loaded,
        organization, profiles_with_accounts_loaded, saved_with_admin_key,
        with_accounts, with_versions,
    };

    fn report(state: &AgentsState) -> FoundReportViewModel {
        FoundReportViewModel::from_state(state).expect("the report is open")
    }

    /// A made-up key at OpenAI that ends with `a1f3`.
    fn key_account() -> Account {
        Account::new(
            Provider::OpenAI,
            AccountIdentity::KeyFingerprint(KeyFingerprint::of(
                "sk-made-up-key-for-tests-a1f3",
            )),
            AccountLogin::Key(Some(String::from("a1f3"))),
            AccountKind::Money,
            FoundIn::ProfileFolder(PathBuf::from("/home/user/.codex")),
        )
    }

    fn chatgpt_in(folder: &str) -> Account {
        let login = chatgpt_login();
        Account::new(
            login.provider(),
            login.identity().clone(),
            login.login().clone(),
            login.kind(),
            FoundIn::ProfileFolder(PathBuf::from(folder)),
        )
    }

    #[test]
    fn each_profile_shows_its_agent_version_folder_and_accounts() {
        let mut state = AgentsState::new(agents(), env());
        let _ = reduce(&mut state, AgentsIntent::Enter);
        let loaded = profiles_with_accounts_loaded(
            &[
                (claude(), connected(), Vec::new()),
                (
                    codex_work(),
                    connected(),
                    vec![chatgpt_in("/home/user/.codex-work")],
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
                    agent: "Claude Code",
                    profile: String::from("Claude Code"),
                    version: String::from("2.1.328"),
                    folder: String::from("~/.claude"),
                    accounts: String::from("0"),
                },
                FoundProfileRowViewModel {
                    agent: "Codex",
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
                (codex(), connected(), vec![chatgpt_in("/home/user/.codex")]),
                (
                    codex_work(),
                    connected(),
                    vec![chatgpt_in("/home/user/.codex-work")],
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
                now: i18n::with_count(Key::TplAgentsMostUsed, 42),
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

    #[test]
    fn the_needs_of_the_accounts_and_of_the_profiles_are_apart() {
        let state = with_accounts(
            &[
                (claude(), api_key(), vec![organization()]),
                (codex(), ProfileStatus::NotLoggedIn, Vec::new()),
                (codex_work(), ProfileStatus::ProgramNotFound, Vec::new()),
            ],
            SavedProfiles::default(),
        );

        let vm = report(&state);

        assert_eq!(
            vm.account_needs,
            vec![NeedViewModel {
                place: String::from("Anthropic · dev@example.com"),
                title: i18n::t(Key::AgentsNeedAdminKeyTitle).to_string(),
                text: i18n::t(Key::AgentsNeedAdminKeyText).to_string(),
                fix: Some(NeedFixViewModel {
                    label: i18n::t(Key::AgentsAddAdminKey),
                    action: NeedAction::AddAdminKey(claude()),
                }),
            }]
        );
        assert_eq!(
            vm.profile_needs,
            vec![
                NeedViewModel {
                    place: String::from("Codex"),
                    title: i18n::t(Key::AgentsStateNeedsLogin).to_string(),
                    text: i18n::t(Key::AgentsNotLoggedInHint).to_string(),
                    fix: Some(NeedFixViewModel {
                        label: i18n::t(Key::AgentsLogIn),
                        action: NeedAction::LogIn(codex()),
                    }),
                },
                NeedViewModel {
                    place: String::from("Codex · Work"),
                    title: i18n::t(Key::AgentsStateProgramNotFound).to_string(),
                    text: i18n::t(Key::AgentsProgramNotFoundHint).to_string(),
                    fix: None,
                },
            ]
        );
    }

    #[test]
    fn an_organization_with_an_admin_key_needs_nothing() {
        let state = with_accounts(
            &[(claude(), api_key(), vec![organization()])],
            saved_with_admin_key(),
        );

        let vm = report(&state);

        assert!(vm.account_needs.is_empty());
        assert!(vm.profile_needs.is_empty());
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
