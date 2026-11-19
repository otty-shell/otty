//! What the user is asked to do (spec "Words" and Decision 6): each need
//! is a title, a line and one action, collected per account and per
//! profile and never mixed. The Found by OTTY report, the All accounts
//! and All profiles pages and the page of an account list them.

use otty_agents::{Account, FormatChange};
use otty_app_domain::folder_path::tilde_path;
use otty_app_i18n::{self as i18n, Key};

use super::super::need::NeedAction;
use super::super::profile_state::ProfileState;
use super::super::state::AgentsState;
use super::account_name::account_name;
use super::reset_time::date_time_text;
use super::user_need::UserNeed;

/// One thing the user is asked to do: where, what, why, and its one
/// button when OTTY can do it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NeedViewModel {
    /// The account or the profile it is about.
    pub(crate) place: String,
    /// What is needed, in a few words.
    pub(crate) title: String,
    /// Why, and what the button does.
    pub(crate) line: String,
    /// The button, when there is one.
    pub(crate) fix: Option<NeedFixViewModel>,
}

/// The one button of a need.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NeedFixViewModel {
    /// The label of the button, such as "Log in".
    pub(crate) label: String,
    /// What it does.
    pub(crate) action: NeedAction,
}

/// What the accounts need from the user, in the order of the accounts,
/// each as [`needs_of_account`] lists them.
pub(crate) fn account_needs(state: &AgentsState) -> Vec<NeedViewModel> {
    state
        .profiles()
        .accounts()
        .iter()
        .flat_map(|account| needs_of_account(state, account))
        .collect()
}

/// What `account` needs from the user: an expired login, renewed by
/// opening the agent it was found in, and an organization whose bill OTTY
/// could read with an admin key it has not got.
pub(crate) fn needs_of_account(
    state: &AgentsState,
    account: &Account,
) -> Vec<NeedViewModel> {
    login_expired_need(state, account)
        .into_iter()
        .chain(admin_key_need(state, account))
        .collect()
}

/// What the profiles need from the user, in the order of the profiles,
/// each as [`needs_of_profile`] lists them.
pub(crate) fn profile_needs(state: &AgentsState) -> Vec<NeedViewModel> {
    state
        .profiles()
        .shown()
        .iter()
        .flat_map(|shown| needs_of_profile(state, shown))
        .collect()
}

/// What the profile `shown` needs from the user: Log in or the program to
/// install, when it can not be used, and a report to copy when its agent
/// changed the format of its session files.
pub(crate) fn needs_of_profile(
    state: &AgentsState,
    shown: &ProfileState,
) -> Vec<NeedViewModel> {
    profile_need(state, shown)
        .into_iter()
        .chain(format_changed_need(state, shown))
        .collect()
}

/// The need of an organization whose bill OTTY could read but has no admin
/// key for: Add admin key opens the profile dialog of its first profile,
/// which has the field.
fn admin_key_need(
    state: &AgentsState,
    account: &Account,
) -> Option<NeedViewModel> {
    account.organization_id()?;
    let profiles = state.profiles();
    let shown = profiles.first_with(account)?;
    let profile = shown.profile();

    let can_add = shown.is_api_key() && profiles.has_bill(profile.agent());
    if !can_add || profiles.admin_key_of(shown).is_some() {
        return None;
    }

    Some(NeedViewModel {
        place: account_name(state.profiles().saved(), account),
        title: i18n::t(Key::AgentsNeedAdminKeyTitle).to_string(),
        line: i18n::t(Key::AgentsNeedAdminKeyText).to_string(),
        fix: Some(NeedFixViewModel {
            label: i18n::t(Key::AgentsAddAdminKey).to_string(),
            action: NeedAction::AddAdminKey(profile.clone()),
        }),
    })
}

/// The need of an account whose login expired at the latest read of its
/// limits (spec 2026-11-19, Decision 3): OTTY never renews it, so the
/// button opens the agent of the profile the limits are read through.
fn login_expired_need(
    state: &AgentsState,
    account: &Account,
) -> Option<NeedViewModel> {
    let profiles = state.profiles();
    let shown = profiles.with_limits_of(account)?;
    if !shown.limits().is_login_expired() {
        return None;
    }

    let profile = shown.profile();
    let agent = state.agent_name(profile.agent());
    let fix = profiles.can_start(profile).then(|| NeedFixViewModel {
        label: i18n::filled(Key::TplAgentsOpenAgent, &[("agent", agent)]),
        action: NeedAction::OpenAgent(profile.clone()),
    });

    Some(NeedViewModel {
        place: account_name(state.profiles().saved(), account),
        title: i18n::t(Key::AgentsNeedLoginExpiredTitle).to_string(),
        line: i18n::filled(
            Key::TplAgentsNeedLoginExpiredText,
            &[("agent", agent)],
        ),
        fix,
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
            label: i18n::t(Key::AgentsLogIn).to_string(),
            action: NeedAction::LogIn(profile.clone()),
        });

    Some(NeedViewModel {
        place: profile.name().to_string(),
        title: need.state().to_string(),
        line: need.hint().to_string(),
        fix,
    })
}

