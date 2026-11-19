//! What the user is asked to do (spec "Words" and Decision 6): each need
//! is a title, a line and one action, collected per account and per
//! profile and never mixed. The Found by OTTY report and the All accounts
//! and All profiles pages list them.

use otty_agents::Account;
use otty_app_i18n::{self as i18n, Key};

use super::super::need::NeedAction;
use super::super::profile_state::ProfileState;
use super::super::state::AgentsState;
use super::account_name::account_name;
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
    pub(crate) label: &'static str,
    /// What it does.
    pub(crate) action: NeedAction,
}

/// What the accounts need from the user, in the order of the accounts:
/// an organization whose bill OTTY could read with an admin key it has
/// not got.
pub(crate) fn account_needs(state: &AgentsState) -> Vec<NeedViewModel> {
    state
        .profiles()
        .accounts()
        .iter()
        .filter_map(|account| admin_key_need(state, account))
        .collect()
}

/// What the profiles need from the user, in the order of the profiles: a
/// profile that can not be used, with Log in while it can log in.
pub(crate) fn profile_needs(state: &AgentsState) -> Vec<NeedViewModel> {
    state
        .profiles()
        .shown()
        .iter()
        .filter_map(|shown| profile_need(state, shown))
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
        place: account_name(account),
        title: i18n::t(Key::AgentsNeedAdminKeyTitle).to_string(),
        line: i18n::t(Key::AgentsNeedAdminKeyText).to_string(),
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
        line: need.hint().to_string(),
        fix,
    })
}

#[cfg(test)]
mod tests {
    use otty_agents::{ProfileStatus, SavedProfiles};

    use super::*;
    use crate::testing::{
        api_key, claude, claude_work, codex, codex_work, organization,
        saved_with_admin_key, with_accounts,
    };

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
                    label: i18n::t(Key::AgentsAddAdminKey),
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
                        label: i18n::t(Key::AgentsLogIn),
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
