//! The dialog of a key a plugin part asks for with an `ask-key` need with
//! a slot (spec 2026-11-19-agent-plugins, issue 20), such as the admin key
//! that reads the bill of an organization: named by the part's label, with
//! its hint and its page, and where the key is kept, a file or an
//! environment variable. OTTY saves only the path or the variable name in
//! `agents.json`, for the account under the part's slot, never the key.
//! The dialog waits for the save, and a failed save shows in it.

use otty_agents::{Account, KeyAsk};

use super::event::AgentsOutcome;
use super::key_place_field::{KeyPlaceChoice, KeyPlaceField};
use super::profile_actions::{save, unread_file};
use super::state::AgentsState;

/// What the key dialog was used for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyDialogIntent {
    /// Where the key is kept was picked: a file or a variable.
    KeyChoicePicked(KeyPlaceChoice),
    /// The path of the key file or the name of the variable changed.
    KeyChanged(String),
    /// The page where the key is made was asked for: it opens in the
    /// browser.
    PageOpened,
    /// Save or Enter: save where the key is kept.
    Submitted,
    /// Cancel, the cross or Esc: close the dialog and save nothing.
    Cancelled,
}

/// The open dialog of a key a plugin part asks for, given for an account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KeyDialog {
    account: Account,
    plugin: String,
    ask: KeyAsk,
    key: KeyPlaceField,
    error: Option<String>,
    is_saving: bool,
}

impl KeyDialog {
    /// The account the key is given for.
    pub(crate) fn account(&self) -> &Account {
        &self.account
    }

    /// The name of the plugin that asks.
    pub(crate) fn plugin(&self) -> &str {
        &self.plugin
    }

    /// The key the part asks for.
    pub(crate) fn ask(&self) -> &KeyAsk {
        &self.ask
    }

    /// Where the key is kept, as typed.
    pub(crate) fn key(&self) -> &KeyPlaceField {
        &self.key
    }

    /// Why the key was not saved, ready to show.
    pub(crate) fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Whether the dialog waits for its save.
    pub(crate) fn is_saving(&self) -> bool {
        self.is_saving
    }

    /// Show `text` as why the save failed, and let the dialog save again.
    pub(crate) fn fail(&mut self, text: String) {
        self.error = Some(text);
        self.is_saving = false;
    }
}

/// Open the dialog of the key `ask` the plugin named `plugin` asks for,
/// given for `account`, with the place given before under its slot.
pub(crate) fn open(
    state: &mut AgentsState,
    account: Account,
    plugin: String,
    ask: KeyAsk,
) -> Vec<AgentsOutcome> {
    let given = state.profiles().given_keys_of(&account);
    let saved = given
        .iter()
        .find(|given| given.slot() == ask.slot())
        .map(|given| given.key());

    let dialog = KeyDialog {
        key: KeyPlaceField::new(saved),
        account,
        plugin,
        ask,
        error: None,
        is_saving: false,
    };
    state.open_key_dialog(dialog);
    Vec::new()
}

/// Reduce what the dialog was used for. Nothing happens while it is
/// closed.
pub(crate) fn reduce(
    state: &mut AgentsState,
    intent: KeyDialogIntent,
) -> Vec<AgentsOutcome> {
    match intent {
        KeyDialogIntent::Submitted => submit(state),
        KeyDialogIntent::Cancelled => {
            let _ = state.take_key_dialog();
            Vec::new()
        },
        KeyDialogIntent::PageOpened => state
            .key_dialog()
            .and_then(|dialog| dialog.ask().page())
            .map(|page| AgentsOutcome::OpenPage(page.to_string()))
            .into_iter()
            .collect(),
        KeyDialogIntent::KeyChoicePicked(choice) => {
            if let Some(dialog) = state.key_dialog_mut() {
                dialog.key.pick(choice);
                dialog.error = None;
            }
            Vec::new()
        },
        KeyDialogIntent::KeyChanged(text) => {
            if let Some(dialog) = state.key_dialog_mut() {
                dialog.key.set_text(text);
                dialog.error = None;
            }
            Vec::new()
        },
    }
}

