//! Log in again and Remove of an account with OTTY's own login (spec
//! 2026-11-23-accounts-apart-from-profiles, Decisions 6, 7 and 9, issue
//! 09). Log in again asks the provider part that made the own login for
//! the agent's login command pointed at its folder and runs it in a
//! terminal tab the feature watches, so the accounts and their numbers
//! are read again once it closes. Remove asks first, then forgets the own
//! login in `agents.json` and deletes its folder, never a profile folder.
//! The account's name and Hide stay, and the next read shows the account
//! again when the plugin still finds it in a profile.

use otty_agents::{Account, OwnLogin};

use super::add_account_dialog::log_in_label;
use super::event::AgentsOutcome;
use super::failure_text::source_error_text;
use super::profile_actions::{save, unread_file};
use super::state::AgentsState;
use super::types::PreparedOwnLogin;

/// OTTY's own logins `account` was found in, as `agents.json` keeps them:
/// made by the part that found the account. None before `agents.json` is
/// read.
pub(crate) fn own_logins_of(
    state: &AgentsState,
    account: &Account,
) -> Vec<OwnLogin> {
    let Some(saved) = state.profiles().saved() else {
        return Vec::new();
    };

    saved
        .own_logins()
        .iter()
        .filter(|login| login.is_of(account.plugin(), account.provider()))
        .filter(|login| account.is_found_in_own_login(login.id()))
        .cloned()
        .collect()
}

/// Whether `account` has OTTY's own login: its menu then has Log in again
/// and Remove, and its page and row the mark.
pub(crate) fn has_own_login(state: &AgentsState, account: &Account) -> bool {
    !own_logins_of(state, account).is_empty()
}

/// Ask the part for the login command of the first own login of
/// `account`; nothing for an account with none.
pub(crate) fn log_in_again(
    state: &AgentsState,
    account: &Account,
) -> Vec<AgentsOutcome> {
    let Some(login) = own_logins_of(state, account).into_iter().next() else {
        return Vec::new();
    };

    vec![AgentsOutcome::PrepareLogInAgain(login)]
}

/// Run the login command the part gave in a watched terminal tab, titled
/// as Add account's choice, such as "Log in with Codex", or tell why the
/// part gave none.
pub(crate) fn log_in_again_prepared(
    state: &AgentsState,
    answer: PreparedOwnLogin,
) -> Vec<AgentsOutcome> {
    match answer.command {
        Ok(command) => vec![AgentsOutcome::OpenFixTab {
            title: log_in_label(state.registry(), &answer.login),
            folder: state.env().home().to_path_buf(),
            command,
        }],
        Err(error) => vec![AgentsOutcome::Failed(source_error_text(&error))],
    }
}

/// Ask to confirm the removal of the own login of `account`; an account
/// with none can not be removed.
pub(crate) fn ask_removal(state: &mut AgentsState, account: Account) {
    if !has_own_login(state, &account) {
        return;
    }

    state.ask_own_login_removal(account);
}

/// Remove the own logins of the account the user confirmed: their entries
/// leave `agents.json` and their folders are deleted. Nothing else goes,
/// and the save reads the accounts again.
pub(crate) fn confirm_removal(state: &mut AgentsState) -> Vec<AgentsOutcome> {
    let Some(account) = state.take_own_login_removal() else {
        return Vec::new();
    };
    let logins = own_logins_of(state, &account);
    let Some(saved) = state.profiles().saved() else {
        return vec![unread_file()];
    };

    let changed = logins.iter().fold(saved.clone(), |saved, login| {
        saved.without_own_login(login.id())
    });
    state.profiles_mut().apply_saved(changed.clone());

    let mut outcomes = vec![save(changed)];
    outcomes
        .extend(logins.into_iter().map(AgentsOutcome::RemoveOwnLoginFolder));
    outcomes
}

#[cfg(test)]
mod tests {
    use otty_agents::{
        AgentCommand, PartNeed, PartNeedAction, SavedProfiles, SourceError,
        Unavailable,
    };
    use otty_app_i18n::{self as i18n, Key};

    use crate::event::{AccountAction, AgentsIntent, AgentsOutcome};
    use crate::failure_text::source_error_text;
    use crate::model::{
        AccountPageViewModel, AllAccountsViewModel, AllProfilesViewModel,
        FactViewModel, RemoveDialogViewModel,
    };
    use crate::reducer::reduce;
    use crate::testing::{
        OWN_LOGIN_FOLDER, chatgpt_login, chatgpt_login_in_both,
        chatgpt_own_login, codex, codex_own_login, codex_work, connected,
        found_in_folders, loaded_profiles, now, open_account_menu,
        open_fix_tab, save, with_own_login, with_own_login_in,
    };
    use crate::types::PreparedOwnLogin;

    /// `codex login` in the folder of [`codex_own_login`].
    fn own_codex_login() -> AgentCommand {
        AgentCommand::new("codex")
            .with_arg("login")
            .with_env("CODEX_HOME", OWN_LOGIN_FOLDER)
    }

