//! The user's own choices for the accounts in `agents.json` (spec
//! 2026-11-19, Decision 7): a name, whether the account is hidden, where
//! the admin key of an organization is kept, and the accounts the user
//! added by hand, by where their keys are kept.

use crate::account::{Account, AccountIdentity};
use crate::admin_key::AdminKeyRef;
use crate::provider::ProviderRef;

/// The choices for one account, kept by its provider and an identity that
/// names it for good: an account id, an organization id, an email or the
/// place of the key of an account the user added, never the fingerprint of
/// a key ([`Account::can_be_saved`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SavedAccount {
    pub(super) provider: ProviderRef,
    pub(super) identity: AccountIdentity,
    pub(super) name: Option<String>,
    pub(super) is_hidden: bool,
    pub(super) admin_key: Option<AdminKeyRef>,
}

impl SavedAccount {
    /// No choice yet for `account`; `None` for an account that can not be
    /// saved.
    pub(super) fn new(account: &Account) -> Option<Self> {
        if !account.can_be_saved() {
            return None;
        }

        Some(Self {
            provider: account.provider().clone(),
            identity: account.identity().clone(),
            name: None,
            is_hidden: false,
            admin_key: None,
        })
    }

    /// Whether these are the choices for `account`.
    pub(super) fn is_of(&self, account: &Account) -> bool {
        *account.provider() == self.provider
            && *account.identity() == self.identity
    }

    /// Whether the entry is an account the user added: it is kept for
    /// itself, with or without a name.
    pub(super) fn is_added(&self) -> bool {
        matches!(self.identity, AccountIdentity::AddedKey(_))
    }

    /// The account the user added that this entry keeps; `None` for the
    /// choices of an account OTTY finds.
    pub(super) fn added_account(&self) -> Option<Account> {
        let AccountIdentity::AddedKey(key) = &self.identity else {
            return None;
        };

        Some(Account::added(self.provider.clone(), key.clone()))
    }

    /// Whether the entry keeps nothing: an account OTTY finds with no
    /// name, not hidden and no admin key. Such an entry is not written.
    pub(super) fn is_empty(&self) -> bool {
        !self.is_added()
            && self.name.is_none()
            && !self.is_hidden
            && self.admin_key.is_none()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::account::{
        Account, AccountIdentity, AccountKind, AccountLogin, FoundIn,
    };
    use crate::admin_key::AdminKeyRef;
    use crate::provider::ProviderRef;
    use crate::saved_profiles::SavedProfiles;
    use crate::testing::{chatgpt_login, key_account, organization};

    fn load(text: &str) -> SavedProfiles {
        SavedProfiles::from_json(text)
            .expect("the text is the layout of agents.json")
    }

    /// A made-up Claude login whose files name only its email.
    fn email_login(email: &str) -> Account {
        Account::new(
            ProviderRef::of_id("anthropic"),
            AccountIdentity::Email(email.to_string()),
            AccountLogin::Email(email.to_string()),
            AccountKind::Limits,
            FoundIn::ProfileFolder(PathBuf::from("/home/user/.claude")),
        )
    }

    fn by_file() -> AdminKeyRef {
        AdminKeyRef::File(PathBuf::from("/home/user/.config/admin-key"))
    }

    fn named_and_hidden() -> SavedProfiles {
        SavedProfiles::default()
            .with_account_name(
                &chatgpt_login("made-up-one"),
                Some(String::from("ChatGPT · Home")),
            )
            .with_account_hidden(&chatgpt_login("made-up-two"), true)
            .with_account_name(
                &organization("made-up-org"),
                Some(String::from("Acme")),
            )
            .with_account_hidden(&email_login("me@example.com"), true)
    }

    /// Spec 2026-11-19, Decision 7: the names and the hidden accounts are
    /// kept in `agents.json`.
    #[test]
    fn names_and_hidden_accounts_are_saved_and_read_back() {
        let saved = named_and_hidden();

        let text = saved.to_json().expect("the paths are text");
        let loaded = load(&text);

        assert_eq!(loaded, saved);
        assert_eq!(
            loaded.account_name(&chatgpt_login("made-up-one")),
            Some("ChatGPT · Home")
        );
        assert_eq!(
            loaded.account_name(&organization("made-up-org")),
            Some("Acme")
        );
        assert_eq!(loaded.account_name(&chatgpt_login("made-up-two")), None);
        assert!(loaded.is_account_hidden(&chatgpt_login("made-up-two")));
        assert!(loaded.is_account_hidden(&email_login("me@example.com")));
        assert!(!loaded.is_account_hidden(&chatgpt_login("made-up-one")));
    }

    #[test]
    fn the_text_keeps_each_account_by_its_id_its_organization_or_its_email() {
        let text = r#"{
            "accounts": [
                { "provider": "openai", "account_id": "made-up-one", "name": "ChatGPT · Home" },
                { "provider": "openai", "account_id": "made-up-two", "hidden": true },
                { "provider": "anthropic", "organization_id": "made-up-org", "name": "Acme" },
                { "provider": "anthropic", "email": "me@example.com", "hidden": true }
            ]
        }"#;

        assert_eq!(load(text), named_and_hidden());
    }

