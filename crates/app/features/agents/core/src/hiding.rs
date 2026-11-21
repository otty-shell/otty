//! Hidden accounts and profiles (spec 2026-11-19, Decision 7). Hide of a
//! menu saves the account or the profile hidden in `agents.json`, Show
//! again saves it shown, and the "⋯" of All accounts and All profiles
//! lists the hidden ones, marked hidden, or not. OTTY does not read what
//! is hidden (`profiles.rs`), and it leaves the lists, the chips and the
//! needs.

use otty_agents::{Account, Profile};

use super::event::AgentsOutcome;
use super::profile_actions::{save, unread_file};
use super::state::AgentsState;

/// Save `account` hidden, or shown again. The change shows at once; the
/// page of the account gives way to All accounts when it is hidden. An
/// account known only by its key can not be hidden: nothing taken from a
/// key is kept.
pub(crate) fn set_account_hidden(
    state: &mut AgentsState,
    account: &Account,
    is_hidden: bool,
) -> Vec<AgentsOutcome> {
    if !account.can_be_saved() {
        return Vec::new();
    }
    let Some(saved) = state.profiles().saved() else {
        return vec![unread_file()];
    };

    let changed = saved.clone().with_account_hidden(account, is_hidden);
    let is_on_screen = state
        .selected_account()
        .is_some_and(|selected| selected.is_same_account(account));

    if is_hidden && is_on_screen {
        state.select_all_accounts();
    }
    state.profiles_mut().apply_saved(changed.clone());
    vec![save(changed)]
}

/// Save `profile` hidden, or shown again. A hidden profile stops being
/// read at once and its page gives way to All profiles; a profile shown
/// again is read with the read of the profiles that follows the save.
pub(crate) fn set_profile_hidden(
    state: &mut AgentsState,
    profile: &Profile,
    is_hidden: bool,
) -> Vec<AgentsOutcome> {
    let Some(saved) = state.profiles().saved() else {
        return vec![unread_file()];
    };

    let changed = saved.clone().with_profile_hidden(profile, is_hidden);
    let is_on_screen = state
        .selected_profile()
        .is_some_and(|selected| selected.profile() == profile);

    if is_hidden && is_on_screen {
        state.select_all_profiles();
    }
    state.profiles_mut().apply_saved(changed.clone());
    vec![save(changed)]
}

#[cfg(test)]
mod tests {
    use otty_agents::{Account, SavedProfiles};

    use crate::event::{
        AccountAction, AgentsIntent, AgentsOutcome, ProfileAction,
    };
    use crate::model::LeftPanelViewModel;
    use crate::reducer::reduce;
    use crate::state::AgentsState;
    use crate::testing::{
        agents, api_key, chatgpt_login, claude, codex, codex_work, connected,
        env, now, open_account_menu, open_profile_menu, organization,
        profiles_with_accounts_loaded, profiles_with_hidden_loaded, save,
        saved_with_given_key, session, sessions_loaded,
        two_agents_with_sessions, with_accounts,
    };

    fn report_shown() -> SavedProfiles {
        SavedProfiles::default().with_found_report_shown()
    }

    /// Claude Code paying through the [`organization`], whose admin key
    /// is not given, and Codex logged in to the [`chatgpt_login`].
    fn two_accounts(saved: SavedProfiles) -> AgentsState {
        with_accounts(
            &[
                (claude(), api_key(), vec![organization()]),
                (codex(), connected(), vec![chatgpt_login()]),
            ],
            saved,
        )
    }

    fn listed_accounts(state: &AgentsState) -> Vec<(String, Option<String>)> {
        LeftPanelViewModel::from_state(state, now())
            .accounts
            .accounts
            .into_iter()
            .map(|row| (row.name, row.number.map(|number| number.text())))
            .collect()
    }

    fn listed_profiles(state: &AgentsState) -> Vec<(String, Option<String>)> {
        LeftPanelViewModel::from_state(state, now())
            .agents
            .profiles
            .into_iter()
            .map(|row| (row.name, row.number))
            .collect()
    }

    fn is_read(outcome: &AgentsOutcome) -> bool {
        matches!(
            outcome,
            AgentsOutcome::LoadLimits(_)
                | AgentsOutcome::LoadLimitsInBackground(_)
                | AgentsOutcome::LoadBill { .. }
        )
    }

    fn same_accounts(accounts: &[Account], expected: &[Account]) -> bool {
        accounts.len() == expected.len()
            && accounts
                .iter()
                .zip(expected)
                .all(|(one, other)| one.is_same_account(other))
    }

    /// Spec 2026-11-19, Decision 7: a hidden account is saved, leaves the
    /// lists and the needs, and its open page gives way to All accounts.
    #[test]
    fn hide_of_an_account_saves_it_and_it_leaves_the_lists_and_the_needs() {
        let mut state = two_accounts(report_shown());
        let _ = reduce(&mut state, AgentsIntent::SelectAccount(organization()));
        open_account_menu(&mut state, organization());

        let outcomes =
            reduce(&mut state, AgentsIntent::AccountMenu(AccountAction::Hide));

        assert_eq!(
            outcomes,
            vec![save(
                report_shown().with_account_hidden(&organization(), true)
            )]
        );
        assert!(same_accounts(
            &state.profiles().accounts(),
            &[chatgpt_login()]
        ));
        assert!(crate::model::account_needs(&state).is_empty());
        assert!(state.is_all_accounts_selected());
        assert!(state.menu().is_none());
    }

