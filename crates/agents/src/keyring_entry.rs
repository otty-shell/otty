//! The keyring entries of the known agents and the accounts found in them
//! (spec 2026-11-19, Decisions 2 and 8).
//!
//! Goose keeps all its secrets in one entry of the system keyring, service
//! `goose` and user `secrets`, as a JSON map of variable names to values,
//! such as `{"OPENROUTER_API_KEY": "sk-or-…"}`. OTTY reads that entry only,
//! only for an agent found on this computer, and only after the user
//! allowed it (the app keeps the allowance in `agents.json`). A key read
//! there is turned into its fingerprint at once and never kept.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::account::{Account, AccountIdentity, FoundIn, merge_accounts};
use crate::admin_key::AdminKey;
use crate::env::AgentEnv;
use crate::key_fingerprint::{KeyFingerprint, SeenKey};
use crate::provider::Provider;
use crate::system_keyring::{KeyringError, KeyringRead, SystemKeyring};

/// The entry Goose keeps its secrets in.
const GOOSE: KeyringEntry = KeyringEntry {
    agent: "Goose",
    config_dir: ".config/goose",
    service: "goose",
    user: "secrets",
};

/// The entries of the known agents, in the order they are read.
const KNOWN_ENTRIES: [KeyringEntry; 1] = [GOOSE];

/// The variables of a secrets map that hold a key, with the provider of
/// the key.
const KEY_VARIABLES: [(&str, Provider); 5] = [
    ("ANTHROPIC_API_KEY", Provider::Anthropic),
    ("OPENAI_API_KEY", Provider::OpenAI),
    ("DEEPSEEK_API_KEY", Provider::DeepSeek),
    ("OPENROUTER_API_KEY", Provider::OpenRouter),
    ("MOONSHOT_API_KEY", Provider::Moonshot),
];

/// The entry of the system keyring a known agent keeps its keys in.
///
/// Only this crate makes one, for the agents it knows, so OTTY never reads
/// another entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyringEntry {
    agent: &'static str,
    config_dir: &'static str,
    service: &'static str,
    user: &'static str,
}

impl KeyringEntry {
    /// The name of the agent, such as "Goose".
    pub fn agent(&self) -> &'static str {
        self.agent
    }

    /// The service of the entry, such as `goose`.
    pub fn service(&self) -> &'static str {
        self.service
    }

    /// The entry of Goose, for the tests of the crates that use this one.
    #[cfg(any(test, feature = "fixtures"))]
    pub(crate) fn goose() -> Self {
        GOOSE
    }
}

/// The keyring entries of the known agents found on this computer: those
/// whose config folder is in the home folder of `env`. Nothing is read
/// from the keyring.
///
/// ```
/// use std::path::PathBuf;
///
/// use otty_agents::{AgentEnv, keyring_entries};
///
/// let env = AgentEnv::new(PathBuf::from("/no/such/home"));
///
/// assert!(keyring_entries(&env).is_empty());
/// ```
pub fn keyring_entries(env: &AgentEnv) -> Vec<KeyringEntry> {
    KNOWN_ENTRIES
        .into_iter()
        .filter(|entry| env.home().join(entry.config_dir).is_dir())
        .collect()
}

/// The accounts whose keys are in `entries` of the system keyring, each
/// once, found in the system keyring. It blocks, and the system may ask
/// the user to unlock the keyring first. An entry that is not there holds
/// no account.
///
/// # Errors
///
/// [`KeyringError`] when the keyring did not answer, stayed locked, or an
/// entry is not in the shape OTTY knows.
pub fn read_keyring_accounts(
    entries: &[KeyringEntry],
) -> Result<Vec<Account>, KeyringError> {
    accounts_in(&SystemKeyring, entries)
}

