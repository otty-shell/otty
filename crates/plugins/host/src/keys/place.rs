//! Key places: reading the key at one right before it is used, the JSON
//! files keys sit in, and how places are named in permissions and errors.

use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use otty_plugin_manifest::{Environment, KeyPlace as PermissionPlace};
use rusqlite::{Connection, OpenFlags};
use serde_json::Value;

use crate::keys::key_text::KeyText;
use crate::operations::v0_1::otty::host::keys::{
    KeyPlace, KeyringEntry, SqliteField,
};
use crate::operations::v0_1::otty::host::types::HostError;
use crate::outside::Outside;

/// The most bytes of a JSON key file (spec "Host operations").
const MOST_KEY_FILE_BYTES: u64 = 1024 * 1024;

/// The key at `place`, read now. Variables come from `env`, keyring
/// entries from `outside` once the user allowed the keyring.
pub(crate) fn read_key(
    place: &KeyPlace,
    env: &Environment,
    outside: &Outside,
) -> Result<KeyText, HostError> {
    let text = match place {
        KeyPlace::JsonField(field) => {
            let file = PathBuf::from(&field.file);
            let document = read_json_file(&file)?;

            text_at(&document, &field.pointer)
        },
        KeyPlace::EnvVar(name) => env.var(name).map(str::to_string),
        KeyPlace::Keyring(entry) => keyring_text(entry, outside)?,
        KeyPlace::SqliteField(field) => sqlite_text(field)?,
    };

    text.as_deref()
        .and_then(KeyText::new)
        .ok_or_else(|| HostError::NotFound(place_text(place)))
}

/// The JSON document of the key file `path`, at most 1 MiB.
pub(crate) fn read_json_file(path: &Path) -> Result<Value, HostError> {
    let mut bytes = Vec::new();
    File::open(path)
        .and_then(|file| {
            file.take(MOST_KEY_FILE_BYTES + 1).read_to_end(&mut bytes)
        })
        .map_err(|error| match error.kind() {
            io::ErrorKind::NotFound => {
                HostError::NotFound(path.display().to_string())
            },
            _ => HostError::Failed(format!("{}: {error}", path.display())),
        })?;

    if bytes.len() as u64 > MOST_KEY_FILE_BYTES {
        return Err(HostError::TooBig(format!(
            "{} is more than 1 MiB",
            path.display()
        )));
    }

    serde_json::from_slice(&bytes).map_err(|_| {
        HostError::Failed(format!("{} is not JSON", path.display()))
    })
}

/// `place` as the permissions name it.
pub(crate) fn permission_place(place: &KeyPlace) -> PermissionPlace {
    match place {
        KeyPlace::JsonField(field) => PermissionPlace::JsonField {
            file: PathBuf::from(&field.file),
            pointer: field.pointer.clone(),
        },
        KeyPlace::EnvVar(name) => PermissionPlace::EnvVar(name.clone()),
        KeyPlace::Keyring(entry) => PermissionPlace::Keyring {
            service: entry.service.clone(),
            user: entry.user.clone(),
            field: entry.pointer.clone(),
        },
        KeyPlace::SqliteField(field) => PermissionPlace::SqliteField {
            database: PathBuf::from(&field.database),
            table: field.table.clone(),
            column: field.column.clone(),
            pointer: field.pointer.clone(),
        },
    }
}

/// `place` in plain words, for an error: where the key is, never the key.
pub(crate) fn place_text(place: &KeyPlace) -> String {
    match place {
        KeyPlace::JsonField(field) => {
            format!("the field {} of {}", field.pointer, field.file)
        },
        KeyPlace::EnvVar(name) => format!("the variable {name}"),
        KeyPlace::Keyring(entry) => {
            let field = entry
                .pointer
                .as_ref()
                .map(|pointer| format!("the field {pointer} of "))
                .unwrap_or_default();

            format!("{field}the keyring entry {} {}", entry.service, entry.user)
        },
        KeyPlace::SqliteField(field) => {
            let json_field = field
                .pointer
                .as_ref()
                .map(|pointer| format!(" field {pointer}"))
                .unwrap_or_default();

            format!(
                "{}.{}{json_field} of row {} in {}",
                field.table, field.column, field.row_id, field.database
            )
        },
    }
}

