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
