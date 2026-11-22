//! The logins and keys found in a profile, each named by where its key
//! sits, never by the key: the logins of `auth.json`, by the id of their
//! provider, and of the `credential` table of the database, whose `value`
//! column is JSON. The host gives both with every key replaced by a
//! handle; the type of a login and its other fields stay readable.
//!
//! A ChatGPT login is told by its account id, as in Codex, so the same
//! login in Codex and in OpenCode is one account; its access token, a
//! JWT, names the email and the plan, read through its handle. A key is
//! told by its fingerprint, which OTTY takes. A login of another
//! provider with no id to tell it apart, such as a Claude login, is left
//! out, and so is a login of a provider OTTY does not read.

use std::collections::BTreeMap;

use otty_plugin_sdk::agents::{
    Billing, Connection, Identity, Login, LoginKind, Profile, ProfileStatus,
    ProviderRef,
};
use otty_plugin_sdk::{JsonField, JsonWithKeys, KeyPlace, SqliteField, files};
use serde::{Deserialize, Deserializer};
use serde_json::Value;

use crate::database::Database;
use crate::glob::escaped;
use crate::location::{database_of, start_command};
use crate::needs;
use crate::plan::plan_name;
use crate::provider_ids::{OPENAI, provider_of};

/// The file of the logins in the data folder.
const AUTH_FILE: &str = "auth.json";

/// The table and the column of the logins in the database.
const CREDENTIAL_TABLE: &str = "credential";
const CREDENTIAL_COLUMN: &str = "value";

/// The field of a key in a login.
const KEY_FIELD: &str = "key";

/// The field of the access token in a login.
const ACCESS_FIELD: &str = "access";

/// A login as OpenCode keeps it: in `auth.json` with the type `api` or
/// `oauth`, in the `credential` table with `key` or `oauth`. Its key and
/// its token are handles.
#[derive(Deserialize)]
struct StoredLogin {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default, deserialize_with = "key_marker")]
    key: Option<usize>,
    #[serde(default, deserialize_with = "key_marker")]
    access: Option<usize>,
    #[serde(rename = "accountId")]
    account_id: Option<String>,
    metadata: Option<Value>,
}

impl StoredLogin {
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

/// One login found, with the document that holds its key handles and how
/// its key place is named.
struct Found<'a> {
    provider_id: &'a str,
    login: StoredLogin,
    document: &'a JsonWithKeys,
    place: &'a dyn Fn(&str) -> KeyPlace,
}

/// The logins of `profile`: those of `auth.json` first, then those of the
/// `credential` table. A login found in both places is given once.
pub(crate) fn logins(profile: &Profile) -> Vec<Login> {
    let auth_path =
        format!("{}/{AUTH_FILE}", profile.folder.trim_end_matches('/'));
    let auth = files::json_with_keys(&auth_path).ok();
    let database = database_of(profile)
        .and_then(|path| Database::open(&path).ok().flatten());
    let credentials = database
        .as_ref()
        .and_then(|database| database.credentials().ok())
        .unwrap_or_default();

    let mut found: Vec<Login> = Vec::new();
    let mut fingerprints: Vec<String> = Vec::new();
    let mut take = |login: Option<Login>, fingerprint: Option<String>| {
        let Some(login) = login else {
            return;
        };
        let is_copy = match &fingerprint {
            Some(fingerprint) => fingerprints.contains(fingerprint),
            None => found.iter().any(|kept| is_same_account(kept, &login)),
        };
        if !is_copy {
            fingerprints.extend(fingerprint);
            found.push(login);
        }
    };

    if let Some(document) = &auth {
        let by_provider: BTreeMap<String, Value> =
            serde_json::from_str(document.json()).unwrap_or_default();

        for (provider_id, value) in by_provider {
            let Ok(login) = serde_json::from_value::<StoredLogin>(value) else {
                continue;
            };
            let prefix = format!("/{}", pointer_part(&provider_id));
            let place = |field: &str| {
                KeyPlace::JsonField(JsonField {
                    file: auth_path.clone(),
                    pointer: format!("{prefix}{field}"),
                })
            };
            let (login, fingerprint) = login_of(&Found {
                provider_id: &provider_id,
                login,
                document,
                place: &place,
            });
            take(login, fingerprint);
        }
    }

    if let Some(database) = &database {
        for row in &credentials {
            let Ok(login) =
                serde_json::from_str::<StoredLogin>(row.value.json())
            else {
                continue;
            };
            let place = |field: &str| {
                KeyPlace::SqliteField(SqliteField {
                    database: database.path().to_string(),
                    table: CREDENTIAL_TABLE.to_string(),
                    column: CREDENTIAL_COLUMN.to_string(),
                    row_id: row.row_id,
                    pointer: Some(field.to_string()),
                })
            };
            let (login, fingerprint) = login_of(&Found {
                provider_id: &row.integration_id,
                login,
                document: &row.value,
                place: &place,
            });
            take(login, fingerprint);
        }
    }

    found
}