    fn prepared(command: Result<AgentCommand, SourceError>) -> AgentsIntent {
        AgentsIntent::LogInAgainPrepared(PreparedOwnLogin {
            login: codex_own_login(),
            command,
        })
    }

    /// Issue 09: Log in again of the menu asks the part that made the own
    /// login for the agent's login command pointed at its folder.
    #[test]
    fn log_in_again_asks_for_the_login_command_of_the_own_folder() {
        let mut state = with_own_login(vec![chatgpt_own_login()]);
        open_account_menu(&mut state, chatgpt_own_login());

        let outcomes = reduce(
            &mut state,
            AgentsIntent::AccountMenu(AccountAction::LogInAgain),
        );

        assert_eq!(
            outcomes,
            [AgentsOutcome::PrepareLogInAgain(codex_own_login())]
        );
        assert!(state.menu().is_none());
    }

    /// The Log in again button of the page header does the same.
    #[test]
    fn the_button_of_the_page_logs_in_again_too() {
        let mut state = with_own_login(vec![chatgpt_own_login()]);

        let outcomes =
            reduce(&mut state, AgentsIntent::LogInAgain(chatgpt_own_login()));

        assert_eq!(
            outcomes,
            [AgentsOutcome::PrepareLogInAgain(codex_own_login())]
        );
    }

    /// Issue 09: the login command runs in the same folder in a watched
    /// terminal tab; once it closes, the accounts and their numbers are
    /// read again.
    #[test]
    fn the_login_runs_in_a_watched_tab_that_reads_again_once_closed() {
        let mut state = with_own_login(vec![chatgpt_own_login()]);

        let opened = reduce(&mut state, prepared(Ok(own_codex_login())));
        let _ = reduce(&mut state, AgentsIntent::FixTabOpened(9));
        let closed = reduce(&mut state, AgentsIntent::TabClosed(9));

        let title =
            i18n::filled(Key::TplAgentsLogInWith, &[("agent", "Codex")]);
        assert_eq!(
            opened,
            [open_fix_tab(&title, "/home/user", own_codex_login())]
        );
        assert_eq!(closed, [AgentsOutcome::LoadProfiles]);
    }

    /// A part that gives no login command is told.
    #[test]
    fn a_part_with_no_login_command_is_told() {
        let mut state = with_own_login(vec![chatgpt_own_login()]);
        let error = SourceError::NotAvailable(Unavailable::NoLogIn);

        let outcomes = reduce(&mut state, prepared(Err(error.clone())));

        assert_eq!(
            outcomes,
            [AgentsOutcome::Failed(source_error_text(&error))]
        );
    }

    /// Issue 09: an account OTTY only found in a profile can not log in
    /// again or be removed from its menu.
    #[test]
    fn a_found_account_can_not_log_in_again_or_be_removed() {
        let mut state = with_own_login(vec![chatgpt_login()]);

        open_account_menu(&mut state, chatgpt_login());
        let again = reduce(
            &mut state,
            AgentsIntent::AccountMenu(AccountAction::LogInAgain),
        );
        open_account_menu(&mut state, chatgpt_login());
        let removed = reduce(
            &mut state,
            AgentsIntent::AccountMenu(AccountAction::Remove),
        );

        assert!(again.is_empty());
        assert!(removed.is_empty());
        assert_eq!(RemoveDialogViewModel::from_state(&state), None);
    }

    /// Issue 09: Remove asks first, naming the private folder OTTY
    /// deletes and saying the user's agents stay as they are.
    #[test]
    fn remove_asks_first_with_the_private_folder() {
        let mut state = with_own_login(vec![chatgpt_own_login()]);
        open_account_menu(&mut state, chatgpt_own_login());

        let asked = reduce(
            &mut state,
            AgentsIntent::AccountMenu(AccountAction::Remove),
        );
        let dialog = RemoveDialogViewModel::from_state(&state);

        assert!(asked.is_empty());
        assert_eq!(
            dialog,
            Some(RemoveDialogViewModel {
                title: i18n::t(Key::AgentsRemoveAccountTitle).to_string(),
                text: i18n::filled(
                    Key::TplAgentsRemoveAccountText,
                    &[("path", "~/.local/share/otty/accounts/6f0c1e2a")]
                ),
                confirm_label: i18n::t(Key::ButtonRemove),
            })
        );
    }

    /// Cancel removes nothing.
    #[test]
    fn a_cancelled_remove_keeps_everything() {
        let mut state = with_own_login(vec![chatgpt_own_login()]);
        open_account_menu(&mut state, chatgpt_own_login());
        let _ = reduce(
            &mut state,
            AgentsIntent::AccountMenu(AccountAction::Remove),
        );

        let cancelled = reduce(&mut state, AgentsIntent::RemoveCancelled);
        let confirmed = reduce(&mut state, AgentsIntent::RemoveConfirmed);

        assert!(cancelled.is_empty());
        assert!(confirmed.is_empty());
        assert_eq!(RemoveDialogViewModel::from_state(&state), None);
    }