    /// Hidden means OTTY does not read it: no limits and no bill are
    /// asked for a hidden account, while its profiles are read as usual.
    #[test]
    fn a_hidden_account_has_no_limits_and_no_bill_read() {
        let first_read = |saved: SavedProfiles| {
            let mut state =
                AgentsState::new(agents(), crate::testing::registry(), env());
            let _ = reduce(&mut state, AgentsIntent::Enter);
            reduce(
                &mut state,
                profiles_with_accounts_loaded(
                    &[
                        (claude(), api_key(), vec![organization()]),
                        (codex(), connected(), vec![chatgpt_login()]),
                    ],
                    saved,
                ),
            )
        };
        let hidden = saved_with_given_key()
            .with_account_hidden(&organization(), true)
            .with_account_hidden(&chatgpt_login(), true);

        let shown_reads = first_read(saved_with_given_key());
        let reads = first_read(hidden);

        assert!(shown_reads.iter().any(is_read), "{shown_reads:?}");
        assert!(!reads.iter().any(is_read), "{reads:?}");
        assert!(
            reads
                .iter()
                .any(|outcome| matches!(outcome, AgentsOutcome::LoadSpend(_))),
            "the profiles are read: {reads:?}"
        );
    }

    #[test]
    fn show_hidden_accounts_lists_them_marked_hidden_and_show_again_saves() {
        let saved = report_shown().with_account_hidden(&chatgpt_login(), true);
        let mut state = two_accounts(saved);
        let before = listed_accounts(&state);

        let _ = reduce(&mut state, AgentsIntent::ShowHiddenAccounts(true));
        let listed = listed_accounts(&state);
        open_account_menu(&mut state, chatgpt_login());
        let outcomes = reduce(
            &mut state,
            AgentsIntent::AccountMenu(AccountAction::ShowAgain),
        );

        assert_eq!(before.len(), 1);
        assert_eq!(listed.len(), 2);
        assert_eq!(
            listed[1],
            (
                String::from("OpenAI · you@example.com"),
                Some(String::from("hidden"))
            )
        );
        assert_eq!(outcomes, vec![save(report_shown())]);
        assert!(!state.profiles().is_account_hidden(&chatgpt_login()));
    }

    /// A hidden profile is not read: its sessions leave the projects and
    /// no read of it starts; its open page gives way to All profiles.
    #[test]
    fn hide_of_a_profile_saves_it_and_it_leaves_the_lists_and_the_projects() {
        let mut state = two_agents_with_sessions();
        let _ = reduce(&mut state, AgentsIntent::SelectProfile(codex()));
        open_profile_menu(&mut state, codex());

        let outcomes =
            reduce(&mut state, AgentsIntent::ProfileMenu(ProfileAction::Hide));
        let late = reduce(
            &mut state,
            sessions_loaded(&codex(), vec![session("x2", "/work/new", 9)]),
        );

        assert_eq!(
            outcomes,
            vec![save(
                SavedProfiles::default().with_profile_hidden(&codex(), true)
            )]
        );
        assert_eq!(listed_profiles(&state).len(), 1);
        let otty = state
            .profiles()
            .project(std::path::Path::new("/work/otty"))
            .expect("Claude Code still has a session there");
        assert_eq!(otty.sessions().len(), 1);
        assert!(late.is_empty());
        assert!(
            state
                .profiles()
                .project(std::path::Path::new("/work/new"))
                .is_none()
        );
        assert!(state.is_all_profiles_selected());
    }

    /// A profile the read found hidden is not read, and shows, marked
    /// hidden, only while Show hidden profiles is on.
    #[test]
    fn a_hidden_profile_is_not_read_and_shows_only_with_show_hidden() {
        let mut state =
            AgentsState::new(agents(), crate::testing::registry(), env());
        let _ = reduce(&mut state, AgentsIntent::Enter);
        let saved = report_shown().with_profile_hidden(&codex_work(), true);

        let outcomes = reduce(
            &mut state,
            profiles_with_hidden_loaded(
                &[(codex(), connected(), Vec::new())],
                &[codex_work()],
                saved,
            ),
        );
        let before = listed_profiles(&state);
        let _ = reduce(&mut state, AgentsIntent::ShowHiddenProfiles(true));
        let listed = listed_profiles(&state);

        assert!(
            !outcomes.iter().any(|outcome| matches!(
                outcome,
                AgentsOutcome::LoadSessions(profile)
                    | AgentsOutcome::LoadSpend(profile)
                    if *profile == codex_work()
            )),
            "{outcomes:?}"
        );
        assert_eq!(before.len(), 1);
        assert_eq!(
            listed[1],
            (String::from("Codex · Work"), Some(String::from("hidden")))
        );
    }

    #[test]
    fn show_again_of_a_profile_saves_it_shown() {
        let mut state =
            AgentsState::new(agents(), crate::testing::registry(), env());
        let _ = reduce(&mut state, AgentsIntent::Enter);
        let saved = report_shown().with_profile_hidden(&codex_work(), true);
        let _ = reduce(
            &mut state,
            profiles_with_hidden_loaded(
                &[(codex(), connected(), Vec::new())],
                &[codex_work()],
                saved,
            ),
        );
        open_profile_menu(&mut state, codex_work());

        let outcomes = reduce(
            &mut state,
            AgentsIntent::ProfileMenu(ProfileAction::ShowAgain),
        );

        assert_eq!(outcomes, vec![save(report_shown())]);
    }
}