/// The key of the key `account` found in the keyring, read again from
/// `entries` of `keyring` for one request: the key of its provider whose
/// fingerprint is the account's. `None` when it is no longer there.
pub(crate) fn key_in_keyring(
    keyring: &dyn KeyringRead,
    entries: &[KeyringEntry],
    account: &Account,
) -> Result<Option<AdminKey>, KeyringError> {
    let AccountIdentity::KeyFingerprint(fingerprint) = account.identity()
    else {
        return Ok(None);
    };

    for entry in entries {
        let Some(secrets) = secrets_in(keyring, entry)? else {
            continue;
        };

        let key = KEY_VARIABLES
            .iter()
            .filter(|(_, provider)| *provider == account.provider())
            .filter_map(|(variable, _)| secrets.get(*variable)?.as_str())
            .find(|key| KeyFingerprint::of(key) == *fingerprint)
            .and_then(AdminKey::of);
        if key.is_some() {
            return Ok(key);
        }
    }

    Ok(None)
}

/// The accounts in `entries` of `keyring`.
fn accounts_in(
    keyring: &dyn KeyringRead,
    entries: &[KeyringEntry],
) -> Result<Vec<Account>, KeyringError> {
    let mut found = Vec::new();

    for entry in entries {
        let Some(secrets) = secrets_in(keyring, entry)? else {
            continue;
        };

        found.extend(key_accounts(&secrets));
    }

    Ok(merge_accounts(found))
}

/// The secrets map of `entry` of `keyring`; `None` when the entry is not
/// there.
fn secrets_in(
    keyring: &dyn KeyringRead,
    entry: &KeyringEntry,
) -> Result<Option<BTreeMap<String, Value>>, KeyringError> {
    let Some(text) = keyring.password(entry.service, entry.user)? else {
        return Ok(None);
    };

    serde_json::from_str(&text)
        .map(Some)
        .map_err(|_| KeyringError::Unreadable)
}