/// The need of a profile whose agent changed the format of its session
/// files (spec 2026-11-19, Decision 9): it names the version, says since
/// when the tokens are missing while the last good numbers stay, and
/// copies a report for the makers of OTTY.
fn format_changed_need(
    state: &AgentsState,
    shown: &ProfileState,
) -> Option<NeedViewModel> {
    let change = shown.spend().format_change()?;
    let profile = shown.profile();
    let agent = state.agent_name(profile.agent());
    let folder = tilde_path(profile.config_dir(), state.env().home());
    let tested = change.tested_version();

    let line = match shown.spend().last_good() {
        Some((_, counted_at)) => {
            let time = date_time_text(counted_at, state.profiles().time_zone());
            i18n::filled(
                Key::TplAgentsNeedFormatChangedText,
                &[("agent", agent), ("tested", tested), ("time", &time)],
            )
        },
        None => i18n::filled(
            Key::TplAgentsNeedFormatChangedNoNumbers,
            &[("agent", agent), ("tested", tested)],
        ),
    };

    Some(NeedViewModel {
        place: profile.name().to_string(),
        title: format_changed_title(agent, change),
        line,
        fix: Some(NeedFixViewModel {
            label: i18n::t(Key::AgentsCopyReport).to_string(),
            action: NeedAction::CopyReport(change.report(agent, &folder)),
        }),
    })
}

/// The title of the need of `change` of the files of `agent`, such as
/// "Claude Code 2.2.0 changed its session files".
fn format_changed_title(agent: &str, change: &FormatChange) -> String {
    match change.version() {
        Some(version) => i18n::filled(
            Key::TplAgentsNeedFormatChangedTitle,
            &[("agent", agent), ("version", version)],
        ),
        None => i18n::filled(
            Key::TplAgentsNeedFormatChangedNoVersion,
            &[("agent", agent)],
        ),
    }
}

#[cfg(test)]
mod tests {
    use otty_agents::fixtures::format_change;
    use otty_agents::{
        FormatChange, ProfileStatus, SavedProfiles, SourceError, SourceId,
    };

    use super::*;
    use crate::model::reset_time::date_time_text;
    use crate::reducer::reduce;
    use crate::testing::{
        api_key, at, chatgpt_login, claude, claude_work, codex, codex_work,
        connected, limits_answer, limits_loaded, organization,
        saved_with_admin_key, spend_loaded, spend_of, with_accounts,
        with_profiles,
    };
    use crate::types::SpendCount;

    /// Spec Decision 6: the needs of the accounts and of the profiles are
    /// collected apart and never mixed.
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

        let accounts = account_needs(&state);
        let profiles = profile_needs(&state);