    #[test]
    fn a_name_and_an_admin_key_of_one_organization_share_its_entry() {
        let organization = organization("made-up-org");
        let saved = SavedProfiles::default()
            .with_admin_key_of(&organization, Some(by_file()))
            .with_account_name(&organization, Some(String::from("Acme")));

        let text = saved.to_json().expect("the paths are text");
        let loaded = load(&text);

        assert_eq!(text.matches("made-up-org").count(), 1, "{text}");
        assert_eq!(loaded.admin_key_of(&organization), Some(&by_file()));
        assert_eq!(loaded.account_name(&organization), Some("Acme"));
    }

    /// The fingerprint of a key is not made to be kept: an account known
    /// only by its key gets no name and is never hidden in the file.
    #[test]
    fn an_account_known_only_by_its_key_is_never_saved() {
        let key = key_account(
            ProviderRef::of_id("deepseek"),
            "sk-made-up-deepseek-key",
        );

        let saved = SavedProfiles::default()
            .with_account_name(&key, Some(String::from("Mine")))
            .with_account_hidden(&key, true);

        assert!(!key.can_be_saved());
        assert_eq!(saved, SavedProfiles::default());
        assert_eq!(saved.account_name(&key), None);
        assert!(!saved.is_account_hidden(&key));
    }

    #[test]
    fn no_name_and_show_again_leave_no_entry() {
        let login = chatgpt_login("made-up-one");
        let saved = SavedProfiles::default()
            .with_account_name(&login, Some(String::from("Home")))
            .with_account_hidden(&login, true);

        let cleared = saved
            .with_account_name(&login, None)
            .with_account_hidden(&login, false);
        let text = cleared.to_json().expect("the paths are text");

        assert_eq!(cleared, SavedProfiles::default());
        assert!(!text.contains("accounts"), "{text}");
    }

    #[test]
    fn showing_an_organization_again_keeps_its_name_and_its_admin_key() {
        let organization = organization("made-up-org");
        let saved = SavedProfiles::default()
            .with_admin_key_of(&organization, Some(by_file()))
            .with_account_name(&organization, Some(String::from("Acme")))
            .with_account_hidden(&organization, true);

        let shown = saved.with_account_hidden(&organization, false);

        assert!(!shown.is_account_hidden(&organization));
        assert_eq!(shown.account_name(&organization), Some("Acme"));
        assert_eq!(shown.admin_key_of(&organization), Some(&by_file()));
    }

    fn deepseek_file() -> AdminKeyRef {
        AdminKeyRef::File(PathBuf::from("/home/user/.config/deepseek-key"))
    }

    fn openai_variable() -> AdminKeyRef {
        AdminKeyRef::EnvVar(String::from("OPENAI_ADMIN_KEY"))
    }

