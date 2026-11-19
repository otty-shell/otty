//! The Rename dialog of an account or a profile (spec 2026-11-19,
//! Decision 7): one field with the name. The name of an account is the
//! user's own, kept by the account in `agents.json`; an empty name, or its
//! own, gives it back its provider and login. The name of a profile must
//! be its own among the profiles and is saved as an edit of the profile.
//! The dialog waits for the save, and a failed save shows in it.

use otty_agents::{Account, Profile, check_profile_name};
use otty_app_i18n::{self as i18n, Key};

use super::event::AgentsOutcome;
use super::model::own_account_name;
use super::profile_actions;
use super::profile_changes::{self, ProfileEdit};
use super::state::AgentsState;

/// What the Rename dialog was used for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenameIntent {
    /// The name typed in the field changed.
    NameChanged(String),
    /// Save or Enter: save the name.
    Submitted,
    /// Cancel, the cross or Esc: close the dialog and save nothing.
    Cancelled,
}

/// What the Rename dialog names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RenameTarget {
    /// An account, known by an id, an organization or an email.
    Account(Account),
    /// A profile.
    Profile(Profile),
}

/// The open Rename dialog: what it names, the name typed, and how its save
/// went.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RenameDialog {
    target: RenameTarget,
    name: String,
    error: Option<String>,
    is_saving: bool,
}

impl RenameDialog {
    /// What the dialog names.
    pub(crate) fn target(&self) -> &RenameTarget {
        &self.target
    }

    /// The name typed in the field.
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    /// Why the name was not saved, ready to show.
    pub(crate) fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Whether the dialog waits for its save.
    pub(crate) fn is_saving(&self) -> bool {
        self.is_saving
    }

    /// Whether Save acts: no save is on its way, and a profile has a name.
    /// An account may be left with no name of its own.
    pub(crate) fn can_save(&self) -> bool {
        let has_name = match &self.target {
            RenameTarget::Account(_) => true,
            RenameTarget::Profile(_) => !self.name.trim().is_empty(),
        };

        !self.is_saving && has_name
    }

    /// The dialog of `target`, with `name` in the field.
    fn new(target: RenameTarget, name: String) -> Self {
        Self {
            target,
            name,
            error: None,
            is_saving: false,
        }
    }

    /// Show `text` as why the save failed, and let Save act again.
    pub(crate) fn fail(&mut self, text: String) {
        self.error = Some(text);
        self.is_saving = false;
    }
}

/// Open the dialog of `account` with the name it is shown by; its field
/// takes the keyboard. An account known only by its key is not renamed:
/// nothing taken from a key is kept.
pub(crate) fn open_for_account(
    state: &mut AgentsState,
    account: Account,
) -> Vec<AgentsOutcome> {
    if !account.can_be_saved() {
        return Vec::new();
    }

    let saved = state.profiles().account_name(&account);
    let name = saved.map_or_else(|| own_account_name(&account), String::from);
    state.open_rename(RenameDialog::new(RenameTarget::Account(account), name));
    vec![AgentsOutcome::FocusRenameField]
}

/// Open the dialog of `profile` with its name; its field takes the
/// keyboard.
pub(crate) fn open_for_profile(
    state: &mut AgentsState,
    profile: Profile,
) -> Vec<AgentsOutcome> {
    let name = profile.name().to_string();

    state.open_rename(RenameDialog::new(RenameTarget::Profile(profile), name));
    vec![AgentsOutcome::FocusRenameField]
}

/// Reduce what the dialog was used for. Nothing happens while it is
/// closed.
pub(crate) fn reduce(
    state: &mut AgentsState,
    intent: RenameIntent,
) -> Vec<AgentsOutcome> {
    match intent {
        RenameIntent::NameChanged(name) => {
            if let Some(dialog) = state.rename_mut() {
                dialog.name = name;
                dialog.error = None;
            }
            Vec::new()
        },
        RenameIntent::Submitted => submit(state),
        RenameIntent::Cancelled => {
            let _ = state.take_rename();
            Vec::new()
        },
    }
}

/// Close the dialog after its save: a renamed profile that was on screen
/// stays on screen under its new name. Nothing happens while no dialog
/// waits for a save.
pub(crate) fn close_after_save(state: &mut AgentsState) {
    if !state.rename().is_some_and(RenameDialog::is_saving) {
        return;
    }
    let Some(dialog) = state.take_rename() else {
        return;
    };

    if let RenameTarget::Profile(profile) = &dialog.target {
        let is_on_screen = state
            .selected_profile()
            .map(|shown| shown.profile())
            .or(state.selected_hidden_profile())
            .is_some_and(|selected| selected == profile);
        if is_on_screen {
            state.select_named(profile.agent(), dialog.name.trim());
        }
    }
}

