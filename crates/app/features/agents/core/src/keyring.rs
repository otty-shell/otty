//! The system keyring (spec 2026-11-19, Decision 8): some agents, such as
//! Goose, keep their keys there. OTTY reads the entries of the known agents
//! found on this computer, and only after the user allowed it once in the
//! keyring dialog; the allowance is saved in `agents.json`.
//!
//! The read may make the system ask the user to unlock the keyring, so it
//! is asked only while Agents mode is on screen: once in a run, again after
//! Refresh, and at once after Allow or Try again. The accounts of the keys
//! found there are listed with the others, found in "system keyring".

use otty_agents::{Account, KeyringEntry, KeyringError};

use super::event::AgentsOutcome;
use super::profile_actions::{save, unread_file};
use super::state::AgentsState;

/// What was read of the system keyring, and the entries there are to read.
#[derive(Debug, Default)]
pub(crate) struct KeyringState {
    entries: Vec<KeyringEntry>,
    accounts: Vec<Account>,
    failure: Option<KeyringError>,
    is_reading: bool,
    is_read: bool,
}

impl KeyringState {
    /// The keyring entries of the known agents found on this computer.
    pub(crate) fn entries(&self) -> &[KeyringEntry] {
        &self.entries
    }

    /// The accounts of the keys found in the keyring at its latest read.
    pub(crate) fn accounts(&self) -> &[Account] {
        &self.accounts
    }

    /// Why the latest read of the keyring failed; `None` when it did not.
    pub(crate) fn failure(&self) -> Option<KeyringError> {
        self.failure
    }

    /// Take the entries the latest read of the profiles found.
    pub(crate) fn set_entries(&mut self, entries: Vec<KeyringEntry>) {
        self.entries = entries;
    }

    /// Read the keyring again with the next read that may: Refresh.
    pub(crate) fn read_again_later(&mut self) {
        self.is_read = false;
    }

    /// Start a read and return the entries to read; `None` while a read is
    /// on its way or there is no entry.
    fn start_read(&mut self) -> Option<Vec<KeyringEntry>> {
        if self.is_reading || self.entries.is_empty() {
            return None;
        }

        self.is_reading = true;
        self.is_read = true;
        Some(self.entries.clone())
    }

    /// Take the answer of a read: the accounts found, or why it failed,
    /// which keeps the accounts of the read before.
    fn apply(&mut self, answer: Result<Vec<Account>, KeyringError>) {
        self.is_reading = false;

        match answer {
            Ok(accounts) => {
                self.accounts = accounts;
                self.failure = None;
            },
            Err(failure) => self.failure = Some(failure),
        }
    }
}

/// Ask for a read of the keyring when the user allowed it, Agents mode is
/// on screen and it was not read in this run or since the last Refresh.
pub(crate) fn ask(state: &mut AgentsState) -> Vec<AgentsOutcome> {
    let is_allowed = state
        .profiles()
        .saved()
        .is_some_and(|saved| saved.is_keyring_allowed());
    if !state.is_active() || !is_allowed || state.profiles().keyring().is_read {
        return Vec::new();
    }

    start_read(state)
}

/// Open the keyring dialog, which asks the user to allow the read.
pub(crate) fn open_dialog(state: &mut AgentsState) {
    state.set_keyring_dialog_open(true);
}

/// Allow of the keyring dialog: save the allowance in `agents.json` and
/// read the keyring at once.
pub(crate) fn allow(state: &mut AgentsState) -> Vec<AgentsOutcome> {
    state.set_keyring_dialog_open(false);
    let Some(saved) = state.profiles().saved() else {
        return vec![unread_file()];
    };

    let changed = saved.clone().with_keyring_allowed();
    state.profiles_mut().apply_saved(changed.clone());

    let mut outcomes = vec![save(changed)];
    outcomes.extend(start_read(state));
    outcomes
}

/// Not now of the keyring dialog: close it; nothing is read or saved.
pub(crate) fn not_now(state: &mut AgentsState) {
    state.set_keyring_dialog_open(false);
}

/// Try again of a failed read: read the keyring at once.
pub(crate) fn read_again(state: &mut AgentsState) -> Vec<AgentsOutcome> {
    start_read(state)
}

/// Take the answer of a read of the keyring.
pub(crate) fn apply(
    state: &mut AgentsState,
    answer: Result<Vec<Account>, KeyringError>,
) {
    state.profiles_mut().keyring_mut().apply(answer);
}

