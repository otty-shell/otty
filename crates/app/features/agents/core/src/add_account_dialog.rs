//! The Add account dialog (spec 2026-11-19, Decision 7), the last resort
//! for a key no agent uses: a provider the registry offers and where its
//! key is kept, a file or an environment variable. OTTY saves only the
//! path or the variable name in `agents.json`, never the key. Edit key…
//! of an account the user added opens the same dialog with its provider
//! fixed, and so does a key a plugin part asks for with a need (spec
//! 2026-11-19-agent-plugins, issue 20): named by the part's label, with
//! its hint and its page, saved for the account under the part's slot.
//! The dialog waits for the save, and a failed save shows in it.

use otty_agents::{Account, KeyAsk, KeyRef, ProviderRef, Registry};
use otty_app_i18n::{self as i18n, Key};

use super::event::AgentsOutcome;
use super::key_place_field::{KeyPlaceChoice, KeyPlaceField};
use super::profile_actions::{save, unread_file};
use super::state::AgentsState;

/// What the Add account dialog was used for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddAccountIntent {
    /// The provider drop-down was pressed: open or close its list.
    ProviderListToggled,
    /// A provider of the list was picked.
    ProviderPicked(ProviderRef),
    /// Where the key is kept was picked: a file or a variable.
    KeyChoicePicked(KeyPlaceChoice),
    /// The path of the key file or the name of the variable changed.
    KeyChanged(String),
    /// The page where the key a plugin part asks for is made was asked
    /// for: it opens in the browser.
    PageOpened,
    /// Add, Save or Enter: save the account.
    Submitted,
    /// Cancel, the cross or Esc: close the dialog and save nothing.
    Cancelled,
}

/// The open Add account dialog, the Edit key dialog of an account the
/// user added, or the dialog of a key a plugin part asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AddAccountDialog {
    editing: Option<Account>,
    asked: Option<AskedKey>,
    provider: ProviderRef,
    is_provider_list_open: bool,
    key: KeyPlaceField,
    error: Option<String>,
    is_saving: bool,
}

impl AddAccountDialog {
    /// The account whose key is edited; `None` while an account is added.
    pub(crate) fn editing(&self) -> Option<&Account> {
        self.editing.as_ref()
    }

    /// The key a plugin part asks for, while the dialog gives one.
    pub(crate) fn asked(&self) -> Option<&AskedKey> {
        self.asked.as_ref()
    }

    /// The provider picked.
    pub(crate) fn provider(&self) -> &ProviderRef {
        &self.provider
    }

    /// Whether the list of the providers is open.
    pub(crate) fn is_provider_list_open(&self) -> bool {
        self.is_provider_list_open
    }

    /// Where the key is kept, as typed.
    pub(crate) fn key(&self) -> &KeyPlaceField {
        &self.key
    }

    /// Why the account was not saved, ready to show.
    pub(crate) fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Whether the dialog waits for its save.
    pub(crate) fn is_saving(&self) -> bool {
        self.is_saving
    }

    /// The dialog that adds an account at `provider`.
    fn add(provider: ProviderRef) -> Self {
        Self {
            editing: None,
            asked: None,
            provider,
            is_provider_list_open: false,
            key: KeyPlaceField::new(None),
            error: None,
            is_saving: false,
        }
    }

    /// The dialog that edits the key of `account`, an account the user
    /// added, at its provider and its place.
    fn edit(account: Account, key: &KeyRef) -> Self {
        let mut dialog = Self::add(account.provider().clone());

        dialog.key = KeyPlaceField::new(Some(key));
        dialog.editing = Some(account);
        dialog
    }

    /// The dialog that gives `asked`, the key a plugin part asks for, with
    /// the place `saved` the user gave before, if any.
    fn ask(asked: AskedKey, saved: Option<&KeyRef>) -> Self {
        let mut dialog = Self::add(asked.account.provider().clone());

        dialog.key = KeyPlaceField::new(saved);
        dialog.asked = Some(asked);
        dialog
    }

    /// Show `text` as why the save failed, and let the dialog save again.
    pub(crate) fn fail(&mut self, text: String) {
        self.error = Some(text);
        self.is_saving = false;
    }
}

/// A key a plugin part asks for with a need, given for an account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AskedKey {
    account: Account,
    plugin: String,
    ask: KeyAsk,
}

impl AskedKey {
    /// The name of the plugin that asks.
    pub(crate) fn plugin(&self) -> &str {
        &self.plugin
    }