/// Save the name typed: the new content of `agents.json` shows at once
/// and is written. A profile name another profile has stays in the dialog
/// with why.
fn submit(state: &mut AgentsState) -> Vec<AgentsOutcome> {
    let Some(dialog) = state.rename() else {
        return Vec::new();
    };
    if !dialog.can_save() {
        return Vec::new();
    }
    let Some(saved) = state.profiles().saved() else {
        return vec![profile_actions::unread_file()];
    };

    let name = dialog.name.trim().to_string();
    let changed = match &dialog.target {
        RenameTarget::Account(account) => {
            let is_own = name.is_empty() || name == own_account_name(account);
            let name = (!is_own).then_some(name);
            saved.clone().with_account_name(account, name)
        },
        RenameTarget::Profile(profile) => {
            // Save is off with no name, so a name that can not be saved is
            // one another profile has.
            let others = state.profiles().all();
            if check_profile_name(&name, Some(profile), &others).is_err() {
                let text = i18n::t(Key::AgentsErrorNameTaken).to_string();
                if let Some(dialog) = state.rename_mut() {
                    dialog.fail(text);
                }
                return Vec::new();
            }

            let edit = ProfileEdit {
                name,
                config_dir: profile.config_dir().to_path_buf(),
                admin_key: None,
            };
            let agent_name = state.agent_name(profile.agent());
            profile_changes::edited(saved, agent_name, profile, edit)
        },
    };

    state.profiles_mut().apply_saved(changed.clone());
    if let Some(dialog) = state.rename_mut() {
        dialog.is_saving = true;
        dialog.error = None;
    }
    vec![profile_actions::save(changed)]
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use otty_agents::{
        Account, AccountIdentity, AccountKind, AccountLogin, FoundIn,
        KeyFingerprint, Profile, Provider, SavedProfiles,
    };
    use otty_app_i18n::{self as i18n, Key};

    use super::*;
    use crate::event::{
        AccountAction, AgentsIntent, AgentsOutcome, ProfileAction,
    };
    use crate::model::LeftPanelViewModel;
    use crate::reducer::reduce;
    use crate::state::AgentsState;
    use crate::testing::{
        CODEX, chatgpt_login, claude, codex, connected, now, open_account_menu,
        open_profile_menu, profiles_loaded_from, save, with_accounts,
        with_profiles,
    };

    fn with_chatgpt(saved: SavedProfiles) -> AgentsState {
        with_accounts(&[(codex(), connected(), vec![chatgpt_login()])], saved)
    }

    fn rename_account(state: &mut AgentsState, account: Account) {
        open_account_menu(state, account);
        let _ = reduce(state, AgentsIntent::AccountMenu(AccountAction::Rename));
    }

    fn rename_profile(state: &mut AgentsState, profile: Profile) {
        open_profile_menu(state, profile);
        let _ = reduce(state, AgentsIntent::ProfileMenu(ProfileAction::Rename));
    }

    fn type_name(state: &mut AgentsState, name: &str) -> Vec<AgentsOutcome> {
        let _ = reduce(
            state,
            AgentsIntent::Rename(RenameIntent::NameChanged(name.to_string())),
        );

        reduce(state, AgentsIntent::Rename(RenameIntent::Submitted))
    }

    fn account_names(state: &AgentsState) -> Vec<String> {
        LeftPanelViewModel::from_state(state, now())
            .accounts
            .accounts
            .into_iter()
            .map(|row| row.name)
            .collect()
    }

    #[test]
    fn rename_of_an_account_opens_the_dialog_with_its_name() {
        let mut state = with_chatgpt(SavedProfiles::default());

        rename_account(&mut state, chatgpt_login());

        let dialog = state.rename().expect("the dialog is open");
        assert_eq!(dialog.name(), "OpenAI · you@example.com");
        assert!(state.menu().is_none());
    }

    /// Spec 2026-11-19, Decision 7: the user's name is saved and shown in
    /// place of the provider and the login.
    #[test]
    fn a_new_name_of_an_account_is_saved_and_shown() {
        let mut state = with_chatgpt(SavedProfiles::default());
        rename_account(&mut state, chatgpt_login());

        let outcomes = type_name(&mut state, "  ChatGPT · Home ");
        let saved = reduce(&mut state, AgentsIntent::ProfilesSaved(Ok(())));

        assert_eq!(
            outcomes,
            vec![save(SavedProfiles::default().with_account_name(
                &chatgpt_login(),
                Some(String::from("ChatGPT · Home"))
            ))]
        );
        assert!(state.rename().is_none());
        assert_eq!(saved, vec![AgentsOutcome::LoadProfiles]);
        assert_eq!(account_names(&state), ["ChatGPT · Home"]);
    }

    #[test]
    fn an_empty_name_gives_the_account_its_own_name_back() {
        let named = SavedProfiles::default()
            .with_account_name(&chatgpt_login(), Some(String::from("Home")));
        let mut state = with_chatgpt(named);
        rename_account(&mut state, chatgpt_login());

        let outcomes = type_name(&mut state, " ");

        assert_eq!(outcomes, vec![save(SavedProfiles::default())]);
        assert_eq!(account_names(&state), ["OpenAI · you@example.com"]);
    }

    /// A key is known only by its fingerprint, which is not kept, so its
    /// name could not be saved.
    #[test]
    fn an_account_known_only_by_its_key_is_not_renamed() {
        let key = Account::new(
            Provider::DeepSeek,
            AccountIdentity::KeyFingerprint(KeyFingerprint::of(
                "sk-made-up-deepseek-key",
            )),
            AccountLogin::Key(Some(String::from("-key"))),
            AccountKind::Money,
            FoundIn::ProfileFolder(PathBuf::from("/home/user/.claude")),
        );
        let mut state = with_accounts(
            &[(claude(), connected(), vec![key.clone()])],
            SavedProfiles::default(),
        );

        rename_account(&mut state, key);

        assert!(state.rename().is_none());
    }

    #[test]
    fn a_new_name_of_a_profile_is_saved_and_its_page_stays_open() {
        let mut state = with_profiles(&[(codex(), connected())]);
        let _ = reduce(&mut state, AgentsIntent::SelectProfile(codex()));
        rename_profile(&mut state, codex());
        let renamed = Profile::default_for(
            CODEX,
            "Codex · Home",
            PathBuf::from("/home/user/.codex"),
        );
        let expected = SavedProfiles::default().with_default_edited(
            "Codex",
            &codex(),
            renamed.clone(),
        );

        let outcomes = type_name(&mut state, "Codex · Home");
        let _ = reduce(&mut state, AgentsIntent::ProfilesSaved(Ok(())));
        let _ = reduce(
            &mut state,
            profiles_loaded_from(
                &[(renamed.clone(), connected())],
                expected.clone(),
            ),
        );

        assert_eq!(outcomes, vec![save(expected)]);
        assert!(state.rename().is_none());
        assert_eq!(
            state.selected_profile().map(|shown| shown.profile()),
            Some(&renamed)
        );
    }

    #[test]
    fn a_name_another_profile_has_is_not_saved() {
        let mut state =
            with_profiles(&[(claude(), connected()), (codex(), connected())]);
        rename_profile(&mut state, codex());

        let outcomes = type_name(&mut state, "Claude Code");

        assert!(outcomes.is_empty());
        let dialog = state.rename().expect("the dialog stays open");
        assert_eq!(dialog.error(), Some(i18n::t(Key::AgentsErrorNameTaken)));
    }

    #[test]
    fn a_failed_save_stays_in_the_dialog() {
        let mut state = with_chatgpt(SavedProfiles::default());
        rename_account(&mut state, chatgpt_login());
        let _ = type_name(&mut state, "Home");

        let outcomes = reduce(
            &mut state,
            AgentsIntent::ProfilesSaved(Err(String::from("disk full"))),
        );

        assert!(outcomes.is_empty());
        let dialog = state.rename().expect("the dialog stays open");
        assert_eq!(
            dialog.error(),
            Some(
                i18n::with_error(Key::TplAgentsSaveFailed, "disk full")
                    .as_str()
            )
        );
        assert!(!dialog.is_saving());
    }

    #[test]
    fn cancel_closes_the_dialog_and_saves_nothing() {
        let mut state = with_chatgpt(SavedProfiles::default());
        rename_account(&mut state, chatgpt_login());

        let outcomes =
            reduce(&mut state, AgentsIntent::Rename(RenameIntent::Cancelled));

        assert!(outcomes.is_empty());
        assert!(state.rename().is_none());
    }
}