/// Start a read of the entries found, unless one is on its way.
fn start_read(state: &mut AgentsState) -> Vec<AgentsOutcome> {
    state
        .profiles_mut()
        .keyring_mut()
        .start_read()
        .map(AgentsOutcome::LoadKeyring)
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests {
    use otty_agents::fixtures::goose_keyring_entry;
    use otty_agents::{
        AccountIdentity, AccountKind, AccountLogin, FoundIn, KeyFingerprint,
        Provider, SavedProfiles,
    };

    use super::*;
    use crate::event::AgentsIntent;
    use crate::model::account_needs;
    use crate::need::NeedAction;
    use crate::reducer::reduce;
    use crate::testing::{
        agents, claude, connected, env, profiles_loaded_from, save as saved,
    };
    use crate::types::LoadedProfiles;

    fn report_shown() -> SavedProfiles {
        SavedProfiles::default().with_found_report_shown()
    }

    fn allowed() -> SavedProfiles {
        report_shown().with_keyring_allowed()
    }

    /// The answer of a profiles read with Claude Code, `saved` in
    /// `agents.json` and the keyring entry of Goose found.
    fn loaded_with_goose(saved: SavedProfiles) -> AgentsIntent {
        let AgentsIntent::ProfilesLoaded(loaded) =
            profiles_loaded_from(&[(claude(), connected())], saved)
        else {
            unreachable!("a profiles read is made");
        };

        AgentsIntent::ProfilesLoaded(LoadedProfiles {
            keyring_entries: vec![goose_keyring_entry()],
            ..loaded
        })
    }

    /// Agents mode on screen with Goose found and `saved` read.
    fn on_screen_with_goose(saved: SavedProfiles) -> AgentsState {
        let mut state = AgentsState::new(agents(), env());
        let _ = reduce(&mut state, AgentsIntent::Enter);
        let _ = reduce(&mut state, loaded_with_goose(saved));
        state
    }

    fn keyring_reads(outcomes: &[AgentsOutcome]) -> Vec<&AgentsOutcome> {
        outcomes
            .iter()
            .filter(|outcome| matches!(outcome, AgentsOutcome::LoadKeyring(_)))
            .collect()
    }

    fn read_goose() -> AgentsOutcome {
        AgentsOutcome::LoadKeyring(vec![goose_keyring_entry()])
    }

    fn keyring_need_actions(state: &AgentsState) -> Vec<Option<NeedAction>> {
        account_needs(state)
            .into_iter()
            .map(|need| need.fix.map(|fix| fix.action))
            .filter(|action| {
                matches!(
                    action,
                    Some(NeedAction::AllowKeyring | NeedAction::ReadKeyring)
                )
            })
            .collect()
    }

    fn openrouter_key() -> Account {
        Account::new(
            Provider::OpenRouter,
            AccountIdentity::KeyFingerprint(KeyFingerprint::of(
                "sk-or-v1-made-up-0000-key-c3d4",
            )),
            AccountLogin::Key(Some(String::from("c3d4"))),
            AccountKind::Money,
            FoundIn::SystemKeyring,
        )
    }

    #[test]
    fn nothing_is_read_from_the_keyring_before_the_user_allows_it() {
        let mut state = AgentsState::new(agents(), env());

        let entered = reduce(&mut state, AgentsIntent::Enter);
        let loaded = reduce(&mut state, loaded_with_goose(report_shown()));
        let _ = reduce(&mut state, AgentsIntent::Refresh);
        let again = reduce(&mut state, loaded_with_goose(report_shown()));

        assert!(keyring_reads(&entered).is_empty(), "{entered:?}");
        assert!(keyring_reads(&loaded).is_empty(), "{loaded:?}");
        assert!(keyring_reads(&again).is_empty(), "{again:?}");
    }

    /// The need shows while the keyring is not allowed; its button opens
    /// the dialog, and Allow saves the allowance, reads the keyring and
    /// the need is gone.
    #[test]
    fn allow_saves_the_allowance_reads_the_keyring_and_the_need_goes() {
        let mut state = on_screen_with_goose(report_shown());
        let before = keyring_need_actions(&state);

        let opened =
            reduce(&mut state, AgentsIntent::FixNeed(NeedAction::AllowKeyring));
        let is_open = state.is_keyring_dialog_open();
        let allowed_now = reduce(&mut state, AgentsIntent::AllowKeyring);

        assert_eq!(before, [Some(NeedAction::AllowKeyring)]);
        assert!(opened.is_empty(), "{opened:?}");
        assert!(is_open, "the button opens the dialog");
        assert_eq!(allowed_now, vec![saved(allowed()), read_goose()]);
        assert!(!state.is_keyring_dialog_open());
        assert!(keyring_need_actions(&state).is_empty());
    }

    #[test]
    fn not_now_closes_the_dialog_and_the_need_stays() {
        let mut state = on_screen_with_goose(report_shown());
        let _ =
            reduce(&mut state, AgentsIntent::FixNeed(NeedAction::AllowKeyring));

        let outcomes = reduce(&mut state, AgentsIntent::KeyringNotNow);

        assert!(outcomes.is_empty(), "{outcomes:?}");
        assert!(!state.is_keyring_dialog_open());
        assert_eq!(
            keyring_need_actions(&state),
            [Some(NeedAction::AllowKeyring)]
        );
    }

    /// No agent that keeps keys in the keyring is on this computer, so the
    /// user is not asked about it.
    #[test]
    fn without_such_an_agent_the_keyring_is_never_asked_about() {
        let mut state = AgentsState::new(agents(), env());
        let _ = reduce(&mut state, AgentsIntent::Enter);

        let outcomes = reduce(
            &mut state,
            profiles_loaded_from(&[(claude(), connected())], allowed()),
        );

        assert!(keyring_need_actions(&state).is_empty());
        assert!(keyring_reads(&outcomes).is_empty(), "{outcomes:?}");
    }

    /// The system may ask to unlock the keyring, so it is read only while
    /// the mode is on screen: once in a run, and again after Refresh.
    #[test]
    fn an_allowed_keyring_is_read_on_screen_once_and_after_refresh() {
        let mut state = AgentsState::new(agents(), env());

        let away = reduce(&mut state, loaded_with_goose(allowed()));
        let entered = reduce(&mut state, AgentsIntent::Enter);
        let _ = reduce(&mut state, AgentsIntent::KeyringLoaded(Ok(Vec::new())));
        let loaded_again = reduce(&mut state, loaded_with_goose(allowed()));
        let _ = reduce(&mut state, AgentsIntent::Refresh);
        let refreshed = reduce(&mut state, loaded_with_goose(allowed()));

        assert!(keyring_reads(&away).is_empty(), "{away:?}");
        assert_eq!(keyring_reads(&entered), [&read_goose()]);
        assert!(keyring_reads(&loaded_again).is_empty(), "{loaded_again:?}");
        assert_eq!(keyring_reads(&refreshed), [&read_goose()]);
    }

    #[test]
    fn the_accounts_of_the_keyring_are_listed_with_the_others() {
        let mut state = on_screen_with_goose(allowed());

        let _ = reduce(
            &mut state,
            AgentsIntent::KeyringLoaded(Ok(vec![openrouter_key()])),
        );

        assert_eq!(state.profiles().accounts(), [openrouter_key()]);
    }

    /// A failed read keeps the accounts found before and asks the user to
    /// try again, which reads at once.
    #[test]
    fn a_failed_read_asks_to_try_again() {
        let mut state = on_screen_with_goose(allowed());
        let _ = reduce(
            &mut state,
            AgentsIntent::KeyringLoaded(Ok(vec![openrouter_key()])),
        );

        let _ = reduce(
            &mut state,
            AgentsIntent::KeyringLoaded(Err(KeyringError::NotAllowed)),
        );
        let needs = keyring_need_actions(&state);
        let again =
            reduce(&mut state, AgentsIntent::FixNeed(NeedAction::ReadKeyring));

        assert_eq!(state.profiles().accounts(), [openrouter_key()]);
        assert_eq!(needs, [Some(NeedAction::ReadKeyring)]);
        assert_eq!(again, vec![read_goose()]);
    }

    /// `agents.json` could not be read, so the allowance could not be
    /// saved: the user is not asked.
    #[test]
    fn with_an_unread_agents_json_the_keyring_is_not_asked_about() {
        let mut state = AgentsState::new(agents(), env());
        let _ = reduce(&mut state, AgentsIntent::Enter);
        let AgentsIntent::ProfilesLoaded(loaded) =
            loaded_with_goose(report_shown())
        else {
            unreachable!("a profiles read is made");
        };
        let _ = reduce(
            &mut state,
            AgentsIntent::ProfilesLoaded(LoadedProfiles {
                saved: None,
                ..loaded
            }),
        );

        assert!(keyring_need_actions(&state).is_empty());
    }
}