/// The accounts of the keys in a secrets map, in the order of
/// [`KEY_VARIABLES`]; a value that is not text is skipped.
fn key_accounts(secrets: &BTreeMap<String, Value>) -> Vec<Account> {
    KEY_VARIABLES
        .iter()
        .filter_map(|(variable, provider)| {
            let key = secrets.get(*variable)?.as_str()?;
            let seen = SeenKey::of(key)?;

            Some(Account::of_key(*provider, &seen, FoundIn::SystemKeyring))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use mockall::predicate::eq;

    use super::*;
    use crate::account::AccountLogin;
    use crate::system_keyring::MockKeyringRead;
    use crate::testing::{TempDir, write_file};

    const OPENROUTER_KEY: &str = "sk-or-v1-made-up-0000-key-c3d4";
    const ANTHROPIC_KEY: &str = "sk-ant-made-up-0000-key-a1f3";

    /// A keyring whose Goose entry, read once, gives `answer`.
    fn goose_keyring(
        answer: Result<Option<String>, KeyringError>,
    ) -> MockKeyringRead {
        let mut keyring = MockKeyringRead::new();
        keyring
            .expect_password()
            .with(eq("goose"), eq("secrets"))
            .times(1)
            .return_once(move |_, _| answer);
        keyring
    }

    fn secrets(text: &str) -> Result<Option<String>, KeyringError> {
        Ok(Some(String::from(text)))
    }

    #[test]
    fn the_entry_of_an_agent_is_listed_only_when_it_is_on_this_computer() {
        let with_goose = TempDir::new();
        write_file(
            &with_goose.path().join(".config/goose/config.yaml"),
            "GOOSE_PROVIDER: openrouter\n",
        );
        let without = TempDir::new();

        let found =
            keyring_entries(&AgentEnv::new(with_goose.path().to_path_buf()));
        let none =
            keyring_entries(&AgentEnv::new(without.path().to_path_buf()));

        assert_eq!(found, [KeyringEntry::goose()]);
        assert_eq!(found[0].agent(), "Goose");
        assert_eq!(found[0].service(), "goose");
        assert!(none.is_empty(), "{none:?}");
    }

    /// Each key of a known provider is an account found in the keyring,
    /// shown by its last characters; other secrets are skipped.
    #[test]
    fn the_keys_of_goose_are_accounts_found_in_the_system_keyring() {
        let keyring = goose_keyring(secrets(&format!(
            r#"{{
                "OPENROUTER_API_KEY": "{OPENROUTER_KEY}",
                "ANTHROPIC_API_KEY": "{ANTHROPIC_KEY}",
                "GOOGLE_API_KEY": "made-up-google-key-0000",
                "databricks_oauth": {{"token": "made-up"}},
                "OPENAI_API_KEY": 42
            }}"#
        )));

        let accounts = accounts_in(&keyring, &[KeyringEntry::goose()])
            .expect("the entry is a secrets map");

        let found: Vec<_> = accounts
            .iter()
            .map(|account| {
                (account.provider(), account.login(), account.found_in())
            })
            .collect();
        assert_eq!(
            found,
            [
                (
                    Provider::Anthropic,
                    &AccountLogin::Key(Some(String::from("a1f3"))),
                    &[FoundIn::SystemKeyring][..]
                ),
                (
                    Provider::OpenRouter,
                    &AccountLogin::Key(Some(String::from("c3d4"))),
                    &[FoundIn::SystemKeyring][..]
                ),
            ]
        );
        assert!(
            !format!("{accounts:?}").contains("made-up-0000"),
            "no key is kept: {accounts:?}"
        );
    }

    #[test]
    fn an_entry_that_is_not_there_holds_no_account() {
        let keyring = goose_keyring(Ok(None));

        let accounts = accounts_in(&keyring, &[KeyringEntry::goose()]);

        assert_eq!(accounts, Ok(Vec::new()));
    }

    #[test]
    fn a_locked_keyring_or_an_odd_entry_is_a_failure_that_shows_no_secret() {
        let locked = goose_keyring(Err(KeyringError::NotAllowed));
        let odd = goose_keyring(secrets("sk-made-up-plain-text-key"));

        let locked = accounts_in(&locked, &[KeyringEntry::goose()]);
        let odd = accounts_in(&odd, &[KeyringEntry::goose()]);

        assert_eq!(locked, Err(KeyringError::NotAllowed));
        assert_eq!(odd, Err(KeyringError::Unreadable));
        assert!(!format!("{odd:?}").contains("made-up"), "{odd:?}");
    }

    #[test]
    fn the_key_of_an_account_is_found_again_by_its_fingerprint() {
        let keyring = || {
            goose_keyring(secrets(&format!(
                r#"{{"OPENROUTER_API_KEY": "{OPENROUTER_KEY}",
                "ANTHROPIC_API_KEY": "{ANTHROPIC_KEY}"}}"#
            )))
        };
        let seen = SeenKey::of(OPENROUTER_KEY).expect("a key");
        let account = Account::of_key(
            Provider::OpenRouter,
            &seen,
            FoundIn::SystemKeyring,
        );
        let other = Account::of_key(
            Provider::OpenRouter,
            &SeenKey::of("sk-or-v1-another-key-0000").expect("a key"),
            FoundIn::SystemKeyring,
        );

        let found =
            key_in_keyring(&keyring(), &[KeyringEntry::goose()], &account);
        let gone = key_in_keyring(&keyring(), &[KeyringEntry::goose()], &other);

        assert_eq!(
            found.map(|key| key.map(|key| key.secret().to_string())),
            Ok(Some(String::from(OPENROUTER_KEY)))
        );
        assert!(matches!(gone, Ok(None)));
    }

    /// No agent that keeps keys in the keyring is on this computer, so the
    /// keyring is not asked at all.
    #[test]
    fn with_no_entry_the_keyring_is_never_asked() {
        let keyring = MockKeyringRead::new();

        let accounts = accounts_in(&keyring, &[]);

        assert_eq!(accounts, Ok(Vec::new()));
    }
}
