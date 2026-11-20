//! The accounts found in a profile of OpenCode or Kilo (spec 2026-11-19,
//! Decision 2): the logins of `auth.json`, of Kilo's `account.json`, which
//! keeps several logins of one provider, and of the `credential` table of
//! the database.
//!
//! A ChatGPT login is told by its account id, as in Codex, so the same
//! login in Codex and in Kilo is one account; its access token, a JWT,
//! names the email and the plan. A key, and the token of a Kilo login,
//! which works as a key of the Kilo gateway, is turned into its
//! fingerprint at once. A login of another provider with no id to tell it
//! apart, such as a Claude login, is no account. No key or token is kept.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;
use serde_json::Value;

use super::database::Database;
use super::provider_ids::provider_of;
use crate::account::{
    Account, AccountIdentity, AccountKind, AccountLogin, FoundIn, join_copies,
};
use crate::account_key::AccountKey;
use crate::account_numbers::{jwt_claims, plan_name};
use crate::admin_key::AdminKey;
use crate::built_in::{KILO, OPENAI};
use crate::json_file::{filled, read_json};
use crate::key_fingerprint::{KeyFingerprint, SeenKey};
use crate::provider::ProviderRef;

/// A login as the agents keep it: in `auth.json` and `account.json` with
/// the type `api` or `oauth`, in the `credential` table with `key` or
/// `oauth`. Its key or token is read only to tell the account.
#[derive(Deserialize)]
struct StoredLogin {
    #[serde(rename = "type")]
    kind: String,
    key: Option<String>,
    access: Option<String>,
    #[serde(rename = "accountId")]
    account_id: Option<String>,
    metadata: Option<Value>,
}

impl StoredLogin {
    /// The key or the token of the login whose fingerprint `fingerprint`
    /// is, with the team of a Kilo login, read for one request.
    fn key_with_fingerprint(
        &self,
        provider: &ProviderRef,
        fingerprint: &KeyFingerprint,
    ) -> Option<AccountKey> {
        let secret = match self.kind.as_str() {
            "api" | "key" => self.key.as_deref()?,
            "oauth" => self.access.as_deref()?,
            _ => return None,
        };
        if KeyFingerprint::of(secret) != *fingerprint {
            return None;
        }

        let team = self
            .account_id()
            .filter(|_| provider.is_id(KILO))
            .map(String::from);
        Some(AccountKey::new(AdminKey::of(secret)?).with_organization_id(team))
    }

    /// The ChatGPT workspace the login names, as `auth.json` or the
    /// `credential` table keeps it.
    fn account_id(&self) -> Option<&str> {
        let in_metadata = self
            .metadata
            .as_ref()
            .and_then(|metadata| metadata.get("accountID"))
            .and_then(Value::as_str);

        filled(self.account_id.as_deref()).or(filled(in_metadata))
    }
}

/// Kilo's `account.json`: its logins by their own ids.
#[derive(Deserialize)]
struct AccountFile {
    accounts: BTreeMap<String, StoredAccount>,
}

/// One login of `account.json`, with the provider it logs in to.
#[derive(Deserialize)]
struct StoredAccount {
    #[serde(rename = "serviceID")]
    service_id: String,
    credential: Value,
}

/// What the access token of a ChatGPT login tells of it.
#[derive(Default, Deserialize)]
struct AccessClaims {
    #[serde(rename = "https://api.openai.com/auth")]
    auth: Option<ChatgptAuth>,
    #[serde(rename = "https://api.openai.com/profile")]
    profile: Option<ChatgptProfile>,
}

/// The ChatGPT part of the access token.
#[derive(Deserialize)]
struct ChatgptAuth {
    chatgpt_account_id: Option<String>,
    chatgpt_plan_type: Option<String>,
}

/// The profile part of the access token.
#[derive(Deserialize)]
struct ChatgptProfile {
    email: Option<String>,
}

/// The accounts found in the data folder `data_dir` and in its
/// `database`, each once.
pub(super) fn accounts(
    data_dir: &Path,
    database: Option<&Database>,
) -> Vec<Account> {
    let found_in = FoundIn::ProfileFolder(data_dir.to_path_buf());

    let found = stored_logins(data_dir, database)
        .into_iter()
        .filter_map(|(provider_id, login)| {
            account_of(&provider_id, &login, &found_in)
        })
        .collect();

    join_copies(found)
}