    /// Issue 09: a confirmed Remove deletes only the own login's folder and
    /// its `own_logins` entry. The added profile, the account's name and
    /// its Hide stay in `agents.json`, as for a removed plugin.
    #[test]
    fn a_confirmed_remove_deletes_the_own_folder_and_entry_only() {
        let kept = SavedProfiles::default()
            .with_added(codex_work())
            .with_account_name(&chatgpt_own_login(), Some("Personal".into()))
            .with_account_hidden(&chatgpt_own_login(), true);
        let mut state =
            with_own_login_in(vec![chatgpt_own_login()], kept.clone());
        open_account_menu(&mut state, chatgpt_own_login());
        let _ = reduce(
            &mut state,
            AgentsIntent::AccountMenu(AccountAction::Remove),
        );

        let removed = reduce(&mut state, AgentsIntent::RemoveConfirmed);

        assert_eq!(
            removed,
            [
                save(kept.clone()),
                AgentsOutcome::RemoveOwnLoginFolder(codex_own_login()),
            ]
        );
        assert_eq!(state.profiles().saved(), Some(&kept));
        assert_eq!(RemoveDialogViewModel::from_state(&state), None);
    }

    /// Issue 09: an account the plugin still finds in `~/.codex` stays
    /// after Remove, with Source "Found in ~/.codex" only.
    #[test]
    fn an_account_also_found_in_a_profile_stays_found_there() {
        let mut state = with_own_login(vec![chatgpt_login_in_both()]);
        open_account_menu(&mut state, chatgpt_login_in_both());
        let _ = reduce(
            &mut state,
            AgentsIntent::AccountMenu(AccountAction::Remove),
        );
        let _ = reduce(&mut state, AgentsIntent::RemoveConfirmed);
        let _ = reduce(&mut state, AgentsIntent::ProfilesSaved(Ok(())));

        let mut loaded = loaded_profiles(
            &[(codex(), connected())],
            SavedProfiles::default(),
        );
        loaded.accounts =
            otty_agents::FoundAccounts::new(vec![chatgpt_login()], Vec::new());
        let _ = reduce(&mut state, AgentsIntent::ProfilesLoaded(loaded));
        let page = AccountPageViewModel::of(&state, chatgpt_login(), now());

        let source = i18n::t(Key::AgentsFieldSource);
        let sources: Vec<&FactViewModel> = page
            .info
            .iter()
            .skip_while(|fact| fact.label != source)
            .take_while(|fact| fact.label == source || fact.label.is_empty())
            .collect();
        assert_eq!(state.profiles().accounts(), [chatgpt_login()]);
        assert_eq!(
            sources,
            [&FactViewModel::new(
                source,
                i18n::filled(
                    Key::TplAgentsSourceFoundIn,
                    &[("folder", "~/.codex")]
                )
            )]
        );
        assert!(!page.has_own_login);
    }

    /// Spec Decision 9: a refused own login shows "Log in again" on the
    /// account and in All accounts → Needs you only; there is no profile.
    #[test]
    fn a_refused_own_login_shows_log_in_again_on_the_account_only() {
        let need = PartNeed::new(
            otty_agents::fixtures::plugin_part("openai", "openai"),
            "log-in-again",
            "Log in again",
            "OpenAI refused OTTY's own login of this account.",
            PartNeedAction::Run(own_codex_login()),
        );
        let refused = found_in_folders(
            &chatgpt_own_login(),
            chatgpt_own_login().found_in().to_vec(),
        )
        .with_needs(vec![need]);
        let state = with_own_login(vec![refused.clone()]);

        let page = AccountPageViewModel::of(&state, refused, now());
        let accounts = AllAccountsViewModel::from_state(&state);
        let profiles = AllProfilesViewModel::from_state(&state);

        let titles: Vec<&str> =
            page.needs.iter().map(|need| need.title.as_str()).collect();
        assert_eq!(titles, ["Log in again"]);
        assert_eq!(accounts.needs.len(), 1);
        assert!(profiles.needs.is_empty(), "{:?}", profiles.needs);
    }

    /// Issue 09, owner's answer: OTTY reads its own logins on the usual
    /// schedule while it runs. On screen the provider part may start the
    /// agent's program there, which keeps the own login fresh.
    #[test]
    fn an_own_login_is_read_on_the_usual_schedule() {
        let mut state = crate::state::AgentsState::new(
            crate::testing::agents(),
            crate::testing::registry(),
            crate::testing::env(),
        );
        let _ = reduce(&mut state, AgentsIntent::Enter);
        let saved = SavedProfiles::default().with_own_login(codex_own_login());
        let mut loaded = loaded_profiles(&[(codex(), connected())], saved);
        loaded.accounts = otty_agents::FoundAccounts::new(
            vec![chatgpt_own_login()],
            Vec::new(),
        );

        let outcomes = reduce(&mut state, AgentsIntent::ProfilesLoaded(loaded));

        assert!(
            outcomes.contains(&AgentsOutcome::LoadAccountNumbers {
                account: chatgpt_own_login(),
                given: Vec::new(),
                may_start_programs: true,
            }),
            "{outcomes:?}"
        );
    }
}
