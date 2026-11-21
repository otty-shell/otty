//! `keys.open`, the calls of a key handle, and
//! `files.read-json-with-keys`: a plugin gets handles for the key places
//! its called part declares, and learns of a key only what a handle tells.

use std::path::PathBuf;

use wasmtime::component::Resource;

use crate::calls::CallState;
use crate::keys::{
    KeyHandle, KeySource, KeyText, permission_place, place_text,
    read_json_file, read_key, read_source, take_key_fields,
};
use crate::operations::v0_1::otty::host::keys::{
    JsonField, JsonWithKeys, KeyPlace,
};
use crate::operations::v0_1::otty::host::types::HostError;

/// A handle to the key at `place`, which the called part must declare.
/// The key is read once to tell that it is there, then dropped.
pub(crate) fn open(
    state: &mut CallState,
    place: KeyPlace,
) -> Result<Resource<KeyHandle>, HostError> {
    if !state.permissions().may_open_key(&permission_place(&place)) {
        return Err(HostError::NotAllowed(place_text(&place)));
    }

    read_key(&place, state.env(), state.outside())?;

    give_handle(state, place)
}

/// SHA-256 of the key of `key`, lowercase hex.
pub(crate) fn fingerprint(
    state: &mut CallState,
    key: &Resource<KeyHandle>,
) -> Result<String, HostError> {
    Ok(key_of(state, key)?.fingerprint())
}

/// Whether the key of `key` starts with `text`.
pub(crate) fn has_prefix(
    state: &mut CallState,
    key: &Resource<KeyHandle>,
    text: &str,
) -> Result<bool, HostError> {
    Ok(key_of(state, key)?.has_prefix(text))
}

/// The claims of the JWT of `key`; none when it is not a JWT.
pub(crate) fn jwt_claims(
    state: &mut CallState,
    key: &Resource<KeyHandle>,
) -> Result<Option<String>, HostError> {
    Ok(key_of(state, key)?.jwt_claims())
}

/// Whether the user gave the key of `key` in OTTY, rather than OTTY
/// reading it from a place of an agent. The key is not read.
pub(crate) fn given_in_otty(
    state: &mut CallState,
    key: &Resource<KeyHandle>,
) -> Result<bool, HostError> {
    let handle = state
        .table()
        .get(key)
        .map_err(|_| HostError::Failed("no such key handle".to_string()))?;

    Ok(matches!(handle.source(), KeySource::Given(_)))
}

/// The key file `path` as JSON, the key fields the called part declares
/// replaced by `{"$key": n}`.
pub(crate) fn read_json_with_keys(
    state: &mut CallState,
    path: &str,
) -> Result<JsonWithKeys, HostError> {
    let file = PathBuf::from(path);
    if !state.permissions().may_read_with_keys(&file) {
        return Err(HostError::NotAllowed(path.to_string()));
    }

    let mut document = read_json_file(&file)?;
    let fields = state.permissions().key_fields(&file);
    let pointers = take_key_fields(&mut document, &fields);

    let places = pointers.into_iter().map(|pointer| {
        KeyPlace::JsonField(JsonField {
            file: path.to_string(),
            pointer,
        })
    });
    let keys = give_handles(state, places)?;

    Ok(JsonWithKeys {
        json: document.to_string(),
        keys,
    })
}

/// Handles to the keys at `places`, given to the plugin.
pub(crate) fn give_handles(
    state: &mut CallState,
    places: impl IntoIterator<Item = KeyPlace>,
) -> Result<Vec<Resource<KeyHandle>>, HostError> {
    places
        .into_iter()
        .map(|place| give_handle(state, place))
        .collect()
}

/// The key of the handle `key`, read from its place now.
pub(crate) fn key_of(
    state: &mut CallState,
    key: &Resource<KeyHandle>,
) -> Result<KeyText, HostError> {
    let source = state
        .table()
        .get(key)
        .map_err(|_| HostError::Failed("no such key handle".to_string()))?
        .source()
        .clone();

    read_source(&source, state.env(), state.outside())
}

/// A handle to the key at `place`, given to the plugin.
pub(crate) fn give_handle(
    state: &mut CallState,
    place: KeyPlace,
) -> Result<Resource<KeyHandle>, HostError> {
    state
        .table()
        .push(KeyHandle::new(place))
        .map_err(|_| HostError::Failed("too many key handles".to_string()))
}