    /// The key the part asks for.
    pub(crate) fn ask(&self) -> &KeyAsk {
        &self.ask
    }
}

/// Open the dialog that adds an account, at the first provider the
/// registry offers; with none offered it does not open.
pub(crate) fn open_add(state: &mut AgentsState) -> Vec<AgentsOutcome> {
    let first = state.registry().addable_providers().first().copied();
    let Some(first) = first else {
        return Vec::new();
    };

    let provider = ProviderRef::Id(first.id().clone());
    state.open_add_account(AddAccountDialog::add(provider));
    Vec::new()
}

/// Open the dialog that edits the key of `account`. Only an account the
/// user added has one.
pub(crate) fn open_edit_key(
    state: &mut AgentsState,
    account: Account,
) -> Vec<AgentsOutcome> {
    let Some(key) = account.added_key().cloned() else {
        return Vec::new();
    };

    state.open_add_account(AddAccountDialog::edit(account, &key));
    Vec::new()
}

/// Open the dialog of the key `ask` the plugin named `plugin` asks for,
/// given for `account`, with the place given before under its slot. An
/// account `agents.json` can not keep, such as a key OTTY found, takes no
/// key.
pub(crate) fn open_ask(
    state: &mut AgentsState,
    account: Account,
    plugin: String,
    ask: KeyAsk,
) -> Vec<AgentsOutcome> {
    if !account.can_be_saved() {
        return Vec::new();
    }
    let given = state.profiles().given_keys_of(&account);
    let saved = given.iter().find(|given| given.slot() == ask.slot());

    let asked = AskedKey {
        account,
        plugin,
        ask,
    };
    let dialog = AddAccountDialog::ask(asked, saved.map(|given| given.key()));
    state.open_add_account(dialog);
    Vec::new()
}

/// Remove of the menu of an account the user added: forget it and its
/// name in `agents.json`. Its page gives way to All accounts. An account
/// OTTY found can not be removed.
pub(crate) fn remove(
    state: &mut AgentsState,
    account: &Account,
) -> Vec<AgentsOutcome> {
    if account.added_key().is_none() {
        return Vec::new();
    }
    let Some(saved) = state.profiles().saved() else {
        return vec![unread_file()];
    };

    let changed = saved.clone().without_added_account(account);
    let is_on_screen = state
        .selected_account()
        .is_some_and(|selected| selected.is_same_account(account));

    if is_on_screen {
        state.select_all_accounts();
    }
    state.profiles_mut().apply_saved(changed.clone());
    vec![save(changed)]
}

/// Reduce what the dialog was used for. Nothing happens while it is
/// closed.
pub(crate) fn reduce(
    state: &mut AgentsState,
    intent: AddAccountIntent,
) -> Vec<AgentsOutcome> {
    if let AddAccountIntent::Submitted = intent {
        return submit(state);
    }
    if let AddAccountIntent::Cancelled = intent {
        let _ = state.take_add_account();
        return Vec::new();
    }
    if let AddAccountIntent::PageOpened = intent {
        let page = state
            .add_account()
            .and_then(AddAccountDialog::asked)
            .and_then(|asked| asked.ask().page());
        return page
            .map(|page| AgentsOutcome::OpenPage(page.to_string()))
            .into_iter()
            .collect();
    }
    let registry = std::sync::Arc::clone(state.registry_arc());
    let Some(dialog) = state.add_account_mut() else {
        return Vec::new();
    };

    match intent {
        AddAccountIntent::ProviderListToggled => {
            dialog.is_provider_list_open = dialog.editing.is_none()
                && dialog.asked.is_none()
                && !dialog.is_provider_list_open;
        },
        AddAccountIntent::ProviderPicked(provider) => {
            let is_fixed = dialog.editing.is_some() || dialog.asked.is_some();
            if !is_fixed && is_addable(&registry, &provider) {
                dialog.provider = provider;
            }
            dialog.is_provider_list_open = false;
        },
        AddAccountIntent::KeyChoicePicked(choice) => dialog.key.pick(choice),
        AddAccountIntent::KeyChanged(text) => dialog.key.set_text(text),
        _ => {},
    }
    dialog.error = None;

    Vec::new()
}

/// Close the dialog after its save. Nothing happens while no dialog
/// waits for a save.
pub(crate) fn close_after_save(state: &mut AgentsState) {
    if state.add_account().is_some_and(AddAccountDialog::is_saving) {
        let _ = state.take_add_account();
    }
}

