//! What the items of the menus of an account and of a profile do (spec
//! 2026-11-19, Decision 7), on the account or the profile of the open
//! menu, and the items of the "⋯" of All accounts and All profiles. An
//! item closes its menu first. A menu opened by a right press on a row
//! leaves what is shown as it is.

use otty_app_i18n::{self as i18n, Key};

use super::event::{AccountAction, AgentsOutcome, ProfileAction};
use super::menus::AgentsMenu;
use super::state::AgentsState;
use super::{hiding, rename_dialog};

/// Open `menu` at the pointer; it takes the keyboard.
pub(crate) fn open(
    state: &mut AgentsState,
    menu: AgentsMenu,
) -> Vec<AgentsOutcome> {
    state.open_menu(menu);

    vec![AgentsOutcome::MenuOpened]
}

/// Do what the item `action` of the open menu of an account asks.
/// Nothing happens while no account menu is open.
pub(crate) fn act_on_account(
    state: &mut AgentsState,
    action: AccountAction,
) -> Vec<AgentsOutcome> {
    let Some(AgentsMenu::Account(account)) =
        state.menu().map(|open| open.menu().clone())
    else {
        return Vec::new();
    };

    let _ = state.close_menu();
    match action {
        AccountAction::Rename => {
            rename_dialog::open_for_account(state, account)
        },
        AccountAction::OpenUsagePage => state
            .registry()
            .usage_page(&account)
            .map(|page| vec![AgentsOutcome::OpenPage(page.to_string())])
            .unwrap_or_default(),
        AccountAction::Hide => {
            hiding::set_account_hidden(state, &account, true)
        },
        AccountAction::ShowAgain => {
            hiding::set_account_hidden(state, &account, false)
        },
    }
}

/// Do what the item `action` of the open menu of a profile asks. Nothing
/// happens while no profile menu is open.
pub(crate) fn act_on_profile(
    state: &mut AgentsState,
    action: ProfileAction,
) -> Vec<AgentsOutcome> {
    let Some(AgentsMenu::Profile(profile)) =
        state.menu().map(|open| open.menu().clone())
    else {
        return Vec::new();
    };

    let _ = state.close_menu();
    match action {
        ProfileAction::Rename => {
            rename_dialog::open_for_profile(state, profile)
        },
        ProfileAction::OpenFolder => {
            vec![AgentsOutcome::ShowInFiles(
                profile.config_dir().to_path_buf(),
            )]
        },
        ProfileAction::Hide => {
            hiding::set_profile_hidden(state, &profile, true)
        },
        ProfileAction::ShowAgain => {
            hiding::set_profile_hidden(state, &profile, false)
        },
    }
}

/// List the hidden accounts or not, from the "⋯" of All accounts.
pub(crate) fn show_hidden_accounts(state: &mut AgentsState, is_showing: bool) {
    let _ = state.close_menu();
    state.show_hidden_accounts(is_showing);
}

/// List the hidden profiles or not, from the "⋯" of All profiles.
pub(crate) fn show_hidden_profiles(state: &mut AgentsState, is_showing: bool) {
    let _ = state.close_menu();
    state.show_hidden_profiles(is_showing);
}

/// Open the Found by OTTY report again, from the "⋯" of All accounts.
pub(crate) fn show_found_report(state: &mut AgentsState) {
    let _ = state.close_menu();
    state.reopen_found_report();
}

/// Tell why the browser could not be started for a page.
pub(crate) fn page_opened(result: Result<(), String>) -> Vec<AgentsOutcome> {
    match result {
        Ok(()) => Vec::new(),
        Err(error) => vec![AgentsOutcome::Failed(i18n::with_error(
            Key::TplLinkOpenFailed,
            &error,
        ))],
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use otty_agents::SavedProfiles;

    use crate::event::{
        AccountAction, AgentsIntent, AgentsOutcome, ProfileAction,
    };
    use crate::reducer::reduce;
    use crate::testing::{
        chatgpt_login, codex, connected, open_account_menu, open_profile_menu,
        with_accounts,
    };

    fn report_shown() -> SavedProfiles {
        SavedProfiles::default().with_found_report_shown()
    }

    #[test]
    fn open_usage_page_opens_the_page_of_the_provider_in_the_browser() {
        let mut state = with_accounts(
            &[(codex(), connected(), vec![chatgpt_login()])],
            report_shown(),
        );
        open_account_menu(&mut state, chatgpt_login());

        let outcomes = reduce(
            &mut state,
            AgentsIntent::AccountMenu(AccountAction::OpenUsagePage),
        );

        assert_eq!(
            outcomes,
            vec![AgentsOutcome::OpenPage(String::from(
                "https://chatgpt.com/codex/settings/usage"
            ))]
        );
        assert!(state.menu().is_none());
    }

    #[test]
    fn open_folder_shows_the_folder_of_the_profile_in_files() {
        let mut state = with_accounts(
            &[(codex(), connected(), Vec::new())],
            report_shown(),
        );
        open_profile_menu(&mut state, codex());

        let outcomes = reduce(
            &mut state,
            AgentsIntent::ProfileMenu(ProfileAction::OpenFolder),
        );

        assert_eq!(
            outcomes,
            vec![AgentsOutcome::ShowInFiles(PathBuf::from(
                "/home/user/.codex"
            ))]
        );
        assert!(state.menu().is_none());
    }

    /// A right click on a row opens the menu of that row: what is shown
    /// stays.
    #[test]
    fn the_menu_of_a_row_leaves_the_selection_as_it_is() {
        let mut state = with_accounts(
            &[(codex(), connected(), vec![chatgpt_login()])],
            report_shown(),
        );
        let _ = reduce(&mut state, AgentsIntent::SelectAllProfiles);

        open_account_menu(&mut state, chatgpt_login());
        open_profile_menu(&mut state, codex());

        assert!(state.is_all_profiles_selected());
        assert!(state.menu().is_some());
    }

    #[test]
    fn an_item_with_no_open_menu_does_nothing() {
        let mut state = with_accounts(
            &[(codex(), connected(), vec![chatgpt_login()])],
            report_shown(),
        );

        let outcomes = reduce(
            &mut state,
            AgentsIntent::AccountMenu(AccountAction::OpenUsagePage),
        );

        assert!(outcomes.is_empty());
    }

    /// Owner, issue 03: "Show Found by OTTY" of All accounts opens the
    /// report again; closing it then writes nothing, as it was shown.
    #[test]
    fn show_found_by_otty_opens_the_report_again() {
        let mut state = with_accounts(
            &[(codex(), connected(), vec![chatgpt_login()])],
            report_shown(),
        );
        let _ = reduce(&mut state, AgentsIntent::OpenAllAccountsMenu);

        let shown = reduce(&mut state, AgentsIntent::ShowFoundReport);
        let is_open = state.is_found_report_open();
        let closed = reduce(&mut state, AgentsIntent::CloseFoundReport);

        assert!(shown.is_empty());
        assert!(is_open);
        assert!(state.menu().is_none());
        assert!(closed.is_empty());
        assert!(!state.is_found_report_open());
    }

    #[test]
    fn the_page_that_could_not_open_is_told() {
        let mut state = with_accounts(
            &[(codex(), connected(), vec![chatgpt_login()])],
            report_shown(),
        );

        let outcomes = reduce(
            &mut state,
            AgentsIntent::PageOpened(Err(String::from("no xdg-open"))),
        );

        assert!(matches!(
            outcomes.as_slice(),
            [AgentsOutcome::Failed(text)] if text.contains("no xdg-open")
        ));
    }
}