#[cfg(test)]
mod tests {
    use mockall::predicate::eq;
    use otty_plugin_manifest::{Environment, PartRef};
    use rusqlite::Connection;

    use super::*;
    use crate::https::MockHttpsSend;
    use crate::operations::v0_1::otty::host::keys::{
        KeyringEntry, SqliteField,
    };
    use crate::operations::v0_1::otty::host::sqlite::{self, Value};
    use crate::system_keyring::MockKeyringRead;
    use crate::testing::{TempDir, engine_with, in_call, load_keys_plugin};

    const KEY: &str = "ex-made-up-key";
    /// The fingerprint of [`KEY`], as `KeyFingerprint` of `otty-agents`
    /// gives it.
    const FINGERPRINT: &str =
        "27c8d30ec451e808415d2307396308459e728ccef0adc5a41908fccae8a2e1ae";
    /// A made-up JWT whose claims are an email and an expiry.
    const JWT: &str = "eyJhbGciOiJub25lIn0.eyJlbWFpbCI6ImRldkBleGFtcGxlLmNvbSIsImV4cCI6MTkwMDAwMDAwMH0.made-up-signature";

    /// A home folder with a key file and a database of the test agent.
    struct Home {
        folder: TempDir,
    }

    impl Home {
        fn path(&self, relative: &str) -> String {
            self.folder.path().join(relative).display().to_string()
        }

        fn new() -> Self {
            let folder = TempDir::new();
            let agent = folder.path().join(".test-agent");
            std::fs::create_dir_all(&agent).expect("the folder");
            std::fs::write(
                agent.join("auth.json"),
                format!(
                    r#"{{"work": {{"key": "{KEY}", "name": "Work"}}, "token": "{JWT}", "other": "ex-other"}}"#
                ),
            )
            .expect("the key file");
            let connection =
                Connection::open(agent.join("data.db")).expect("a database");
            connection
                .execute_batch(&format!(
                    "CREATE TABLE api_key (secret TEXT, other TEXT);
                     INSERT INTO api_key VALUES ('{KEY}', 'ex-other');
                     CREATE TABLE credential (value TEXT);
                     INSERT INTO credential VALUES
                         ('{{\"type\": \"api\", \"key\": \"{KEY}\"}}');"
                ))
                .expect("the database");

            Self { folder }
        }

        fn env(&self) -> Environment {
            Environment::new(self.folder.path().to_path_buf())
                .with_var("TEST_AGENT_KEY", KEY)
                .with_var("OTHER_KEY", "ex-other")
        }

        fn json_field(&self, pointer: &str) -> KeyPlace {
            KeyPlace::JsonField(JsonField {
                file: self.path(".test-agent/auth.json"),
                pointer: pointer.to_string(),
            })
        }

        fn sqlite_field(
            &self,
            table: &str,
            column: &str,
            pointer: Option<&str>,
        ) -> KeyPlace {
            KeyPlace::SqliteField(SqliteField {
                database: self.path(".test-agent/data.db"),
                table: table.to_string(),
                column: column.to_string(),
                row_id: 1,
                pointer: pointer.map(str::to_string),
            })
        }
    }

    fn keyring_place(user: &str, pointer: Option<&str>) -> KeyPlace {
        KeyPlace::Keyring(KeyringEntry {
            service: "test-agent".to_string(),
            user: user.to_string(),
            pointer: pointer.map(str::to_string),
        })
    }