/// Close the dialog after its save. Nothing happens while no dialog
/// waits for a save.
pub(crate) fn close_after_save(state: &mut AgentsState) {
    if state.key_dialog().is_some_and(KeyDialog::is_saving) {
        let _ = state.take_key_dialog();
    }
}

/// Whether the dialog of `state` can save: no save is on its way and the
/// place of the key is typed and well formed.
pub(crate) fn can_submit(state: &AgentsState) -> bool {
    let Some(dialog) = state.key_dialog() else {
        return false;
    };

    let home = state.env().home();
    !dialog.is_saving && matches!(dialog.key.key_place(home), Ok(Some(_)))
}

/// Save the typed place as the key given for the account under the
/// part's slot: the new content of `agents.json` shows at once and is
/// written, and the account is read again with it, so its earlier read
/// and its need go.
fn submit(state: &mut AgentsState) -> Vec<AgentsOutcome> {
    if !can_submit(state) {
        return Vec::new();
    }
    let Some(dialog) = state.key_dialog() else {
        return Vec::new();
    };
    let Ok(Some(key)) = dialog.key.key_place(state.env().home()) else {
        return Vec::new();
    };
    let Some(saved) = state.profiles().saved() else {
        return vec![unread_file()];
    };

    let account = dialog.account.clone();
    let changed =
        saved
            .clone()
            .with_given_key_of(&account, dialog.ask.slot(), Some(key));

    if let Some(dialog) = state.key_dialog_mut() {
        dialog.is_saving = true;
        dialog.error = None;
    }
    state.profiles_mut().apply_saved(changed.clone());
    state.profiles_mut().forget_own_read(&account);
    vec![save(changed)]
}

#[cfg(test)]
mod tests {
    use otty_agents::{GivenKey, KeyRef, SavedProfiles};

    use super::*;
    use crate::event::AgentsIntent;
    use crate::need::NeedAction;
    use crate::reducer::reduce as reduce_all;
    use crate::testing::{
        api_key, bill_key_needed, claude, key_ask, now, organization,
        profiles_with_accounts_loaded, with_accounts,
    };
    use crate::types::LoadedBill;

    fn send(
        state: &mut AgentsState,
        intent: KeyDialogIntent,
    ) -> Vec<AgentsOutcome> {
        reduce_all(state, AgentsIntent::KeyDialog(intent))
    }

    fn saved_of(outcomes: &[AgentsOutcome]) -> Option<&SavedProfiles> {
        outcomes.iter().find_map(|outcome| match outcome {
            AgentsOutcome::SaveProfiles { saved, .. } => Some(saved),
            _ => None,
        })
    }

    /// The state of an organization whose bill needs the admin key, with
    /// the key dialog of its need open.
    fn asking_for_the_admin_key() -> AgentsState {
        let profiles = [(claude(), api_key(), vec![organization()])];
        let mut state = with_accounts(&profiles, SavedProfiles::default());
        let _ = reduce_all(
            &mut state,
            AgentsIntent::BillLoaded(LoadedBill {
                profile: claude(),
                given: Vec::new(),
                bill: Err(bill_key_needed()),
                read_at: now(),
            }),
        );
        let _ = reduce_all(
            &mut state,
            AgentsIntent::FixNeed(NeedAction::AskKey {
                account: organization(),
                plugin: String::from("Anthropic"),
                ask: key_ask(),
            }),
        );

        state
    }