/// Whether the profile in `profile` is logged in: connected with any
/// login it keeps, paid per token when one of them is a key. With no
/// login it needs one by `opencode auth login`.
pub(crate) fn status(profile: &Profile) -> ProfileStatus {
    if !folder_exists(&profile.folder) {
        return ProfileStatus::ConfigFolderNotFound;
    }

    let found = logins(profile);
    if found.is_empty() {
        let login = start_command(
            profile,
            vec!["auth".to_string(), "login".to_string()],
        );

        return ProfileStatus::NotLoggedIn(needs::not_logged_in(login));
    }

    let pays_per_token = found
        .iter()
        .any(|login| matches!(login.kind, LoginKind::ApiKey));

    ProfileStatus::Connected(Connection {
        account: None,
        plan: None,
        billing: pays_per_token.then_some(Billing::ApiKey),
    })
}

/// The login of `found`, with the fingerprint of its key when it is told
/// by its key; `None` for a provider OTTY does not read or a login it can
/// not tell apart.
fn login_of(found: &Found<'_>) -> (Option<Login>, Option<String>) {
    let Some(provider) = provider_of(found.provider_id) else {
        return (None, None);
    };
    let id = match &provider {
        ProviderRef::Id(id) => id.as_str(),
        ProviderRef::Host(_) => "",
    };

    match (found.login.kind.as_str(), id) {
        ("api" | "key", _) => key_login(found, provider),
        ("oauth", OPENAI) => (chatgpt_login(found), None),
        _ => (None, None),
    }
}

/// The key login of `found` for `provider`, told by the fingerprint of
/// its key; none when the login holds no key.
fn key_login(
    found: &Found<'_>,
    provider: ProviderRef,
) -> (Option<Login>, Option<String>) {
    let fingerprint = found
        .login
        .key
        .and_then(|index| found.document.key(index))
        .and_then(|key| key.fingerprint().ok());
    let Some(fingerprint) = fingerprint else {
        return (None, None);
    };

    let login = Login {
        kind: LoginKind::ApiKey,
        for_whom: provider,
        place: (found.place)(&format!("/{KEY_FIELD}")),
        identity: Identity::default(),
        plan: None,
    };

    (Some(login), Some(fingerprint))
}

/// The ChatGPT login of `found`: told by its account id, else its email,
/// with the plan its access token names.
fn chatgpt_login(found: &Found<'_>) -> Option<Login> {
    let token = found
        .login
        .access
        .and_then(|index| found.document.key(index))?;
    // A token that is no JWT is not a ChatGPT login. One whose claims name
    // no account, such as a login by "chatgpt-token-sharing" that keeps
    // them encrypted, is still a login, with no name.
    let claims = token.jwt_claims().ok().flatten()?;
    let claims: AccessClaims =
        serde_json::from_str(&claims).unwrap_or_default();
    let auth = claims.auth.as_ref();

    let account_id = found.login.account_id().or_else(|| {
        filled(auth.and_then(|auth| auth.chatgpt_account_id.as_deref()))
    });
    let email = claims
        .profile
        .as_ref()
        .and_then(|profile| filled(profile.email.as_deref()));
    let plan = auth
        .and_then(|auth| filled(auth.chatgpt_plan_type.as_deref()))
        .and_then(plan_name);

    Some(Login {
        kind: LoginKind::Subscription,
        for_whom: ProviderRef::Id(OPENAI.to_string()),
        place: (found.place)(&format!("/{ACCESS_FIELD}")),
        identity: Identity {
            account_id: account_id.map(String::from),
            email: email.map(String::from),
            ..Identity::default()
        },
        plan,
    })
}

