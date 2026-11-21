//! Key handles: what the host gives a plugin in place of a key (spec
//! Decision 5). A handle holds where the key sits; the key is read there
//! right before each use and dropped right after. A key the user gave by
//! hand is read by OTTY for one read of a provider part instead.

mod key_fields;
mod key_text;
mod place;

use otty_plugin_manifest::Environment;

pub(crate) use crate::keys::key_fields::{key_value, take_key_fields};
pub use crate::keys::key_text::KeyText;
pub(crate) use crate::keys::place::{
    permission_place, place_text, read_json_file, read_key,
};
use crate::operations::v0_1::otty::host::keys::KeyPlace;
use crate::operations::v0_1::otty::host::types::HostError;
use crate::outside::Outside;

/// The host side of a key handle: where its key comes from, never shown
/// to the plugin. A plugin learns of the key only what the handle's
/// calls tell.
#[derive(Debug, Clone)]
pub struct KeyHandle {
    source: KeySource,
}

impl KeyHandle {
    /// Where the key comes from.
    pub fn source(&self) -> &KeySource {
        &self.source
    }

    /// The handle of the key at `place`.
    pub fn new(place: KeyPlace) -> Self {
        Self::of(KeySource::Place(place))
    }

    /// The handle of the key of `source`.
    pub fn of(source: KeySource) -> Self {
        Self { source }
    }
}

/// Where the key of a handle comes from.
///
/// ```
/// use otty_plugin_host::{KeySource, KeyText};
///
/// let given = KeyText::new("ex-made-up-key").expect("a filled key");
/// let source = KeySource::Given(given);
///
/// assert_eq!(format!("{source:?}"), "Given(KeyText(hidden))");
/// ```
#[derive(Debug, Clone)]
pub enum KeySource {
    /// A place an agent part named; the host reads the key there right
    /// before each use and drops it right after.
    Place(KeyPlace),
    /// A key the user gave in OTTY, the key of an account added by hand
    /// or one given for an `ask-key` slot (a file or a variable), which
    /// only OTTY can read: OTTY reads it for one read of a provider part,
    /// and it is dropped with that read. Its handle tells the part that
    /// the key was given in OTTY (`given-in-otty`).
    Given(KeyText),
}

/// The key of `source`, read now: from its place, with variables from
/// `env` and keyring entries from `outside`.
pub(crate) fn read_source(
    source: &KeySource,
    env: &Environment,
    outside: &Outside,
) -> Result<KeyText, HostError> {
    match source {
        KeySource::Place(place) => read_key(place, env, outside),
        KeySource::Given(key) => Ok(key.clone()),
    }
}