    /// Issue 20: the key given in the field a part's `ask-key` opened is
    /// kept by its place under the part's slot, and the next read of the
    /// bill gets it with that slot.
    #[test]
    fn a_key_given_for_a_slot_reaches_the_next_bill_read() {
        let profiles = [(claude(), api_key(), vec![organization()])];
        let mut state = asking_for_the_admin_key();
        let variable = KeyRef::EnvVar(String::from("ANTHROPIC_ORG_KEY"));
        let _ = send(
            &mut state,
            KeyDialogIntent::KeyChoicePicked(KeyPlaceChoice::Variable),
        );
        let _ = send(
            &mut state,
            KeyDialogIntent::KeyChanged(String::from("ANTHROPIC_ORG_KEY")),
        );

        let submitted = send(&mut state, KeyDialogIntent::Submitted);
        let saved = saved_of(&submitted).expect("the key is saved").clone();
        let reload =
            reduce_all(&mut state, AgentsIntent::ProfilesSaved(Ok(())));
        let reloaded = reduce_all(
            &mut state,
            profiles_with_accounts_loaded(&profiles, saved.clone()),
        );

        let given = vec![GivenKey::new("admin", variable)];
        assert_eq!(saved.given_keys_of(&organization()), given.as_slice());
        assert_eq!(state.key_dialog(), None);
        assert_eq!(reload, [AgentsOutcome::LoadProfiles]);
        assert!(reloaded.contains(&AgentsOutcome::LoadBill {
            profile: claude(),
            organization: organization(),
            given,
        }));
    }

    /// The dialog opens on the place given before under the slot.
    #[test]
    fn the_dialog_shows_the_place_given_before_under_the_slot() {
        let variable = KeyRef::EnvVar(String::from("ANTHROPIC_ORG_KEY"));
        let saved = SavedProfiles::default().with_given_key_of(
            &organization(),
            "admin",
            Some(variable),
        );
        let mut state = with_accounts(
            &[(claude(), api_key(), vec![organization()])],
            saved,
        );

        let _ = reduce_all(
            &mut state,
            AgentsIntent::FixNeed(NeedAction::AskKey {
                account: organization(),
                plugin: String::from("Anthropic"),
                ask: key_ask(),
            }),
        );

        let dialog = state.key_dialog().expect("the dialog is open");
        assert_eq!(dialog.key().choice(), KeyPlaceChoice::Variable);
        assert_eq!(dialog.key().text(), "ANTHROPIC_ORG_KEY");
    }

    #[test]
    fn nothing_is_saved_until_a_full_path_or_a_variable_name_is_typed() {
        let mut state = asking_for_the_admin_key();

        let empty = send(&mut state, KeyDialogIntent::Submitted);
        let _ =
            send(&mut state, KeyDialogIntent::KeyChanged(String::from("key")));
        let not_full = send(&mut state, KeyDialogIntent::Submitted);

        assert!(empty.is_empty());
        assert!(not_full.is_empty());
        assert!(!can_submit(&state));
    }

    #[test]
    fn a_failed_save_stays_in_the_dialog() {
        let mut state = asking_for_the_admin_key();
        let _ = send(
            &mut state,
            KeyDialogIntent::KeyChanged(String::from("~/.config/org-key")),
        );
        let _ = send(&mut state, KeyDialogIntent::Submitted);

        let _ = reduce_all(
            &mut state,
            AgentsIntent::ProfilesSaved(Err(String::from("disk full"))),
        );

        let dialog = state.key_dialog().expect("the dialog stays");
        assert!(!dialog.is_saving());
        assert!(
            dialog
                .error()
                .is_some_and(|error| error.contains("disk full"))
        );
    }

    /// Cancel closes the dialog and saves nothing; the page button opens
    /// the page of the key.
    #[test]
    fn cancel_saves_nothing_and_the_page_opens() {
        let mut state = asking_for_the_admin_key();

        let page = send(&mut state, KeyDialogIntent::PageOpened);
        let cancelled = send(&mut state, KeyDialogIntent::Cancelled);

        assert_eq!(
            page,
            [AgentsOutcome::OpenPage(String::from(
                "https://example.com/keys"
            ))]
        );
        assert!(cancelled.is_empty());
        assert_eq!(state.key_dialog(), None);
    }
}
