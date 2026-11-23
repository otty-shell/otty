//! The user's own choices for the accounts in `agents.json` (spec
//! 2026-11-19, Decision 7): a name, whether the account is hidden, and
//! where the keys the user gave for it are kept.

use crate::account::{Account, AccountIdentity};
use crate::given_key::GivenKey;
use crate::provider::ProviderRef;

/// The choices for one account, kept by its provider and an identity that
/// names it for good: an account id, an organization id or an email, never
/// the fingerprint of a key ([`Account::can_be_saved`]). The choices are
/// kept with the plugin that reads the account, as each plugin's reading
/// is its own account (issue 29).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SavedAccount {
    pub(super) provider: ProviderRef,
    pub(super) identity: AccountIdentity,
    pub(super) plugin: Option<String>,
    pub(super) name: Option<String>,
    pub(super) is_hidden: bool,
    pub(super) given_keys: Vec<GivenKey>,
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
            plugin: account.read_by().map(String::from),
            name: None,
            is_hidden: false,
            given_keys: Vec::new(),
        })
    }

    /// Whether these are the choices for `account`: the same provider,
    /// identity and plugin.
    pub(super) fn is_of(&self, account: &Account) -> bool {
        *account.provider() == self.provider
            && *account.identity() == self.identity
            && self.plugin.as_deref() == account.read_by()
    }

    /// Whether the entry keeps nothing: no name, not hidden and no given
    /// key. Such an entry is not written.
    pub(super) fn is_empty(&self) -> bool {
        self.name.is_none() && !self.is_hidden && self.given_keys.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::account::{
        Account, AccountIdentity, AccountKind, AccountLogin, FoundIn,
    };
    use crate::given_key::{GivenKey, KeyRef};
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

    fn by_file() -> KeyRef {
        KeyRef::File(PathBuf::from("/home/user/.config/org-key"))
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
    fn a_name_and_a_given_key_of_one_organization_share_its_entry() {
        let organization = organization("made-up-org");
        let saved = SavedProfiles::default()
            .with_given_key_of(&organization, "admin", Some(by_file()))
            .with_account_name(&organization, Some(String::from("Acme")));

        let text = saved.to_json().expect("the paths are text");
        let loaded = load(&text);

        assert_eq!(text.matches("made-up-org").count(), 1, "{text}");
        assert_eq!(
            loaded.given_keys_of(&organization),
            [GivenKey::new("admin", by_file())]
        );
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
            .with_account_hidden(&key, true)
            .with_given_key_of(&key, "admin", Some(by_file()));

        assert!(!key.can_be_saved());
        assert_eq!(saved, SavedProfiles::default());
        assert_eq!(saved.account_name(&key), None);
        assert!(!saved.is_account_hidden(&key));
        assert!(saved.given_keys_of(&key).is_empty());
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
    fn showing_an_organization_again_keeps_its_name_and_its_given_key() {
        let organization = organization("made-up-org");
        let saved = SavedProfiles::default()
            .with_given_key_of(&organization, "admin", Some(by_file()))
            .with_account_name(&organization, Some(String::from("Acme")))
            .with_account_hidden(&organization, true);

        let shown = saved.with_account_hidden(&organization, false);

        assert!(!shown.is_account_hidden(&organization));
        assert_eq!(shown.account_name(&organization), Some("Acme"));
        assert_eq!(
            shown.given_keys_of(&organization),
            [GivenKey::new("admin", by_file())]
        );
    }

    /// Spec 2026-11-23, Decision 10: an account entry keeps the user's
    /// name, Hide and given keys with no `key`; an entry known only by the
    /// place of a key is not read, and none is written with one.
    #[test]
    fn an_account_entry_keeps_its_choices_with_no_key() {
        let text = r#"{
            "accounts": [
                { "provider": "deepseek", "key": { "env_var": "DEEPSEEK_API_KEY" }, "name": "Mine" },
                {
                    "provider": "anthropic",
                    "organization_id": "made-up-org",
                    "name": "Acme",
                    "hidden": true,
                    "given_keys": [{ "slot": "admin", "file": "/home/user/.config/org-key" }]
                }
            ]
        }"#;
        let organization = organization("made-up-org");

        let loaded = load(text);
        let written = loaded.to_json().expect("the paths are text");

        assert_eq!(
            loaded,
            SavedProfiles::default()
                .with_account_name(&organization, Some(String::from("Acme")))
                .with_account_hidden(&organization, true)
                .with_given_key_of(&organization, "admin", Some(by_file()))
        );
        assert!(!written.contains(r#""key""#), "{written}");
        assert!(!written.contains("deepseek"), "{written}");
    }

    /// Issue 29: a login two plugins read is two accounts, so the user
    /// hides or names each for itself: the choice is kept with the plugin
    /// that reads it.
    #[test]
    fn the_choices_of_a_found_account_are_kept_per_plugin() {
        let read_by = |plugin: &str| {
            chatgpt_login("made-up-one").with_read_by(Some(plugin.to_string()))
        };
        let saved = SavedProfiles::default()
            .with_account_hidden(&read_by("openai-fork"), true)
            .with_account_name(&read_by("openai"), Some("Home".into()));

        let text = saved.to_json().expect("the paths are text");
        let loaded = load(&text);

        assert_eq!(loaded, saved);
        assert!(text.contains(r#""plugin": "openai-fork""#), "{text}");
        assert!(loaded.is_account_hidden(&read_by("openai-fork")));
        assert!(!loaded.is_account_hidden(&read_by("openai")));
        assert_eq!(loaded.account_name(&read_by("openai")), Some("Home"));
        assert_eq!(loaded.account_name(&chatgpt_login("made-up-one")), None);
    }

    /// An entry that names no account or no provider, or whose given key
    /// has no slot or no place, keeps nothing and is left out.
    #[test]
    fn an_entry_that_names_no_account_is_left_out() {
        let text = r#"{
            "accounts": [
                { "provider": "anthropic", "name": "Nobody" },
                { "provider": "", "account_id": "x", "hidden": true },
                { "provider": "openai", "account_id": "y", "given_keys": [{ "slot": "", "file": "/k" }] },
                { "provider": "openai", "account_id": "z", "given_keys": [{ "slot": "admin" }] }
            ]
        }"#;

        assert_eq!(load(text), SavedProfiles::default());
    }
}