        assert_eq!(
            accounts,
            vec![NeedViewModel {
                place: String::from("Anthropic · dev@example.com"),
                title: i18n::t(Key::AgentsNeedAdminKeyTitle).to_string(),
                line: i18n::t(Key::AgentsNeedAdminKeyText).to_string(),
                fix: Some(NeedFixViewModel {
                    label: i18n::t(Key::AgentsAddAdminKey).to_string(),
                    action: NeedAction::AddAdminKey(claude()),
                }),
            }]
        );
        assert_eq!(
            profiles,
            vec![
                NeedViewModel {
                    place: String::from("Codex"),
                    title: i18n::t(Key::AgentsStateNeedsLogin).to_string(),
                    line: i18n::t(Key::AgentsNotLoggedInHint).to_string(),
                    fix: Some(NeedFixViewModel {
                        label: i18n::t(Key::AgentsLogIn).to_string(),
                        action: NeedAction::LogIn(codex()),
                    }),
                },
                NeedViewModel {
                    place: String::from("Codex · Work"),
                    title: i18n::t(Key::AgentsStateProgramNotFound).to_string(),
                    line: i18n::t(Key::AgentsProgramNotFoundHint).to_string(),
                    fix: None,
                },
            ]
        );
    }

    /// Spec 2026-11-19, Decision 3: an account whose login expired asks
    /// to open the agent it is read through, which renews the login or
    /// asks for a new one; it is a need of the account, not of the
    /// profile.
    #[test]
    fn an_account_whose_login_expired_asks_to_open_its_agent() {
        let mut state = with_accounts(
            &[(codex(), connected(), vec![chatgpt_login()])],
            SavedProfiles::default(),
        );
        let _ = reduce(
            &mut state,
            limits_loaded(
                &codex(),
                limits_answer(vec![(
                    SourceId::ChatGptUsageApi,
                    Err(SourceError::NotLoggedIn),
                )]),
                at(10),
            ),
        );

        assert_eq!(
            account_needs(&state),
            vec![NeedViewModel {
                place: String::from("OpenAI · you@example.com"),
                title: i18n::t(Key::AgentsNeedLoginExpiredTitle).to_string(),
                line: i18n::filled(
                    Key::TplAgentsNeedLoginExpiredText,
                    &[("agent", "Codex")]
                ),
                fix: Some(NeedFixViewModel {
                    label: i18n::filled(
                        Key::TplAgentsOpenAgent,
                        &[("agent", "Codex")]
                    ),
                    action: NeedAction::OpenAgent(codex()),
                }),
            }]
        );
        assert!(profile_needs(&state).is_empty());
    }

    /// Claude Code, whose last good count ended at 10 and whose next count
    /// found its session files changed by `change`.
    fn with_changed_format(change: &FormatChange) -> AgentsState {
        let mut state = with_profiles(&[(claude(), connected())]);
        let error = SourceError::FormatChanged(Box::new(change.clone()));
        for (spend, counted_at) in [
            (SpendCount::Counted(spend_of(1, 2)), at(10)),
            (SpendCount::Failed(error), at(20)),
        ] {
            let _ =
                reduce(&mut state, spend_loaded(&claude(), spend, counted_at));
        }

        state
    }

    /// Spec 2026-11-19, Decision 9: a profile whose session files OTTY no
    /// longer understands names the version, says the last good numbers
    /// are kept, and copies a report.
    #[test]
    fn a_profile_whose_files_changed_asks_to_copy_a_report() {
        let change = format_change(Some("2.2.0"), 40, 0);
        let state = with_changed_format(&change);

        let needs = profile_needs(&state);

        let since = date_time_text(at(10), state.profiles().time_zone());
        assert_eq!(
            needs,
            vec![NeedViewModel {
                place: String::from("Claude Code"),
                title: i18n::filled(
                    Key::TplAgentsNeedFormatChangedTitle,
                    &[("agent", "Claude Code"), ("version", "2.2.0")]
                ),
                line: i18n::filled(
                    Key::TplAgentsNeedFormatChangedText,
                    &[
                        ("agent", "Claude Code"),
                        ("tested", "2.1.0"),
                        ("time", &since)
                    ]
                ),
                fix: Some(NeedFixViewModel {
                    label: i18n::t(Key::AgentsCopyReport).to_string(),
                    action: NeedAction::CopyReport(
                        change.report("Claude Code", "~/.claude")
                    ),
                }),
            }]
        );
        assert!(account_needs(&state).is_empty());
    }

    /// With no version in the files and no good count yet, the need names
    /// the agent only and says no tokens are shown.
    #[test]
    fn a_changed_format_with_no_version_and_no_numbers_says_so() {
        let mut state = with_profiles(&[(claude(), connected())]);
        let error =
            SourceError::FormatChanged(Box::new(format_change(None, 40, 0)));
        let _ = reduce(
            &mut state,
            spend_loaded(&claude(), SpendCount::Failed(error), at(20)),
        );

        let needs = profile_needs(&state);

        assert_eq!(
            needs[0].title,
            i18n::filled(
                Key::TplAgentsNeedFormatChangedNoVersion,
                &[("agent", "Claude Code")]
            )
        );
        assert_eq!(
            needs[0].line,
            i18n::filled(
                Key::TplAgentsNeedFormatChangedNoNumbers,
                &[("agent", "Claude Code"), ("tested", "2.1.0")]
            )
        );
    }

    #[test]
    fn an_organization_with_an_admin_key_needs_nothing() {
        let state = with_accounts(
            &[(claude(), api_key(), vec![organization()])],
            saved_with_admin_key(),
        );

        assert!(account_needs(&state).is_empty());
        assert!(profile_needs(&state).is_empty());
    }

    /// An organization found in two profiles asks for its admin key once.
    #[test]
    fn an_organization_of_two_profiles_needs_its_key_once() {
        let state = with_accounts(
            &[
                (claude(), api_key(), vec![organization()]),
                (claude_work(), api_key(), vec![organization()]),
            ],
            SavedProfiles::default(),
        );

        assert_eq!(account_needs(&state).len(), 1);
    }
}
