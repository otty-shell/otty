//! `keyring.read`: a keyring entry the called part declares, as JSON with
//! its key fields replaced by handles, once the user allowed the keyring.

use serde_json::Value;

use crate::calls::CallState;
use crate::keys::{key_value, take_key_fields};
use crate::operations::keys::give_handles;
use crate::operations::v0_1::otty::host::keys::{
    JsonWithKeys, KeyPlace, KeyringEntry,
};
use crate::operations::v0_1::otty::host::types::HostError;

/// The entry of `service` and `user`; none when there is no such entry.
/// An entry declared with no key fields is a key as a whole and comes as
/// `{"$key": 0}`.
pub(crate) fn read(
    state: &mut CallState,
    service: &str,
    user: &str,
) -> Result<Option<JsonWithKeys>, HostError> {
    if !state.permissions().may_read_keyring(service, user) {
        return Err(HostError::NotAllowed(format!(
            "the keyring entry {service} {user}"
        )));
    }

    let Some(text) = state.outside().keyring_entry(service, user)? else {
        return Ok(None);
    };

    let fields = state.permissions().keyring_fields(service, user);
    let (document, pointers) = if fields.is_empty() {
        (key_value(0), vec![None])
    } else {
        let mut document: Value =
            serde_json::from_str(&text).map_err(|_| {
                HostError::Failed(format!(
                    "the keyring entry {service} {user} is not JSON"
                ))
            })?;
        let pointers = take_key_fields(&mut document, &fields);

        (document, pointers.into_iter().map(Some).collect())
    };
    drop(text);

    let places = pointers.into_iter().map(|pointer| {
        KeyPlace::Keyring(KeyringEntry {
            service: service.to_string(),
            user: user.to_string(),
            pointer,
        })
    });
    let keys = give_handles(state, places)?;

    Ok(Some(JsonWithKeys {
        json: document.to_string(),
        keys,
    }))
}

#[cfg(test)]
mod tests {
    use mockall::predicate::eq;
    use otty_plugin_manifest::{Environment, PartRef};

    use super::*;
    use crate::https::MockHttpsSend;
    use crate::operations::keys::fingerprint;
    use crate::system_keyring::MockKeyringRead;
    use crate::testing::{
        TempDir, engine_with, in_call, load_keys_plugin, test_env,
    };

    const KEY: &str = "ex-made-up-key";
    /// The fingerprint of [`KEY`].
    const FINGERPRINT: &str =
        "27c8d30ec451e808415d2307396308459e728ccef0adc5a41908fccae8a2e1ae";

    /// A keyring with the entries `work` (JSON with a key field), `plain`
    /// (a key as a whole) and `other`; `missing` has no entry.
    fn keyring() -> MockKeyringRead {
        let mut keyring = MockKeyringRead::new();
        keyring
            .expect_password()
            .with(eq("test-agent"), eq("work"))
            .returning(|_, _| {
                Ok(Some(format!(r#"{{"key": "{KEY}", "name": "Work"}}"#)))
            });
        keyring
            .expect_password()
            .with(eq("test-agent"), eq("plain"))
            .returning(|_, _| Ok(Some(format!("{KEY}\n"))));
        keyring
            .expect_password()
            .with(eq("test-agent"), eq("other"))
            .never();

        keyring
    }

    /// What the test agent reads of the entry of `user`, with the keyring
    /// allowed or not: the JSON and the fingerprints of its keys.
    fn read_entry(
        user: &str,
        keyring_allowed: bool,
        env: &Environment,
    ) -> Result<Option<(String, Vec<String>)>, HostError> {
        let cache = TempDir::new();
        let engine = engine_with(&cache, MockHttpsSend::new(), keyring());
        engine.set_keyring_allowed(keyring_allowed);
        let plugin = load_keys_plugin(&engine, env);

        in_call(&plugin, &PartRef::agent("test-agent"), |state| {
            let Some(entry) = read(state, "test-agent", user)? else {
                return Ok(None);
            };
            let fingerprints = entry
                .keys
                .iter()
                .map(|key| fingerprint(state, key))
                .collect::<Result<_, _>>()?;

            Ok(Some((entry.json, fingerprints)))
        })
    }

    #[test]
    fn an_entry_comes_with_handles_in_place_of_its_key_fields() {
        let entry = read_entry("work", true, &test_env()).expect("the entry");

        let (json, fingerprints) = entry.expect("an entry");
        assert_eq!(json, r#"{"key":{"$key":0},"name":"Work"}"#);
        assert_eq!(fingerprints, [FINGERPRINT]);
    }

    #[test]
    fn an_entry_with_no_key_fields_is_a_key_as_a_whole() {
        let entry = read_entry("plain", true, &test_env()).expect("the entry");

        let (json, fingerprints) = entry.expect("an entry");
        assert_eq!(json, r#"{"$key":0}"#);
        assert_eq!(fingerprints, [FINGERPRINT]);
    }

    #[test]
    fn the_keyring_is_refused_before_the_users_allow() {
        let entry = read_entry("work", false, &test_env());

        assert!(matches!(entry, Err(HostError::NotAllowed(_))));
    }

    #[test]
    fn an_entry_the_part_does_not_declare_is_refused() {
        let entry = read_entry("other", true, &test_env());

        assert!(matches!(entry, Err(HostError::NotAllowed(_))));
    }

    #[test]
    fn a_provider_part_reads_no_keyring_entry() {
        let cache = TempDir::new();
        let engine = engine_with(&cache, MockHttpsSend::new(), keyring());
        engine.set_keyring_allowed(true);
        let plugin = load_keys_plugin(&engine, &test_env());

        let entry =
            in_call(&plugin, &PartRef::provider("test-cloud"), |state| {
                read(state, "test-agent", "work").map(|_| ())
            });

        assert!(matches!(entry, Err(HostError::NotAllowed(_))));
    }
}