/// The key or the login token of `account`, read again from the logins of
/// the data folder `data_dir` and its `database` for one request: the key
/// or the Kilo token with the account's fingerprint, or the access token
/// of the ChatGPT login of the account. A login is never renewed.
pub(super) fn account_key(
    data_dir: &Path,
    database: Option<&Database>,
    account: &Account,
) -> Option<AccountKey> {
    let found_in = FoundIn::ProfileFolder(data_dir.to_path_buf());

    stored_logins(data_dir, database)
        .into_iter()
        .filter(|(provider_id, _)| {
            provider_of(provider_id).as_ref() == Some(account.provider())
        })
        .find_map(|(_, login)| match account.identity() {
            AccountIdentity::KeyFingerprint(fingerprint) => {
                login.key_with_fingerprint(account.provider(), fingerprint)
            },
            AccountIdentity::AccountId(_) if login.kind == "oauth" => {
                let found = chatgpt_account(&login, &found_in)?;
                if found.identity() != account.identity() {
                    return None;
                }
                let token = AdminKey::of(login.access.as_deref()?)?;
                Some(AccountKey::new(token))
            },
            _ => None,
        })
}

/// The logins of the data folder `data_dir` and of its `database`, each
/// with the id of the provider the agent keeps it under; one that is not
/// a login is left out.
fn stored_logins(
    data_dir: &Path,
    database: Option<&Database>,
) -> Vec<(String, StoredLogin)> {
    let mut logins: Vec<(String, Value)> = Vec::new();
    if let Some(auth) =
        read_json::<BTreeMap<String, Value>>(&data_dir.join("auth.json"))
    {
        logins.extend(auth);
    }
    if let Some(file) = read_json::<AccountFile>(&data_dir.join("account.json"))
    {
        logins.extend(
            file.accounts
                .into_values()
                .map(|account| (account.service_id, account.credential)),
        );
    }
    if let Some(rows) =
        database.and_then(|database| database.credentials().ok())
    {
        logins.extend(rows.into_iter().filter_map(|row| {
            let value = serde_json::from_str(&row.value).ok()?;
            Some((row.integration_id, value))
        }));
    }

    logins
        .into_iter()
        .filter_map(|(provider_id, login)| {
            let login = serde_json::from_value::<StoredLogin>(login).ok()?;
            Some((provider_id, login))
        })
        .collect()
}

/// The account of `login` at the provider the agent names `provider_id`;
/// `None` for a provider OTTY does not know or a login it can not tell
/// apart.
fn account_of(
    provider_id: &str,
    login: &StoredLogin,
    found_in: &FoundIn,
) -> Option<Account> {
    let provider = provider_of(provider_id)?;
    let id = provider.as_id().map_or("", |id| id.as_str());

    match (login.kind.as_str(), id) {
        ("api" | "key", _) => {
            let key = SeenKey::of(login.key.as_deref()?)?;
            Some(Account::of_key(provider, &key, found_in.clone()))
        },
        ("oauth", OPENAI) => chatgpt_account(login, found_in),
        ("oauth", KILO) => {
            let token = SeenKey::of(login.access.as_deref()?)?;
            Some(Account::of_key(provider, &token, found_in.clone()))
        },
        _ => None,
    }
}