/// Whether the dialog of `state` can save: no save is on its way and the
/// place of the key is typed and well formed.
pub(crate) fn can_submit(state: &AgentsState) -> bool {
    let Some(dialog) = state.add_account() else {
        return false;
    };

    let home = state.env().home();
    !dialog.is_saving && matches!(dialog.key.key_place(home), Ok(Some(_)))
}

/// Save the account: the new content of `agents.json` shows at once, the
/// account's page opens, and the content is written. A place another
/// account the user added has stays in the dialog with why.
fn submit(state: &mut AgentsState) -> Vec<AgentsOutcome> {
    if !can_submit(state) {
        return Vec::new();
    }
    let Some(dialog) = state.add_account() else {
        return Vec::new();
    };
    let Ok(Some(key)) = dialog.key.key_place(state.env().home()) else {
        return Vec::new();
    };
    let Some(saved) = state.profiles().saved() else {
        return vec![unread_file()];
    };
    if let Some(asked) = dialog.asked.clone() {
        return submit_asked(state, &asked, key);
    }

    let account = Account::added(dialog.provider.clone(), key.clone());
    let is_own_place = dialog
        .editing
        .as_ref()
        .is_some_and(|editing| editing.is_same_account(&account));
    let is_taken = !is_own_place
        && saved
            .added_accounts()
            .iter()
            .any(|added| added.is_same_account(&account));
    if is_taken {
        let text = i18n::t(Key::AgentsErrorAccountAdded).to_string();
        if let Some(dialog) = state.add_account_mut() {
            dialog.error = Some(text);
        }
        return Vec::new();
    }

    let changed = match &dialog.editing {
        Some(editing) => saved.clone().with_added_key(editing, key),
        None => saved
            .clone()
            .with_account_added(dialog.provider.clone(), key),
    };

    if let Some(dialog) = state.add_account_mut() {
        dialog.is_saving = true;
        dialog.error = None;
    }
    state.profiles_mut().apply_saved(changed.clone());
    state.select_account(account);
    vec![save(changed)]
}

/// Save `key` as the key given for the account of `asked` under the
/// part's slot. The account is read again with it: its earlier read and
/// its need go.
fn submit_asked(
    state: &mut AgentsState,
    asked: &AskedKey,
    key: KeyRef,
) -> Vec<AgentsOutcome> {
    let Some(saved) = state.profiles().saved() else {
        return vec![unread_file()];
    };

    let changed = saved.clone().with_given_key_of(
        &asked.account,
        asked.ask.slot(),
        Some(key),
    );

    if let Some(dialog) = state.add_account_mut() {
        dialog.is_saving = true;
        dialog.error = None;
    }
    state.profiles_mut().apply_saved(changed.clone());
    state.profiles_mut().forget_own_read(&asked.account);
    vec![save(changed)]
}