/// Whether `one` and `other` are the same account told by its identity.
fn is_same_account(one: &Login, other: &Login) -> bool {
    let identity = &one.identity;
    let has_identity =
        identity.account_id.is_some() || identity.email.is_some();

    has_identity
        && one.for_whom == other.for_whom
        && match (&identity.account_id, &other.identity.account_id) {
            (Some(one), Some(other)) => one == other,
            _ => {
                identity.email.is_some()
                    && identity.email == other.identity.email
            },
        }
}

/// Whether the data folder `folder` holds any file. A folder is never
/// listed itself, so an empty folder counts as missing.
fn folder_exists(folder: &str) -> bool {
    let folder = escaped(folder);
    let has_files =
        |glob: String| files::list(&glob).is_ok_and(|found| !found.is_empty());

    has_files(format!("{folder}/*")) || has_files(format!("{folder}/**"))
}

/// `name` as one part of a JSON pointer.
fn pointer_part(name: &str) -> String {
    name.replace('~', "~0").replace('/', "~1")
}

/// `text` when it holds more than blanks.
fn filled(text: Option<&str>) -> Option<&str> {
    text.filter(|text| !text.trim().is_empty())
}

/// The index of the handle where the host put a key, `{"$key": n}`; any
/// other value is no key.
fn key_marker<'de, D>(deserializer: D) -> Result<Option<usize>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;

    Ok(value
        .get("$key")
        .and_then(Value::as_u64)
        .and_then(|index| usize::try_from(index).ok()))
}

#[cfg(test)]
mod tests {
    use otty_plugin_sdk::{Command, NeedAction, keys};

    use super::*;
    use crate::testing::{DataFolder, made_up_jwt};

    const SHARED_ID: &str = "00000000-0000-4000-8000-0000000000c1";
    const OTHER_ID: &str = "00000000-0000-4000-8000-0000000000c2";
    const KEY: &str = "sk-made-up-deepseek-key-0000-9b1d";
    const OPENROUTER_KEY: &str = "sk-or-made-up-key-0000-4c2e";

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

