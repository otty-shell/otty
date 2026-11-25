//! The terminal tab where OTTY's own login logs in (spec
//! 2026-11-23-accounts-apart-from-profiles, Decision 6), and what OTTY does
//! once it closes: it asks the provider part for the accounts in that
//! folder alone. An account found there is shown, read with the others.
//! With none, the folder and its `own_logins` entry go and the user is
//! told the login did not finish. A part that could not be asked keeps
//! both, since the login may well be there.

use otty_agents::OwnLogin;
use otty_app_i18n::{self as i18n, Key};

use super::event::AgentsOutcome;
use super::failure_text::source_error_text;
use super::profile_actions::save;
use super::reads;
use super::state::AgentsState;
use super::types::CheckedOwnLogin;

/// The tab `tab_id` closed: for a login tab, ask for the accounts in the
/// folder of its own login; `None` for a tab that is no login tab.
pub(crate) fn tab_closed(
    state: &mut AgentsState,
    tab_id: u64,
) -> Option<Vec<AgentsOutcome>> {
    let login = state.forget_login_tab(tab_id)?;

    Some(vec![AgentsOutcome::CheckOwnLogin(login)])
}

/// Take what the part found in the folder of an own login whose tab
/// closed.
pub(crate) fn checked(
    state: &mut AgentsState,
    checked: CheckedOwnLogin,
) -> Vec<AgentsOutcome> {
    match checked.found {
        Ok(true) => reads::load_profiles(state),
        Ok(false) => no_login(state, checked.login),
        Err(error) => {
            let mut outcomes =
                vec![AgentsOutcome::Failed(source_error_text(&error))];
            outcomes.extend(reads::load_profiles(state));
            outcomes
        },
    }
}

/// The folder of an own login with no account was removed, or why not.
pub(crate) fn folder_removed(result: Result<(), String>) -> Vec<AgentsOutcome> {
    match result {
        Ok(()) => Vec::new(),
        Err(text) => vec![AgentsOutcome::Failed(text)],
    }
}

/// No account in the folder of `login`: its entry goes from
/// `agents.json`, its folder is removed, and the user is told.
fn no_login(state: &mut AgentsState, login: OwnLogin) -> Vec<AgentsOutcome> {
    let mut outcomes = Vec::new();

    if let Some(saved) = state.profiles().saved() {
        let changed = saved.clone().without_own_login(login.id());
        state.profiles_mut().apply_saved(changed.clone());
        outcomes.push(save(changed));
    }

    outcomes.push(AgentsOutcome::RemoveOwnLoginFolder(login));
    outcomes.push(AgentsOutcome::Failed(
        i18n::t(Key::AgentsLoginNotFinished).to_string(),
    ));
    outcomes
}

#[cfg(test)]
mod tests {
    use otty_agents::{
        Account, AccountKind, FoundAccounts, FoundIn, OwnLogin, ProviderId,
        SavedProfiles, SourceError, Unavailable,
    };
    use otty_app_i18n::{self as i18n, Key};

    use crate::event::{AgentsIntent, AgentsOutcome};
    use crate::reducer::reduce;
    use crate::state::AgentsState;
    use crate::testing::{
        claude, claude_id, connected, loaded_profiles, with_saved_profiles,
    };
    use crate::types::CheckedOwnLogin;

    fn own_login() -> OwnLogin {
        OwnLogin::new(
            "6f0c1e2a",
            "anthropic",
            ProviderId::new("anthropic"),
            claude_id(),
        )
    }

