//! Made-up keys: their handles, fingerprints and JWT claims, as OTTY
//! gives them.

use std::fs;
use std::path::PathBuf;

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD_INDIFFERENT;
use otty_plugin_manifest::{KeyPlace as Place, Permissions};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::test_host::files::io_error;
use crate::test_host::{HostState, keyring, sqlite};
use crate::{HostError, Key, KeyPlace};

/// SHA-256 of `key` without the spaces around it, lowercase hex, as OTTY
/// computes it.
pub(super) fn fingerprint(key: &str) -> String {
    Sha256::digest(key.trim().as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// The claims of `token` when it is a JWT: its middle part, decoded.
pub(super) fn jwt_claims(token: &str) -> Option<String> {
    let mut parts = token.trim().split('.');
    let (Some(_), Some(payload), Some(_), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return None;
    };

    let bytes = URL_SAFE_NO_PAD_INDIFFERENT.decode(payload).ok()?;
    let claims: Value = serde_json::from_slice(&bytes).ok()?;

    claims.is_object().then(|| claims.to_string())
}

/// A handle to the key at `place`, which the part must declare.
pub(super) fn open(
    state: &mut HostState,
    permissions: &Permissions,
    place: &KeyPlace,
) -> Result<Key, HostError> {
    if !permissions.may_open_key(&permission_place(place)) {
        return Err(HostError::NotAllowed(format!("{place:?}")));
    }

    let value = match place {
        KeyPlace::JsonField(field) => {
            json_field(&PathBuf::from(&field.file), &field.pointer)?
        },
        KeyPlace::EnvVar(name) => state
            .var(name)
            .map(str::to_string)
            .ok_or_else(|| HostError::NotFound(name.clone()))?,
        KeyPlace::Keyring(entry) => keyring::key_text(state, entry)?,
        KeyPlace::SqliteField(field) => sqlite::key_field(field)?,
    };

    Ok(state.add_key(value))
}

/// The text at `pointer` of the JSON file `file`.
fn json_field(file: &PathBuf, pointer: &str) -> Result<String, HostError> {
    let text =
        fs::read_to_string(file).map_err(|error| io_error(&error, file))?;
    let document: Value = serde_json::from_str(&text).map_err(|_| {
        HostError::Failed(format!("{} is not JSON", file.display()))
    })?;

    document
        .pointer(pointer)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| {
            HostError::NotFound(format!("{} {pointer}", file.display()))
        })
}

/// `place` as the permissions name it.
fn permission_place(place: &KeyPlace) -> Place {
    match place {
        KeyPlace::JsonField(field) => Place::JsonField {
            file: PathBuf::from(&field.file),
            pointer: field.pointer.clone(),
        },
        KeyPlace::EnvVar(name) => Place::EnvVar(name.clone()),
        KeyPlace::Keyring(entry) => Place::Keyring {
            service: entry.service.clone(),
            user: entry.user.clone(),
            field: entry.pointer.clone(),
        },
        KeyPlace::SqliteField(field) => Place::SqliteField {
            database: PathBuf::from(&field.database),
            table: field.table.clone(),
            column: field.column.clone(),
            pointer: field.pointer.clone(),
        },
    }
}