/// The text at `pointer` of `document`, when it is text.
fn text_at(document: &Value, pointer: &str) -> Option<String> {
    document
        .pointer(pointer)
        .and_then(Value::as_str)
        .map(str::to_string)
}

/// The text of a keyring entry, or of one JSON field of it.
fn keyring_text(
    entry: &KeyringEntry,
    outside: &Outside,
) -> Result<Option<String>, HostError> {
    let Some(text) = outside.keyring_entry(&entry.service, &entry.user)? else {
        return Ok(None);
    };
    let Some(pointer) = &entry.pointer else {
        return Ok(Some(text));
    };

    let document: Value = serde_json::from_str(&text).map_err(|_| {
        HostError::Failed(format!(
            "the keyring entry {} {} is not JSON",
            entry.service, entry.user
        ))
    })?;

    Ok(text_at(&document, pointer))
}

/// The text of a column of one row, or of one JSON field of it, read from
/// the database opened read-only.
fn sqlite_text(field: &SqliteField) -> Result<Option<String>, HostError> {
    let failed = |_: rusqlite::Error| {
        HostError::Failed(format!(
            "{}.{} of {} could not be read",
            field.table, field.column, field.database
        ))
    };
    let flags =
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let connection =
        Connection::open_with_flags(&field.database, flags).map_err(failed)?;
    connection
        .pragma_update(None, "query_only", true)
        .map_err(failed)?;

    let sql = format!(
        "SELECT {} FROM {} WHERE rowid = ?1",
        quoted(&field.column),
        quoted(&field.table)
    );
    let text: Option<String> =
        match connection.query_row(&sql, [field.row_id], |row| row.get(0)) {
            Ok(text) => text,
            Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
            Err(error) => return Err(failed(error)),
        };

    let (Some(text), Some(pointer)) = (text.as_deref(), &field.pointer) else {
        return Ok(text);
    };
    let document: Value = serde_json::from_str(text).map_err(|_| {
        HostError::Failed(format!(
            "{}.{} of {} is not JSON",
            field.table, field.column, field.database
        ))
    })?;

    Ok(text_at(&document, pointer))
}