    fn own_folder() -> &'static str {
        "/home/user/.local/share/otty/accounts/6f0c1e2a"
    }

    /// The Claude account found in OTTY's own login only.
    fn own_account() -> Account {
        Account::new(
            "anthropic",
            ProviderId::new("anthropic"),
            "claude:made-up",
            "own@example.com",
            AccountKind::Subscription,
            vec![FoundIn::own_login("6f0c1e2a", own_folder())],
        )
    }

    /// Agents mode with the own login saved and its login tab 7 open.
    fn logging_in() -> AgentsState {
        let saved = SavedProfiles::default().with_own_login(own_login());
        let mut state = with_saved_profiles(&[(claude(), connected())], saved);
        let _ = reduce(
            &mut state,
            AgentsIntent::LoginTabOpened {
                tab_id: 7,
                login: own_login(),
            },
        );

        state
    }

    fn checked(found: Result<bool, SourceError>) -> AgentsIntent {
        AgentsIntent::OwnLoginChecked(CheckedOwnLogin {
            login: own_login(),
            found,
        })
    }

    /// Spec 2026-11-23-accounts-apart-from-profiles, Decision 6: once the
    /// login tab closes, OTTY asks the part for the accounts in that
    /// folder, and the tab is no longer watched.
    #[test]
    fn a_closed_login_tab_asks_for_the_accounts_of_its_folder() {
        let mut state = logging_in();

        let closed = reduce(&mut state, AgentsIntent::TabClosed(7));
        let again = reduce(&mut state, AgentsIntent::TabClosed(7));

        assert_eq!(closed, [AgentsOutcome::CheckOwnLogin(own_login())]);
        assert!(again.is_empty());
        assert!(!state.is_fix_tab(7));
    }

    /// Decision 6: an account found there is shown: the profiles and the
    /// accounts are read again, and the account found in the own login is
    /// listed with no profile of its own.
    #[test]
    fn a_found_account_is_read_and_shown() {
        let mut state = logging_in();
        let _ = reduce(&mut state, AgentsIntent::TabClosed(7));

        let found = reduce(&mut state, checked(Ok(true)));
        let saved = SavedProfiles::default().with_own_login(own_login());
        let mut loaded = loaded_profiles(&[(claude(), connected())], saved);
        loaded.accounts = FoundAccounts::new(vec![own_account()], Vec::new());
        let _ = reduce(&mut state, AgentsIntent::ProfilesLoaded(loaded));

        assert_eq!(found, [AgentsOutcome::LoadProfiles]);
        assert_eq!(state.profiles().accounts(), [own_account()]);
        assert!(
            state
                .profiles()
                .shown()
                .iter()
                .all(|shown| shown.profile().config_dir() != own_folder()),
            "an own login is never a profile"
        );
    }

    /// Decision 6: with no account there, the folder and the entry go and
    /// the user is told the login did not finish.
    #[test]
    fn no_account_removes_the_folder_and_the_entry() {
        let mut state = logging_in();
        let _ = reduce(&mut state, AgentsIntent::TabClosed(7));

        let none = reduce(&mut state, checked(Ok(false)));

        assert_eq!(
            none,
            [
                AgentsOutcome::SaveProfiles {
                    saved: SavedProfiles::default(),
                    create_dir: None,
                },
                AgentsOutcome::RemoveOwnLoginFolder(own_login()),
                AgentsOutcome::Failed(
                    i18n::t(Key::AgentsLoginNotFinished).to_string()
                ),
            ]
        );
        assert_eq!(state.profiles().saved(), Some(&SavedProfiles::default()));
    }

    /// A part that could not be asked keeps the folder and the entry: the
    /// login may well be there, and the next read finds it.
    #[test]
    fn a_failed_check_keeps_the_own_login() {
        let mut state = logging_in();
        let _ = reduce(&mut state, AgentsIntent::TabClosed(7));

        let failed = reduce(
            &mut state,
            checked(Err(SourceError::NotAvailable(
                Unavailable::NoAccountSource,
            ))),
        );

        assert!(
            failed.iter().all(|outcome| !matches!(
                outcome,
                AgentsOutcome::RemoveOwnLoginFolder(_)
                    | AgentsOutcome::SaveProfiles { .. }
            )),
            "{failed:?}"
        );
        assert!(failed.contains(&AgentsOutcome::LoadProfiles), "{failed:?}");
        assert_eq!(
            state.profiles().saved().map(SavedProfiles::own_logins),
            Some([own_login()].as_slice())
        );
    }

    /// A folder that could not be removed is told.
    #[test]
    fn a_folder_that_could_not_be_removed_is_told() {
        let mut state = logging_in();

        let told = reduce(
            &mut state,
            AgentsIntent::OwnLoginFolderRemoved(Err(String::from("busy"))),
        );
        let fine =
            reduce(&mut state, AgentsIntent::OwnLoginFolderRemoved(Ok(())));

        assert_eq!(told, [AgentsOutcome::Failed(String::from("busy"))]);
        assert!(fine.is_empty());
    }
}
