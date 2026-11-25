//! Made-up keyring entries, read as OTTY reads the system keyring: only
//! the entries the part declares, only once the user allowed the keyring,
//! keys as handles.

use otty_plugin_manifest::Permissions;
use serde_json::Value;

use crate::test_host::HostState;
use crate::test_host::files::replace_keys;
use crate::{HostError, JsonWithKeys, KeyringEntry};

/// The entry of `service` and `user` with its key fields replaced by
/// handles; an entry declared with no key fields is a key as a whole.
pub(super) fn read(
    state: &mut HostState,
    permissions: &Permissions,
    service: &str,
    user: &str,
) -> Result<Option<JsonWithKeys>, HostError> {
    if !permissions.may_read_keyring(service, user) {
        return Err(HostError::NotAllowed(format!(
            "the keyring entry {service} {user}"
        )));
    }

    let Some(text) = state.keyring_entry(service, user)? else {
        return Ok(None);
    };

    let fields = permissions.keyring_fields(service, user);
    if fields.is_empty() {
        let key = state.add_key(text);

        return Ok(Some(JsonWithKeys::new(
            r#"{"$key":0}"#.to_string(),
            vec![key],
        )));
    }

    let mut document: Value = serde_json::from_str(&text).map_err(|_| {
        HostError::Failed(format!(
            "the keyring entry {service} {user} is not JSON"
        ))
    })?;
    let mut keys = Vec::new();
    replace_keys(&mut document, "", &fields, &mut |value| {
        keys.push(state.add_key(value));

        keys.len() - 1
    });

    Ok(Some(JsonWithKeys::new(document.to_string(), keys)))
}

/// The key in the keyring entry `entry`, or in one JSON field of it.
pub(super) fn key_text(
    state: &HostState,
    entry: &KeyringEntry,
) -> Result<String, HostError> {
    let not_found = || {
        HostError::NotFound(format!(
            "the keyring entry {} {}",
            entry.service, entry.user
        ))
    };
    let text = state
        .keyring_entry(&entry.service, &entry.user)?
        .ok_or_else(not_found)?;
    let Some(pointer) = &entry.pointer else {
        return Ok(text);
    };

    let document: Value =
        serde_json::from_str(&text).map_err(|_| not_found())?;

    document
        .pointer(pointer)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(not_found)
}