    /// A ChatGPT login as `auth.json` keeps it.
    fn oauth(id: &str, email: &str, plan: &str) -> String {
        format!(
            r#"{{"type": "oauth", "refresh": "made-up-refresh", "access": "{}",
            "expires": 1, "accountId": "{id}"}}"#,
            access_token(id, email, plan)
        )
    }

    fn openai() -> ProviderRef {
        ProviderRef::Id("openai".to_string())
    }

    fn json_field(data: &DataFolder, pointer: &str) -> KeyPlace {
        KeyPlace::JsonField(JsonField {
            file: data.file("auth.json"),
            pointer: pointer.to_string(),
        })
    }

    fn chatgpt(id: &str, email: &str, plan: &str, place: KeyPlace) -> Login {
        Login {
            kind: LoginKind::Subscription,
            for_whom: openai(),
            place,
            identity: Identity {
                account_id: Some(id.to_string()),
                email: Some(email.to_string()),
                ..Identity::default()
            },
            plan: Some(plan.to_string()),
        }
    }

    fn key_of(kind: LoginKind, provider: &str, place: KeyPlace) -> Login {
        Login {
            kind,
            for_whom: ProviderRef::Id(provider.to_string()),
            place,
            identity: Identity::default(),
            plan: None,
        }
    }

    /// Two ChatGPT logins: the one of `auth.json`, also in the
    /// `credential` table, and another only in the table.
    #[test]
    fn two_chatgpt_logins_are_two_and_one_login_in_two_places_is_one() {
        let data = DataFolder::new();
        let shared = oauth(SHARED_ID, "dev@example.com", "plus");
        data.write("auth.json", &format!(r#"{{"openai": {shared}}}"#));
        let database = data.first_shape();
        database.credential(
            "cred_1",
            "openai",
            &format!(
                r#"{{"type": "oauth", "methodID": "chatgpt-browser",
                "refresh": "r", "access": "{}", "expires": 1,
                "metadata": {{"accountID": "{SHARED_ID}"}}}}"#,
                access_token(SHARED_ID, "dev@example.com", "plus")
            ),
        );
        let other_row = database.credential(
            "cred_2",
            "openai",
            &format!(
                r#"{{"type": "oauth", "refresh": "r", "access": "{}",
                "expires": 1}}"#,
                access_token(OTHER_ID, "team@example.com", "team")
            ),
        );
        let profile = data.profile();

        let found = data.run(|| logins(&profile));

        let other_place = KeyPlace::SqliteField(SqliteField {
            database: data.file("opencode.db"),
            table: "credential".to_string(),
            column: "value".to_string(),
            row_id: other_row,
            pointer: Some("/access".to_string()),
        });
        assert_eq!(
            found,
            [
                chatgpt(
                    SHARED_ID,
                    "dev@example.com",
                    "Plus",
                    json_field(&data, "/openai/access")
                ),
                chatgpt(OTHER_ID, "team@example.com", "Business", other_place),
            ]
        );
    }

    /// Keys are told by their fingerprints.
    #[test]
    fn keys_are_key_logins() {
        let data = DataFolder::new();
        data.write(
            "auth.json",
            &format!(r#"{{"deepseek": {{"type": "api", "key": "{KEY}"}}}}"#),
        );
        let database = data.first_shape();
        let row = database.credential(
            "cred_1",
            "openrouter",
            r#"{"type": "key", "key": "sk-or-made-up-0000"}"#,
        );
        database.credential(
            "cred_2",
            "deepseek",
            &format!(r#"{{"type": "key", "key": "{KEY}"}}"#),
        );
        let profile = data.profile();

        let found = data.run(|| logins(&profile));

        let in_table = KeyPlace::SqliteField(SqliteField {
            database: data.file("opencode.db"),
            table: "credential".to_string(),
            column: "value".to_string(),
            row_id: row,
            pointer: Some("/key".to_string()),
        });
        assert_eq!(
            found,
            [
                key_of(
                    LoginKind::ApiKey,
                    "deepseek",
                    json_field(&data, "/deepseek/key")
                ),
                key_of(LoginKind::ApiKey, "openrouter", in_table),
            ],
            "the same DeepSeek key in the table is the one of auth.json"
        );
    }

    /// The host reads each key again at its place: the key of a login is
    /// there behind a handle, and the plugin never sees it.
    #[test]
    fn each_login_names_the_place_of_its_own_key() {
        let data = DataFolder::new();
        let token = access_token(SHARED_ID, "dev@example.com", "plus");
        data.write(
            "auth.json",
            &format!(
                r#"{{"deepseek": {{"type": "api", "key": "{KEY}"}},
                "openai": {{"type": "oauth", "access": "{token}",
                "refresh": "made-up-refresh", "expires": 1}}}}"#
            ),
        );
        let database = data.first_shape();
        database.credential(
            "cred_1",
            "openrouter",
            &format!(r#"{{"type": "key", "key": "{OPENROUTER_KEY}"}}"#),
        );
        let profile = data.profile();

        let fingerprints = data.run(|| {
            logins(&profile)
                .iter()
                .map(|login| {
                    keys::open(&login.place).and_then(|key| key.fingerprint())
                })
                .collect::<Vec<_>>()
        });
        let debug = format!("{:?}", data.run(|| logins(&profile)));

        assert_eq!(fingerprints.len(), 3);
        assert!(fingerprints.iter().all(Result::is_ok), "{fingerprints:?}");
        assert!(!debug.contains(KEY), "{debug}");
        assert!(!debug.contains(&token), "{debug}");
        assert!(!debug.contains(OPENROUTER_KEY), "{debug}");
        assert!(!debug.contains("made-up-refresh"), "{debug}");
    }

    #[test]
    fn a_login_of_an_unknown_provider_or_with_no_id_is_left_out() {
        let data = DataFolder::new();
        data.write(
            "auth.json",
            r#"{"github-copilot": {"type": "oauth", "access": "a", "refresh": "r", "expires": 1},
            "anthropic": {"type": "oauth", "access": "opaque", "refresh": "r", "expires": 1},
            "openai": {"type": "oauth", "access": "opaque", "refresh": "r", "expires": 1},
            "deepseek": {"type": "api", "key": " "},
            "local": {"type": "api", "key": "made-up-local-key-000000"}}"#,
        );
        let profile = data.profile();

        assert_eq!(data.run(|| logins(&profile)), []);
    }

    /// Owner, 2026-11-22: a ChatGPT login made by `opencode auth login`
    /// with "chatgpt-token-sharing" keeps its account and email encrypted
    /// in the token. It is still a login: the profile is connected, the
    /// login has no name.
    #[test]
    fn a_chatgpt_login_whose_token_names_no_account_is_a_login() {
        let data = DataFolder::new();
        let database = data.first_shape();
        let token = made_up_jwt(
            r#"{"https://api.openai.com/auth": {
                "encrypted_auth_metadata": "made-up", "per_user_salt": "made-up"},
                "sub": "made-up-user"}"#,
        );
        let row = database.credential(
            "cred_1",
            "openai",
            &format!(
                r#"{{"type": "oauth", "methodID": "chatgpt-token-sharing",
                "refresh": "r", "access": "{token}", "expires": 1,
                "metadata": {{"clientID": "made-up", "scopes": []}}}}"#
            ),
        );
        let profile = data.profile();

        let (found, status) = data.run(|| (logins(&profile), status(&profile)));

        let place = KeyPlace::SqliteField(SqliteField {
            database: data.file("opencode.db"),
            table: "credential".to_string(),
            column: "value".to_string(),
            row_id: row,
            pointer: Some("/access".to_string()),
        });
        assert_eq!(
            found,
            [Login {
                kind: LoginKind::Subscription,
                for_whom: openai(),
                place,
                identity: Identity::default(),
                plan: None,
            }]
        );
        assert!(
            matches!(status, ProfileStatus::Connected(_)),
            "connected: {status:?}"
        );
    }

    /// A folder with no login needs one: the button runs `opencode auth
    /// login`, pointed at the folder when it is not the default one.
    #[test]
    fn a_folder_with_no_login_needs_a_login_by_opencode_auth_login() {
        let data = DataFolder::new();
        data.write("auth.json", "{}");
        let moved = Profile {
            is_default: false,
            ..data.profile()
        };

        let statuses = data.run(|| [status(&data.profile()), status(&moved)]);

        let login = |env: Vec<(String, String)>| Command {
            program: "opencode".to_string(),
            args: vec!["auth".to_string(), "login".to_string()],
            env,
        };
        let data_home = data.path().parent().map(|parent| {
            ("XDG_DATA_HOME".to_string(), parent.display().to_string())
        });
        let expected =
            [login(Vec::new()), login(data_home.into_iter().collect())];
        for (status, command) in statuses.into_iter().zip(expected) {
            let ProfileStatus::NotLoggedIn(need) = status else {
                panic!("not logged in: {status:?}");
            };
            assert_eq!(need.id, "not-logged-in");
            assert_eq!(need.title, "Needs login");
            assert_eq!(need.action, NeedAction::Run(command));
        }
    }

    #[test]
    fn a_folder_with_a_login_is_connected_and_a_key_pays_per_token() {
        let chatgpt = DataFolder::new();
        chatgpt.write(
            "auth.json",
            &format!(
                r#"{{"openai": {}}}"#,
                oauth(SHARED_ID, "dev@example.com", "plus")
            ),
        );
        let key = DataFolder::new();
        key.write(
            "auth.json",
            &format!(r#"{{"deepseek": {{"type": "api", "key": "{KEY}"}}}}"#),
        );
        let none = DataFolder::new();
        none.write("auth.json", "{}");
        let gone = DataFolder::new();
        let gone_profile = Profile {
            folder: gone.file("missing"),
            ..gone.profile()
        };

        let connected = |billing| {
            ProfileStatus::Connected(Connection {
                account: None,
                plan: None,
                billing,
            })
        };
        assert_eq!(chatgpt.run(|| status(&chatgpt.profile())), connected(None));
        assert_eq!(
            key.run(|| status(&key.profile())),
            connected(Some(Billing::ApiKey))
        );
        assert!(matches!(
            none.run(|| status(&none.profile())),
            ProfileStatus::NotLoggedIn(_)
        ));
        assert_eq!(
            gone.run(|| status(&gone_profile)),
            ProfileStatus::ConfigFolderNotFound
        );
    }
}