/// The account of the ChatGPT `login`: told by its account id, else its
/// email, with the plan its access token names.
fn chatgpt_account(login: &StoredLogin, found_in: &FoundIn) -> Option<Account> {
    let claims: AccessClaims = filled(login.access.as_deref())
        .and_then(jwt_claims)
        .unwrap_or_default();
    let auth = claims.auth.as_ref();
    let account_id = login.account_id().or_else(|| {
        filled(auth.and_then(|auth| auth.chatgpt_account_id.as_deref()))
    });
    let email = claims
        .profile
        .as_ref()
        .and_then(|profile| filled(profile.email.as_deref()));

    let identity = match (account_id, email) {
        (Some(id), _) => AccountIdentity::AccountId(id.to_string()),
        (None, Some(email)) => AccountIdentity::Email(email.to_string()),
        (None, None) => return None,
    };
    let shown = match email {
        Some(email) => AccountLogin::Email(email.to_string()),
        None => AccountLogin::Unnamed,
    };
    let plan = auth
        .and_then(|auth| filled(auth.chatgpt_plan_type.as_deref()))
        .and_then(plan_name);

    let account = Account::new(
        ProviderRef::of_id(OPENAI),
        identity,
        shown,
        AccountKind::Limits,
        found_in.clone(),
    );
    Some(account.with_plan(plan))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::key_fingerprint::KeyFingerprint;
    use crate::open_code::testing::MadeUpDatabase;
    use crate::testing::{TempDir, made_up_jwt, write_file};

    const SHARED_ID: &str = "00000000-0000-4000-8000-0000000000c1";
    const OTHER_ID: &str = "00000000-0000-4000-8000-0000000000c2";
    const KEY: &str = "sk-made-up-deepseek-key-0000-9b1d";

    /// The access token of a ChatGPT login of the account `id` on `plan`.
    fn access_token(id: &str, email: &str, plan: &str) -> String {
        made_up_jwt(&format!(
            r#"{{
                "https://api.openai.com/auth": {{
                    "chatgpt_account_id": "{id}",
                    "chatgpt_plan_type": "{plan}"
                }},
                "https://api.openai.com/profile": {{"email": "{email}"}}
            }}"#
        ))
    }

    /// A ChatGPT login as `auth.json` and `account.json` keep it.
    fn oauth(id: &str, email: &str, plan: &str) -> String {
        format!(
            r#"{{"type": "oauth", "refresh": "made-up-refresh", "access": "{}",
            "expires": 1, "accountId": "{id}"}}"#,
            access_token(id, email, plan)
        )
    }

    fn data_dir(temp: &TempDir) -> PathBuf {
        let dir = temp.path().join("kilo");
        std::fs::create_dir_all(&dir).expect("create the data folder");
        dir
    }

    fn chatgpt(id: &str, email: &str, plan: &str, dir: &Path) -> Account {
        Account::new(
            ProviderRef::of_id("openai"),
            AccountIdentity::AccountId(id.into()),
            AccountLogin::Email(email.into()),
            AccountKind::Limits,
            FoundIn::ProfileFolder(dir.to_path_buf()),
        )
        .with_plan(Some(plan.into()))
    }

    /// Two ChatGPT logins in Kilo: the one of `auth.json`, also in the
    /// `credential` table, and another in `account.json`.
    #[test]
    fn two_chatgpt_logins_are_two_accounts_and_one_login_is_one() {
        let temp = TempDir::new();
        let dir = data_dir(&temp);
        let shared = oauth(SHARED_ID, "dev@example.com", "plus");
        write_file(
            &dir.join("auth.json"),
            &format!(r#"{{"openai": {shared}}}"#),
        );
        write_file(
            &dir.join("account.json"),
            &format!(
                r#"{{"version": 2, "accounts": {{"acc_1": {{"id": "acc_1",
                "serviceID": "openai", "description": "Team",
                "credential": {}}}}}, "active": {{"openai": "acc_1"}}}}"#,
                oauth(OTHER_ID, "team@example.com", "team")
            ),
        );
        let made_up = MadeUpDatabase::first_shape(&dir.join("kilo.db"));
        made_up.credential(
            "cred_1",
            "openai",
            &format!(
                r#"{{"type": "oauth", "methodID": "chatgpt-browser",
                "refresh": "r", "access": "{}", "expires": 1,
                "metadata": {{"accountID": "{SHARED_ID}"}}}}"#,
                access_token(SHARED_ID, "dev@example.com", "plus")
            ),
        );
        let database = Database::open(made_up.path())
            .expect("the database opens")
            .expect("the database is there");

        let found = accounts(&dir, Some(&database));

        assert_eq!(
            found,
            [
                chatgpt(SHARED_ID, "dev@example.com", "Plus", &dir),
                chatgpt(OTHER_ID, "team@example.com", "Business", &dir),
            ]
        );
    }

    /// The ChatGPT login of Codex and of Kilo is one account found in both
    /// folders (spec Decision 2).
    #[test]
    fn the_chatgpt_login_of_codex_is_the_same_account() {
        let temp = TempDir::new();
        let dir = data_dir(&temp);
        let shared = oauth(SHARED_ID, "dev@example.com", "plus");
        write_file(
            &dir.join("auth.json"),
            &format!(r#"{{"openai": {shared}}}"#),
        );
        let codex = PathBuf::from("/home/user/.codex");
        let of_codex = chatgpt(SHARED_ID, "dev@example.com", "Plus", &codex);

        let mut all = vec![of_codex];
        all.extend(accounts(&dir, None));
        let merged = join_copies(all);

        assert_eq!(merged.len(), 1);
        assert_eq!(
            merged[0].found_in(),
            [FoundIn::ProfileFolder(codex), FoundIn::ProfileFolder(dir)]
        );
    }

    #[test]
    fn keys_and_a_kilo_login_are_key_accounts() {
        let temp = TempDir::new();
        let dir = data_dir(&temp);
        let kilo_token = "made-up-kilo-token-0000-4c2e";
        write_file(
            &dir.join("auth.json"),
            &format!(
                r#"{{"deepseek": {{"type": "api", "key": "{KEY}"}},
                "kilo": {{"type": "oauth", "refresh": "", "access": "{kilo_token}",
                "expires": 1}}}}"#
            ),
        );

        let found = accounts(&dir, None);

        let key_of = |provider, key: &str, ending: &str| {
            Account::new(
                provider,
                AccountIdentity::KeyFingerprint(KeyFingerprint::of(key)),
                AccountLogin::Key(Some(ending.into())),
                AccountKind::Money,
                FoundIn::ProfileFolder(dir.clone()),
            )
        };
        assert_eq!(
            found,
            [
                key_of(ProviderRef::of_id("deepseek"), KEY, "9b1d"),
                key_of(ProviderRef::of_id("kilo"), kilo_token, "4c2e"),
            ]
        );
    }

    #[test]
    fn a_login_of_an_unknown_provider_or_with_no_id_is_no_account() {
        let temp = TempDir::new();
        let dir = data_dir(&temp);
        write_file(
            &dir.join("auth.json"),
            r#"{"github-copilot": {"type": "oauth", "access": "a", "refresh": "r", "expires": 1},
            "anthropic": {"type": "oauth", "access": "opaque", "refresh": "r", "expires": 1},
            "openai": {"type": "oauth", "access": "opaque", "refresh": "r", "expires": 1},
            "local": {"type": "api", "key": "made-up-local-key-000000"}}"#,
        );

        assert_eq!(accounts(&dir, None), []);
        assert_eq!(accounts(&temp.path().join("missing"), None), []);
    }

    #[test]
    fn no_key_or_token_is_in_the_debug_output() {
        let temp = TempDir::new();
        let dir = data_dir(&temp);
        let token = access_token(SHARED_ID, "dev@example.com", "pro");
        write_file(
            &dir.join("auth.json"),
            &format!(
                r#"{{"deepseek": {{"type": "api", "key": "{KEY}"}},
                "openai": {{"type": "oauth", "access": "{token}",
                "refresh": "made-up-refresh", "expires": 1}}}}"#
            ),
        );

        let found = accounts(&dir, None);
        let debug = format!("{found:?}");

        assert_eq!(found.len(), 2);
        assert!(!debug.contains(KEY), "{debug}");
        assert!(!debug.contains(&token), "{debug}");
        assert!(!debug.contains("made-up-refresh"), "{debug}");
    }

    /// Spec 2026-11-19: the token of a ChatGPT login Kilo keeps is read
    /// again for its limits, and the token of the Kilo login, with its
    /// team, for its balance.
    #[test]
    fn the_token_of_each_login_is_read_again_for_its_own_account() {
        let temp = TempDir::new();
        let dir = data_dir(&temp);
        let kilo_token = "made-up-kilo-token-0000-4c2e";
        let shared = oauth(SHARED_ID, "dev@example.com", "plus");
        write_file(
            &dir.join("auth.json"),
            &format!(
                r#"{{"openai": {shared},
                "deepseek": {{"type": "api", "key": "{KEY}"}},
                "kilo": {{"type": "oauth", "refresh": "", "access": "{kilo_token}",
                "expires": 1, "accountId": "made-up-team"}}}}"#
            ),
        );
        let found = accounts(&dir, None);
        let secret_of = |account: &Account| {
            account_key(&dir, None, account)
                .map(|key| key.key().secret().to_string())
        };

        let chatgpt = found
            .iter()
            .find(|account| account.provider().is_id("openai"))
            .expect("the ChatGPT login is found");
        let kilo = found
            .iter()
            .find(|account| account.provider().is_id("kilo"))
            .expect("the Kilo login is found");
        let deepseek = found
            .iter()
            .find(|account| account.provider().is_id("deepseek"))
            .expect("the key is found");

        assert_eq!(
            secret_of(chatgpt),
            Some(access_token(SHARED_ID, "dev@example.com", "plus"))
        );
        assert_eq!(secret_of(kilo), Some(String::from(kilo_token)));
        assert_eq!(
            account_key(&dir, None, kilo)
                .and_then(|key| key.organization_id().map(String::from)),
            Some(String::from("made-up-team"))
        );
        assert_eq!(secret_of(deepseek), Some(String::from(KEY)));
        assert_eq!(
            secret_of(&chatgpt_login_of(OTHER_ID, &dir)),
            None,
            "another login has no token here"
        );
    }

    fn chatgpt_login_of(id: &str, dir: &Path) -> Account {
        chatgpt(id, "team@example.com", "Team", dir)
    }
}