    /// A keyring with the entry `work` holding [`KEY`] in a JSON field.
    fn keyring() -> MockKeyringRead {
        let mut keyring = MockKeyringRead::new();
        keyring
            .expect_password()
            .with(eq("test-agent"), eq("work"))
            .returning(|_, _| Ok(Some(format!(r#"{{"key": "{KEY}"}}"#))));

        keyring
    }

    /// What the plugin gets of the key at `place` as the test agent: the
    /// handle's place, which is all a handle holds, and the fingerprint of
    /// its key.
    fn opened(
        home: &Home,
        place: &KeyPlace,
        keyring_allowed: bool,
    ) -> Result<(KeyPlace, String), HostError> {
        let cache = TempDir::new();
        let engine = engine_with(&cache, MockHttpsSend::new(), keyring());
        engine.set_keyring_allowed(keyring_allowed);
        let plugin = load_keys_plugin(&engine, &home.env());

        in_call(&plugin, &PartRef::agent("test-agent"), |state| {
            let handle = open(state, place.clone())?;
            let held =
                state.table().get(&handle).expect("held").source().clone();
            let KeySource::Place(held) = held else {
                panic!("a handle of a place: {held:?}");
            };

            Ok((held, fingerprint(state, &handle)?))
        })
    }

    #[test]
    fn a_plugin_gets_a_handle_never_the_text_from_every_kind_of_place() {
        let home = Home::new();
        let places = [
            home.json_field("/work/key"),
            KeyPlace::EnvVar("TEST_AGENT_KEY".to_string()),
            keyring_place("work", Some("/key")),
            home.sqlite_field("api_key", "secret", None),
            home.sqlite_field("credential", "value", Some("/key")),
        ];

        for place in places {
            let (held, fingerprint) =
                opened(&home, &place, true).expect("a handle");

            assert_eq!(format!("{held:?}"), format!("{place:?}"));
            assert!(!format!("{held:?}").contains(KEY));
            assert_eq!(fingerprint, FINGERPRINT, "{place:?}");
        }
    }

    #[test]
    fn a_query_gives_key_columns_and_json_key_fields_as_handles() {
        let home = Home::new();
        let cache = TempDir::new();
        let engine =
            engine_with(&cache, MockHttpsSend::new(), MockKeyringRead::new());
        let plugin = load_keys_plugin(&engine, &home.env());
        let database = home.path(".test-agent/data.db");

        let (rows, fingerprints) =
            in_call(&plugin, &PartRef::agent("test-agent"), |state| {
                let mut query = |sql: &str| {
                    sqlite::Host::query(
                        state,
                        database.clone(),
                        sql.to_string(),
                        Vec::new(),
                    )
                    .expect("the rows")
                };
                let mut rows =
                    query("SELECT rowid, secret, other FROM api_key");
                rows.extend(query("SELECT rowid, value FROM credential"));

                let fingerprints: Vec<String> = rows
                    .iter()
                    .map(|row| {
                        let key = match &row[1] {
                            Value::Key(key) => key,
                            Value::JsonWithKeys(document) => &document.keys[0],
                            other => panic!("a key, not {other:?}"),
                        };

                        fingerprint(state, key).expect("a key")
                    })
                    .collect();

                (format!("{rows:?}"), fingerprints)
            });

        assert!(!rows.contains(KEY), "{rows}");
        assert!(
            rows.contains(r#"{\"key\":{\"$key\":0},\"type\":\"api\"}"#),
            "{rows}"
        );
        assert!(rows.contains("Text(\"ex-other\")"), "{rows}");
        assert_eq!(fingerprints, [FINGERPRINT, FINGERPRINT]);
    }

    #[test]
    fn a_key_place_the_part_does_not_declare_is_refused() {
        let home = Home::new();
        let places = [
            home.json_field("/other"),
            KeyPlace::EnvVar("OTHER_KEY".to_string()),
            keyring_place("other", None),
            home.sqlite_field("api_key", "other", None),
            home.sqlite_field("credential", "value", Some("/type")),
            KeyPlace::JsonField(JsonField {
                file: home.path(".other/auth.json"),
                pointer: "/work/key".to_string(),
            }),
        ];

        for place in places {
            let opened = opened(&home, &place, true);

            assert!(
                matches!(&opened, Err(HostError::NotAllowed(text)) if !text.contains(KEY)),
                "{place:?}: {opened:?}"
            );
        }
    }

    #[test]
    fn a_key_place_declared_only_by_another_part_is_refused() {
        let home = Home::new();
        let cache = TempDir::new();
        let engine =
            engine_with(&cache, MockHttpsSend::new(), MockKeyringRead::new());
        let plugin = load_keys_plugin(&engine, &home.env());

        let opened =
            in_call(&plugin, &PartRef::provider("test-cloud"), |state| {
                (
                    open(state, home.json_field("/work/key")).map(|_| ()),
                    read_json_with_keys(
                        state,
                        &home.path(".test-agent/auth.json"),
                    )
                    .map(|_| ()),
                )
            });

        assert!(matches!(opened.0, Err(HostError::NotAllowed(_))));
        assert!(matches!(opened.1, Err(HostError::NotAllowed(_))));
    }

    #[test]
    fn a_keyring_key_is_refused_before_the_users_allow() {
        let home = Home::new();

        let opened = opened(&home, &keyring_place("work", Some("/key")), false);

        assert!(matches!(opened, Err(HostError::NotAllowed(_))));
    }

    #[test]
    fn a_handle_tells_a_prefix_and_the_claims_of_a_jwt_only() {
        let home = Home::new();
        let cache = TempDir::new();
        let engine =
            engine_with(&cache, MockHttpsSend::new(), MockKeyringRead::new());
        let plugin = load_keys_plugin(&engine, &home.env());

        let (claims, is_jwt, other_prefix, plain_claims) =
            in_call(&plugin, &PartRef::agent("test-agent"), |state| {
                let token =
                    open(state, home.json_field("/token")).expect("a handle");
                let key = open(state, home.json_field("/work/key"))
                    .expect("a handle");

                (
                    jwt_claims(state, &token),
                    has_prefix(state, &token, "eyJ"),
                    has_prefix(state, &token, "sk-"),
                    jwt_claims(state, &key),
                )
            });

        assert_eq!(
            claims.expect("the claims").as_deref(),
            Some(r#"{"email":"dev@example.com","exp":1900000000}"#)
        );
        assert!(is_jwt.expect("a prefix"));
        assert!(!other_prefix.expect("a prefix"));
        assert_eq!(plain_claims.expect("no claims"), None);
    }

    /// Issue 26: a handle tells whether the user gave its key in OTTY,
    /// and nothing of the place an agent keeps it in.
    #[test]
    fn a_handle_tells_whether_its_key_was_given_in_otty() {
        let home = Home::new();
        let cache = TempDir::new();
        let engine =
            engine_with(&cache, MockHttpsSend::new(), MockKeyringRead::new());
        let plugin = load_keys_plugin(&engine, &home.env());
        let given_key = KeyText::new(KEY).expect("a filled key");

        let (found, given) =
            in_call(&plugin, &PartRef::agent("test-agent"), |state| {
                let found = open(state, home.json_field("/work/key"))
                    .expect("a handle");
                let given = state
                    .table()
                    .push(KeyHandle::of(KeySource::Given(given_key)))
                    .expect("a handle");

                (given_in_otty(state, &found), given_in_otty(state, &given))
            });

        assert!(matches!(found, Ok(false)), "{found:?}");
        assert!(matches!(given, Ok(true)), "{given:?}");
    }

    #[test]
    fn a_key_file_comes_with_handles_in_place_of_its_keys() {
        let home = Home::new();
        let cache = TempDir::new();
        let engine =
            engine_with(&cache, MockHttpsSend::new(), MockKeyringRead::new());
        let plugin = load_keys_plugin(&engine, &home.env());
        let auth = home.path(".test-agent/auth.json");

        let (json, fingerprints) =
            in_call(&plugin, &PartRef::agent("test-agent"), |state| {
                let document =
                    read_json_with_keys(state, &auth).expect("the file");
                let fingerprints: Vec<String> = document
                    .keys
                    .iter()
                    .map(|key| fingerprint(state, key).expect("a key"))
                    .collect();

                (document.json, fingerprints)
            });

        assert_eq!(
            json,
            r#"{"other":"ex-other","token":{"$key":0},"work":{"key":{"$key":1},"name":"Work"}}"#
        );
        assert!(!json.contains(KEY));
        assert_eq!(fingerprints.len(), 2);
        assert_eq!(fingerprints[1], FINGERPRINT);
    }

    #[test]
    fn a_missing_key_file_is_not_found() {
        let home = Home::new();
        let cache = TempDir::new();
        let engine =
            engine_with(&cache, MockHttpsSend::new(), MockKeyringRead::new());
        let plugin = load_keys_plugin(&engine, &home.env());
        std::fs::remove_file(home.path(".test-agent/auth.json"))
            .expect("removed");

        let read = in_call(&plugin, &PartRef::agent("test-agent"), |state| {
            read_json_with_keys(state, &home.path(".test-agent/auth.json"))
                .map(|_| ())
        });

        assert!(matches!(read, Err(HostError::NotFound(_))));
    }
}
