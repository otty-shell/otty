//! The user's own choices for the accounts in `agents.json` (spec
//! 2026-11-19, Decision 7): a name, whether the account is hidden, and
//! where the keys the user gave for it are kept. They are kept by the
//! account's plugin and id (spec 2026-11-23-accounts-apart-from-profiles,
//! "What OTTY stores").

use crate::account::{Account, AccountKey};
use crate::given_key::GivenKey;

/// The choices for one account, kept by its plugin and the part's id of
/// it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SavedAccount {
    pub(super) key: AccountKey,
    pub(super) name: Option<String>,
    pub(super) is_hidden: bool,
    pub(super) given_keys: Vec<GivenKey>,
}

impl SavedAccount {
    /// No choice yet for `account`.
    pub(super) fn new(account: &Account) -> Self {
        Self {
            key: account.key().clone(),
            name: None,
            is_hidden: false,
            given_keys: Vec::new(),
        }
    }

    /// Whether these are the choices for `account`: the same plugin and
    /// id.
    pub(super) fn is_of(&self, account: &Account) -> bool {
        *account.key() == self.key
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

    use crate::account::{Account, AccountKind};
    use crate::given_key::{GivenKey, KeyRef};
    use crate::provider::ProviderId;
    use crate::saved_profiles::SavedProfiles;
    use crate::testing::{chatgpt_login, organization};

    fn load(text: &str) -> SavedProfiles {
        SavedProfiles::from_json(text)
            .expect("the text is the layout of agents.json")
    }

    /// A made-up key of DeepSeek, known by its fingerprint.
    fn key() -> Account {
        Account::new(
            "deepseek",
            ProviderId::new("deepseek"),
            "key:27c8d30e",
            "key ••••d30e",
            AccountKind::Key,
            Vec::new(),
        )
    }

    fn by_file() -> KeyRef {
        KeyRef::File(PathBuf::from("/home/user/.config/org-key"))
    }

    fn named_and_hidden() -> SavedProfiles {
        SavedProfiles::default()
            .with_account_name(
                &chatgpt_login("made-up-one"),
                Some(String::from("ChatGPT (Home)")),
            )
            .with_account_hidden(&chatgpt_login("made-up-two"), true)
            .with_account_name(
                &organization("made-up-org"),
                Some(String::from("Acme")),
            )
            .with_account_hidden(&key(), true)
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
            Some("ChatGPT (Home)")
        );
        assert_eq!(
            loaded.account_name(&organization("made-up-org")),
            Some("Acme")
        );
        assert_eq!(loaded.account_name(&chatgpt_login("made-up-two")), None);
        assert!(loaded.is_account_hidden(&chatgpt_login("made-up-two")));
        assert!(loaded.is_account_hidden(&key()));
        assert!(!loaded.is_account_hidden(&chatgpt_login("made-up-one")));
    }

    /// Spec 2026-11-23-accounts-apart-from-profiles, "What OTTY stores":
    /// an entry is the plugin and the part's id of the account.
    #[test]
    fn the_text_keeps_each_account_by_its_plugin_and_id() {
        let text = r#"{
            "accounts": [
                { "plugin": "example", "id": "chatgpt:made-up-one", "name": "ChatGPT (Home)" },
                { "plugin": "example", "id": "chatgpt:made-up-two", "hidden": true },
                { "plugin": "example", "id": "org:made-up-org", "name": "Acme" },
                { "plugin": "deepseek", "id": "key:27c8d30e", "hidden": true }
            ]
        }"#;

        assert_eq!(load(text), named_and_hidden());
    }

    /// Decision 8: a name and Hide stay when the part reads the account
    /// again with another name, plan or folder: they are kept by its
    /// plugin and id.
    #[test]
    fn a_name_and_hide_survive_a_new_read_of_the_account() {
        let saved = SavedProfiles::default()
            .with_account_name(&key(), Some(String::from("Mine")))
            .with_account_hidden(&key(), true);
        let read_again = Account::new(
            "deepseek",
            ProviderId::new("deepseek"),
            "key:27c8d30e",
            "you@example.com",
            AccountKind::Key,
            Vec::new(),
        )
        .with_plan(Some(String::from("Pay as you go")));

        assert_eq!(saved.account_name(&read_again), Some("Mine"));
        assert!(saved.is_account_hidden(&read_again));
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

    /// Decision 1: one id of two plugins is two accounts, so the user
    /// hides or names each for itself.
    #[test]
    fn the_choices_of_an_account_are_kept_per_plugin() {
        let of_plugin = |plugin: &str| {
            Account::new(
                plugin,
                ProviderId::new("openai"),
                "chatgpt:made-up-one",
                "you@example.com",
                AccountKind::Subscription,
                Vec::new(),
            )
        };
        let saved = SavedProfiles::default()
            .with_account_hidden(&of_plugin("openai-fork"), true)
            .with_account_name(&of_plugin("openai"), Some("Home".into()));

        let text = saved.to_json().expect("the paths are text");
        let loaded = load(&text);

        assert_eq!(loaded, saved);
        assert!(text.contains(r#""plugin": "openai-fork""#), "{text}");
        assert!(loaded.is_account_hidden(&of_plugin("openai-fork")));
        assert!(!loaded.is_account_hidden(&of_plugin("openai")));
        assert_eq!(loaded.account_name(&of_plugin("openai")), Some("Home"));
    }

    /// An entry that names no plugin or no id, one that keeps nothing,
    /// and an entry of the earlier layout are left out (spec 2026-11-23,
    /// Decision 12).
    #[test]
    fn an_entry_that_names_no_account_is_left_out() {
        let text = r#"{
            "accounts": [
                { "plugin": "example", "id": "", "name": "Nobody" },
                { "plugin": "", "id": "x", "hidden": true },
                { "plugin": "example", "id": "y" },
                { "plugin": "example", "id": "z", "given_keys": [{ "slot": "admin" }] },
                { "provider": "openai", "account_id": "made-up", "hidden": true }
            ]
        }"#;

        assert_eq!(load(text), SavedProfiles::default());
    }
}