/// Whether Add account offers `provider` in `registry`.
fn is_addable(registry: &Registry, provider: &ProviderRef) -> bool {
    registry
        .addable_providers()
        .iter()
        .any(|entry| provider.as_id() == Some(entry.id()))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use otty_agents::{GivenKey, SavedProfiles};

    use super::*;
    use crate::event::{AccountAction, AgentsIntent};
    use crate::need::NeedAction;
    use crate::reducer::reduce as reduce_all;
    use crate::testing::{
        api_key, bill_key_needed, chatgpt_login, claude, codex, connected,
        key_ask, now, open_account_menu, organization,
        profiles_with_accounts_loaded, with_accounts,
    };
    use crate::types::LoadedBill;

    fn send(
        state: &mut AgentsState,
        intent: AddAccountIntent,
    ) -> Vec<AgentsOutcome> {
        reduce_all(state, AgentsIntent::AddAccount(intent))
    }

    fn state_with(saved: SavedProfiles) -> AgentsState {
        with_accounts(&[(codex(), connected(), vec![chatgpt_login()])], saved)
    }

    fn deepseek_file() -> KeyRef {
        KeyRef::File(PathBuf::from("/home/user/.config/deepseek-key"))
    }

    fn saved_of(outcomes: &[AgentsOutcome]) -> Option<&SavedProfiles> {
        outcomes.iter().find_map(|outcome| match outcome {
            AgentsOutcome::SaveProfiles { saved, .. } => Some(saved),
            _ => None,
        })
    }

    /// The owner's check: add a DeepSeek key file; only its path is saved.
    #[test]
    fn an_added_key_file_is_saved_by_its_path_and_its_page_opens() {
        let mut state = state_with(SavedProfiles::default());
        let _ = reduce_all(&mut state, AgentsIntent::OpenAddAccount);
        let _ = send(
            &mut state,
            AddAccountIntent::ProviderPicked(ProviderRef::of_id("deepseek")),
        );

        let _ = send(
            &mut state,
            AddAccountIntent::KeyChanged(String::from(
                "~/.config/deepseek-key",
            )),
        );
        let outcomes = send(&mut state, AddAccountIntent::Submitted);

        let saved = saved_of(&outcomes).expect("the account is saved");
        let added =
            Account::added(ProviderRef::of_id("deepseek"), deepseek_file());
        assert_eq!(saved.added_accounts(), std::slice::from_ref(&added));
        assert_eq!(state.selected_account(), Some(added));
        assert!(state.add_account().is_some_and(AddAccountDialog::is_saving));

        let _ = reduce_all(&mut state, AgentsIntent::ProfilesSaved(Ok(())));

        assert_eq!(state.add_account(), None);
    }

    #[test]
    fn an_organization_is_added_with_its_key_variable() {
        let mut state = state_with(SavedProfiles::default());
        let _ = reduce_all(&mut state, AgentsIntent::OpenAddAccount);

        let _ = send(&mut state, AddAccountIntent::ProviderListToggled);
        let _ = send(
            &mut state,
            AddAccountIntent::ProviderPicked(ProviderRef::of_id("openai")),
        );
        let _ = send(
            &mut state,
            AddAccountIntent::KeyChoicePicked(KeyPlaceChoice::Variable),
        );
        let _ = send(
            &mut state,
            AddAccountIntent::KeyChanged(String::from("OPENAI_BILL_KEY")),
        );
        let outcomes = send(&mut state, AddAccountIntent::Submitted);

        assert_eq!(
            saved_of(&outcomes).map(SavedProfiles::added_accounts),
            Some(vec![Account::added(
                ProviderRef::of_id("openai"),
                KeyRef::EnvVar(String::from("OPENAI_BILL_KEY"))
            )])
        );
    }

    #[test]
    fn nothing_is_saved_until_a_full_path_or_a_variable_name_is_typed() {
        let mut state = state_with(SavedProfiles::default());
        let _ = reduce_all(&mut state, AgentsIntent::OpenAddAccount);

        let empty = send(&mut state, AddAccountIntent::Submitted);
        let _ = send(
            &mut state,
            AddAccountIntent::KeyChanged(String::from("key")),
        );
        let not_full = send(&mut state, AddAccountIntent::Submitted);

        assert!(empty.is_empty());
        assert!(not_full.is_empty());
        assert!(!can_submit(&state));
    }

    #[test]
    fn a_place_already_added_is_not_added_twice() {
        let saved = SavedProfiles::default().with_account_added(
            ProviderRef::of_id("deepseek"),
            deepseek_file(),
        );
        let mut state = state_with(saved);
        let _ = reduce_all(&mut state, AgentsIntent::OpenAddAccount);
        let _ = send(
            &mut state,
            AddAccountIntent::ProviderPicked(ProviderRef::of_id("deepseek")),
        );
        let _ = send(
            &mut state,
            AddAccountIntent::KeyChanged(String::from(
                "/home/user/.config/deepseek-key",
            )),
        );

        let outcomes = send(&mut state, AddAccountIntent::Submitted);

        assert!(outcomes.is_empty());
        assert_eq!(
            state.add_account().and_then(AddAccountDialog::error),
            Some(i18n::t(Key::AgentsErrorAccountAdded))
        );
    }

    #[test]
    fn edit_key_moves_the_account_and_keeps_its_page() {
        let added =
            Account::added(ProviderRef::of_id("deepseek"), deepseek_file());
        let saved = SavedProfiles::default()
            .with_account_added(ProviderRef::of_id("deepseek"), deepseek_file())
            .with_account_name(&added, Some(String::from("Mine")));
        let mut state = state_with(saved);
        state.select_account(added.clone());
        open_account_menu(&mut state, added.clone());
        let _ = reduce_all(
            &mut state,
            AgentsIntent::AccountMenu(AccountAction::EditKey),
        );

        let _ = send(
            &mut state,
            AddAccountIntent::ProviderPicked(ProviderRef::of_id("openrouter")),
        );
        let _ = send(
            &mut state,
            AddAccountIntent::KeyChoicePicked(KeyPlaceChoice::Variable),
        );
        let _ = send(
            &mut state,
            AddAccountIntent::KeyChanged(String::from("DEEPSEEK_API_KEY")),
        );
        let outcomes = send(&mut state, AddAccountIntent::Submitted);

        let moved = Account::added(
            ProviderRef::of_id("deepseek"),
            KeyRef::EnvVar(String::from("DEEPSEEK_API_KEY")),
        );
        let saved = saved_of(&outcomes).expect("the key is saved");
        assert_eq!(saved.added_accounts(), std::slice::from_ref(&moved));
        assert_eq!(saved.account_name(&moved), Some("Mine"));
        assert_eq!(state.selected_account(), Some(moved));
    }

    #[test]
    fn remove_forgets_the_account_and_shows_all_accounts() {
        let added =
            Account::added(ProviderRef::of_id("deepseek"), deepseek_file());
        let saved = SavedProfiles::default().with_account_added(
            ProviderRef::of_id("deepseek"),
            deepseek_file(),
        );
        let mut state = state_with(saved);
        state.select_account(added.clone());
        open_account_menu(&mut state, added);

        let outcomes = reduce_all(
            &mut state,
            AgentsIntent::AccountMenu(AccountAction::Remove),
        );

        assert_eq!(
            saved_of(&outcomes).map(SavedProfiles::added_accounts),
            Some(Vec::new())
        );
        assert!(state.is_all_accounts_selected());
        assert_eq!(state.profiles().all_accounts(), [chatgpt_login()]);
    }

    #[test]
    fn a_found_account_has_no_key_to_edit_and_can_not_be_removed() {
        let mut state = state_with(SavedProfiles::default());

        let edit = open_edit_key(&mut state, chatgpt_login());
        let removed = remove(&mut state, &chatgpt_login());

        assert!(edit.is_empty());
        assert_eq!(state.add_account(), None);
        assert!(removed.is_empty());
    }

    #[test]
    fn a_failed_save_stays_in_the_dialog() {
        let mut state = state_with(SavedProfiles::default());
        let _ = reduce_all(&mut state, AgentsIntent::OpenAddAccount);
        let _ = send(
            &mut state,
            AddAccountIntent::KeyChanged(String::from(
                "~/.config/deepseek-key",
            )),
        );
        let _ = send(&mut state, AddAccountIntent::Submitted);

        let _ = reduce_all(
            &mut state,
            AgentsIntent::ProfilesSaved(Err(String::from("disk full"))),
        );

        let dialog = state.add_account().expect("the dialog stays");
        assert!(!dialog.is_saving());
        assert!(
            dialog
                .error()
                .is_some_and(|error| error.contains("disk full"))
        );
    }

    /// Issue 20: the key given in the field a part's `ask-key` opened is
    /// kept by its place under the part's slot, and the next read of the
    /// bill gets it with that slot.
    #[test]
    fn a_key_given_for_a_slot_reaches_the_next_bill_read() {
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
        let variable = KeyRef::EnvVar(String::from("ANTHROPIC_ORG_KEY"));
        let _ = reduce_all(
            &mut state,
            AgentsIntent::FixNeed(NeedAction::AskKey {
                account: organization(),
                plugin: String::from("Anthropic"),
                ask: key_ask(),
            }),
        );
        let _ = send(
            &mut state,
            AddAccountIntent::KeyChoicePicked(KeyPlaceChoice::Variable),
        );
        let _ = send(
            &mut state,
            AddAccountIntent::KeyChanged(String::from("ANTHROPIC_ORG_KEY")),
        );

        let submitted = send(&mut state, AddAccountIntent::Submitted);
        let saved = saved_of(&submitted).expect("the key is saved").clone();
        let reload =
            reduce_all(&mut state, AgentsIntent::ProfilesSaved(Ok(())));
        let reloaded = reduce_all(
            &mut state,
            profiles_with_accounts_loaded(&profiles, saved.clone()),
        );

        let given = vec![GivenKey::new("admin", variable)];
        assert_eq!(saved.given_keys_of(&organization()), given.as_slice());
        assert!(saved.added_accounts().is_empty(), "no account is added");
        assert_eq!(state.add_account(), None);
        assert_eq!(reload, [AgentsOutcome::LoadProfiles]);
        assert!(reloaded.contains(&AgentsOutcome::LoadBill {
            profile: claude(),
            organization: organization(),
            given,
        }));
    }
}