    /// Spec 2026-11-19, Decision 7: an account added by hand is saved by
    /// its provider and the place of its key, never the key.
    #[test]
    fn an_added_account_is_saved_with_the_place_of_its_key_only() {
        let saved = SavedProfiles::default()
            .with_account_added(ProviderRef::of_id("deepseek"), deepseek_file())
            .with_account_added(
                ProviderRef::of_id("openai"),
                openai_variable(),
            );

        let text = saved.to_json().expect("the paths are text");
        let loaded = load(&text);

        assert_eq!(loaded, saved);
        assert_eq!(
            loaded.added_accounts(),
            [
                Account::added(ProviderRef::of_id("deepseek"), deepseek_file()),
                Account::added(ProviderRef::of_id("openai"), openai_variable()),
            ]
        );
        assert!(text.contains(r#""file": "/home/user/.config/deepseek-key""#));
        assert!(text.contains(r#""env_var": "OPENAI_ADMIN_KEY""#));
        assert!(!text.contains("fingerprint"), "{text}");
    }

    #[test]
    fn an_account_added_twice_is_kept_once() {
        let saved = SavedProfiles::default()
            .with_account_added(ProviderRef::of_id("deepseek"), deepseek_file())
            .with_account_added(
                ProviderRef::of_id("deepseek"),
                deepseek_file(),
            );

        assert_eq!(saved.added_accounts().len(), 1);
    }

    #[test]
    fn remove_forgets_an_added_account_and_its_choices() {
        let added =
            Account::added(ProviderRef::of_id("deepseek"), deepseek_file());
        let saved = SavedProfiles::default()
            .with_account_added(ProviderRef::of_id("deepseek"), deepseek_file())
            .with_account_name(&added, Some(String::from("Mine")))
            .with_account_hidden(&added, true);

        let removed = saved.without_added_account(&added);
        let text = removed.to_json().expect("the paths are text");

        assert_eq!(removed, SavedProfiles::default());
        assert!(!text.contains("deepseek"), "{text}");
    }

    /// Remove is for an account the user added: a found account keeps its
    /// choices.
    #[test]
    fn remove_leaves_a_found_account_as_it_is() {
        let login = chatgpt_login("made-up-one");
        let saved = SavedProfiles::default()
            .with_account_name(&login, Some(String::from("Home")));

        let removed = saved.clone().without_added_account(&login);

        assert_eq!(removed, saved);
    }

    #[test]
    fn edit_key_moves_the_account_to_its_new_place_with_its_name() {
        let added =
            Account::added(ProviderRef::of_id("deepseek"), deepseek_file());
        let variable = AdminKeyRef::EnvVar(String::from("DEEPSEEK_API_KEY"));
        let saved = SavedProfiles::default()
            .with_account_added(ProviderRef::of_id("deepseek"), deepseek_file())
            .with_account_name(&added, Some(String::from("Mine")))
            .with_account_hidden(&added, true);

        let edited = saved.with_added_key(&added, variable.clone());
        let moved = Account::added(ProviderRef::of_id("deepseek"), variable);

        assert_eq!(edited.added_accounts(), std::slice::from_ref(&moved));
        assert_eq!(edited.account_name(&moved), Some("Mine"));
        assert!(edited.is_account_hidden(&moved));
        assert_eq!(edited.account_name(&added), None);
    }

    /// An added account with no place for its key is left out; one of a
    /// provider no entry of the registry names is kept as it is (spec
    /// 2026-11-19-agent-plugins, "`agents.json`").
    #[test]
    fn an_added_account_with_no_key_is_left_out() {
        let text = r#"{
            "accounts": [
                { "provider": "future", "key": { "env_var": "K" } },
                { "provider": "deepseek", "key": {} },
                { "provider": "moonshot", "key": { "env_var": "MOONSHOT_API_KEY" } }
            ]
        }"#;

        let loaded = load(text);

        assert_eq!(
            loaded.added_accounts(),
            [
                Account::added(
                    ProviderRef::of_id("future"),
                    AdminKeyRef::EnvVar(String::from("K"))
                ),
                Account::added(
                    ProviderRef::of_id("moonshot"),
                    AdminKeyRef::EnvVar(String::from("MOONSHOT_API_KEY"))
                )
            ]
        );
    }

    /// An entry that names no account or no provider, or with an admin
    /// key on an account that is not an organization, is left out.
    #[test]
    fn an_entry_that_names_no_account_is_left_out() {
        let text = r#"{
            "accounts": [
                { "provider": "anthropic", "name": "Nobody" },
                { "provider": "", "account_id": "x", "hidden": true },
                { "provider": "openai", "account_id": "y", "admin_key": { "file": "/k" } }
            ]
        }"#;

        assert_eq!(load(text), SavedProfiles::default());
    }
}
