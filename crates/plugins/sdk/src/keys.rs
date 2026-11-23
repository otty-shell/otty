//! Keys and login tokens: a plugin holds a handle and never the key.

use std::fmt;
#[cfg(target_arch = "wasm32")]
use std::mem::ManuallyDrop;

#[cfg(target_arch = "wasm32")]
use crate::bindings::host::otty::host::keys as wit;
use crate::{HostError, KeyPlace};

/// A key or a login token the plugin may use but never read (spec
/// Decision 5). It goes into a request with
/// [`Request::header_with_key`](crate::http::Request::header_with_key).
pub struct Key {
    #[cfg(target_arch = "wasm32")]
    handle: wit::Key,
    #[cfg(not(target_arch = "wasm32"))]
    id: usize,
}

impl Key {
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn wasm(&self) -> &wit::Key {
        &self.handle
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn id(&self) -> usize {
        self.id
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn from_wasm(handle: wit::Key) -> Self {
        Self { handle }
    }

    /// The key OTTY lends a provider call: the handle stays OTTY's, so it
    /// is never dropped here.
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn lent(key: &wit::Key) -> ManuallyDrop<Self> {
        // SAFETY: the handle is a borrow OTTY gave for this call. It is
        // used only while the call runs and `ManuallyDrop` keeps it from
        // being dropped, which only its owner may do.
        let handle = unsafe { wit::Key::from_handle(key.handle()) };

        ManuallyDrop::new(Self { handle })
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn from_test_host(id: usize) -> Self {
        Self { id }
    }

    /// SHA-256 of the key, lowercase hex: the same key found in two places
    /// gives the same fingerprint.
    pub fn fingerprint(&self) -> Result<String, HostError> {
        #[cfg(target_arch = "wasm32")]
        {
            self.handle.fingerprint()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            crate::test_host::key_fingerprint(self.id)
        }
    }

    /// Whether the key starts with `text`, such as `sk-ant-admin`.
    pub fn has_prefix(&self, text: &str) -> Result<bool, HostError> {
        #[cfg(target_arch = "wasm32")]
        {
            self.handle.has_prefix(text)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            crate::test_host::key_has_prefix(self.id, text)
        }
    }

    /// The claims of a JWT, the decoded middle part, as JSON text; `None`
    /// when the key is not a JWT. Never the signature or the token.
    pub fn jwt_claims(&self) -> Result<Option<String>, HostError> {
        #[cfg(target_arch = "wasm32")]
        {
            self.handle.jwt_claims()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            crate::test_host::key_jwt_claims(self.id)
        }
    }

    /// Whether the user gave the key in OTTY: one given for an
    /// [`NeedAction::AskKey`](crate::NeedAction) slot. `false` for a key
    /// read from a place of an agent: a file field, a variable, a keyring
    /// entry or a database. A refused key given in OTTY is asked for again
    /// under its slot; one of an agent is replaced where the agent keeps
    /// it.
    pub fn is_given_in_otty(&self) -> Result<bool, HostError> {
        #[cfg(target_arch = "wasm32")]
        {
            self.handle.given_in_otty()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            crate::test_host::key_is_given_in_otty(self.id)
        }
    }
}

impl fmt::Debug for Key {
    /// Only the handle number: a key is never shown.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        #[cfg(target_arch = "wasm32")]
        let number = self.handle.handle() as usize;
        #[cfg(not(target_arch = "wasm32"))]
        let number = self.id;

        write!(formatter, "Key(#{number})")
    }
}

/// A JSON document whose key fields are replaced by `{"$key": n}`, where
/// `n` is the index of the key in [`key`](Self::key).
#[derive(Debug)]
pub struct JsonWithKeys {
    json: String,
    keys: Vec<Key>,
}

impl JsonWithKeys {
    /// The JSON text, with `{"$key": n}` in place of each key.
    pub fn json(&self) -> &str {
        &self.json
    }

    /// The key that `{"$key": index}` stands for.
    pub fn key(&self, index: usize) -> Option<&Key> {
        self.keys.get(index)
    }

    pub(crate) fn new(json: String, keys: Vec<Key>) -> Self {
        Self { json, keys }
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn from_wasm(document: wit::JsonWithKeys) -> Self {
        let keys = document.keys.into_iter().map(Key::from_wasm).collect();

        Self::new(document.json, keys)
    }

    /// The JSON text and its keys.
    pub fn into_parts(self) -> (String, Vec<Key>) {
        (self.json, self.keys)
    }
}

/// A key the user gave, under the `slot` a need of the part asked for it
/// with [`NeedAction::AskKey`](crate::NeedAction), such as the admin key
/// of an account.
pub struct GivenKey {
    slot: String,
    key: Key,
}

impl GivenKey {
    /// The part's own name for the key, such as `admin`.
    pub fn slot(&self) -> &str {
        &self.slot
    }

    /// The handle to the key.
    pub fn key(&self) -> &Key {
        &self.key
    }

    /// The key a call is given under `slot`.
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn of_call(slot: String, key: Key) -> Self {
        Self { slot, key }
    }

    /// The key `key` given under `slot`, for the tests of a plugin; the
    /// handle comes from [`TestHost::key`](crate::test_host::TestHost::key).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new(slot: &str, key: Key) -> Self {
        Self {
            slot: slot.to_string(),
            key,
        }
    }
}

/// A handle to the key at `place`, which must be a key place the called
/// part declares in `keys`.
pub fn open(place: &KeyPlace) -> Result<Key, HostError> {
    #[cfg(target_arch = "wasm32")]
    {
        wit::open(place).map(Key::from_wasm)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        crate::test_host::open_key(place)
    }
}

/// The key of `given` under `slot`, when the user gave one, such as the
/// admin key a part asked for under `admin`.
pub fn given_key<'a>(given: &'a [GivenKey], slot: &str) -> Option<&'a Key> {
    given
        .iter()
        .find(|given| given.slot() == slot)
        .map(GivenKey::key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_host::TestHost;
    use crate::testing::MadeUpPlugin;

    #[test]
    fn a_given_key_is_found_by_its_slot_only() {
        let plugin = MadeUpPlugin::new();
        let host = TestHost::new(plugin.folder());
        let given = [GivenKey::new("admin", host.key("made-up-given-key"))];

        assert!(given_key(&given, "admin").is_some());
        assert!(given_key(&given, "other").is_none());
        assert!(given_key(&[], "admin").is_none());
    }

    /// Issue 26: a part learns only where a key came from, OTTY or an
    /// agent's place, so it knows which need replaces a refused key.
    #[test]
    fn a_key_tells_whether_the_user_gave_it_in_otty() {
        let plugin = MadeUpPlugin::new();
        let host = TestHost::new(plugin.folder());
        let found = host.key("made-up-found-key");
        let given = host.given_key("made-up-given-key");

        let (is_found_given, is_given, given_prefix) =
            host.as_provider("made-up-cloud", || {
                (
                    found.is_given_in_otty(),
                    given.is_given_in_otty(),
                    given.has_prefix("made-up-given"),
                )
            });

        assert_eq!(is_found_given, Ok(false));
        assert_eq!(is_given, Ok(true));
        assert_eq!(given_prefix, Ok(true));
    }
}