/// `name` as an SQL identifier in double quotes.
fn quoted(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

#[cfg(test)]
mod tests {
    use mockall::predicate::eq;

    use super::*;
    use crate::https::MockHttpsSend;
    use crate::operations::v0_1::otty::host::keys::JsonField;
    use crate::system_keyring::MockKeyringRead;
    use crate::testing::TempDir;

    const KEY: &str = "ex-made-up-key";

    fn outside(keyring: MockKeyringRead) -> Outside {
        Outside::new(Box::new(MockHttpsSend::new()), Box::new(keyring))
    }

    fn no_keyring() -> Outside {
        outside(MockKeyringRead::new())
    }

    fn env() -> Environment {
        Environment::new(PathBuf::from("/home/test-user"))
    }

    fn json_field(file: &Path, pointer: &str) -> KeyPlace {
        KeyPlace::JsonField(JsonField {
            file: file.display().to_string(),
            pointer: pointer.to_string(),
        })
    }

    fn sqlite_field(
        database: &Path,
        table: &str,
        column: &str,
        pointer: Option<&str>,
    ) -> KeyPlace {
        KeyPlace::SqliteField(SqliteField {
            database: database.display().to_string(),
            table: table.to_string(),
            column: column.to_string(),
            row_id: 1,
            pointer: pointer.map(str::to_string),
        })
    }

    /// A database with a key column and a JSON column holding a key.
    fn database(folder: &TempDir) -> PathBuf {
        let path = folder.path().join("data.db");
        let connection = Connection::open(&path).expect("a database");
        connection
            .execute_batch(&format!(
                "CREATE TABLE api_key (secret TEXT);
                 INSERT INTO api_key VALUES ('  {KEY}  ');
                 CREATE TABLE credential (value TEXT);
                 INSERT INTO credential VALUES
                     ('{{\"type\": \"api\", \"key\": \"{KEY}\"}}');"
            ))
            .expect("the fixture");

        path
    }

    #[test]
    fn a_key_is_read_from_a_field_of_a_json_file() {
        let folder = TempDir::new();
        let auth = folder.path().join("auth.json");
        std::fs::write(&auth, format!(r#"{{"work": {{"key": "{KEY}"}}}}"#))
            .expect("the file");

        let key =
            read_key(&json_field(&auth, "/work/key"), &env(), &no_keyring());
        let missing =
            read_key(&json_field(&auth, "/home/key"), &env(), &no_keyring());

        assert_eq!(key.expect("the key").as_str(), KEY);
        assert!(matches!(missing, Err(HostError::NotFound(_))));
    }

    #[test]
    fn a_key_is_read_from_a_variable() {
        let env = env().with_var("MADE_UP_KEY", KEY);

        let key = read_key(
            &KeyPlace::EnvVar("MADE_UP_KEY".to_string()),
            &env,
            &no_keyring(),
        );
        let missing = read_key(
            &KeyPlace::EnvVar("OTHER_KEY".to_string()),
            &env,
            &no_keyring(),
        );

        assert_eq!(key.expect("the key").as_str(), KEY);
        assert!(matches!(missing, Err(HostError::NotFound(_))));
    }

    #[test]
    fn a_key_is_read_from_a_keyring_entry_once_allowed() {
        let mut keyring = MockKeyringRead::new();
        keyring
            .expect_password()
            .with(eq("made-up"), eq("work"))
            .returning(|_, _| Ok(Some(format!(r#"{{"key": "{KEY}"}}"#))));
        let outside = outside(keyring);
        let place = KeyPlace::Keyring(KeyringEntry {
            service: "made-up".to_string(),
            user: "work".to_string(),
            pointer: Some("/key".to_string()),
        });

        let before = read_key(&place, &env(), &outside);
        outside.set_keyring_allowed(true);
        let after = read_key(&place, &env(), &outside);

        assert!(matches!(before, Err(HostError::NotAllowed(_))));
        assert_eq!(after.expect("the key").as_str(), KEY);
    }

    #[test]
    fn a_key_is_read_from_a_column_and_from_a_field_of_a_json_column() {
        let folder = TempDir::new();
        let database = database(&folder);

        let column = read_key(
            &sqlite_field(&database, "api_key", "secret", None),
            &env(),
            &no_keyring(),
        );
        let json_field = read_key(
            &sqlite_field(&database, "credential", "value", Some("/key")),
            &env(),
            &no_keyring(),
        );
        let no_field = read_key(
            &sqlite_field(&database, "credential", "value", Some("/access")),
            &env(),
            &no_keyring(),
        );

        assert_eq!(column.expect("the key").as_str(), KEY);
        assert_eq!(json_field.expect("the key").as_str(), KEY);
        assert!(matches!(no_field, Err(HostError::NotFound(_))));
    }

    #[test]
    fn an_error_names_the_place_and_never_the_key() {
        let folder = TempDir::new();
        let database = database(&folder);
        let places = [
            sqlite_field(&database, "credential", "value", Some("/missing")),
            sqlite_field(&database, "no_table", "value", None),
            KeyPlace::EnvVar("OTHER_KEY".to_string()),
        ];

        for place in places {
            let error = read_key(&place, &env(), &no_keyring())
                .expect_err("no key there");

            let text = format!("{error:?}");
            assert!(!text.contains(KEY), "{text}");
        }
    }
}
