//! Key handles: what the host gives a plugin in place of a key (spec
//! Decision 5). A handle holds where the key sits; the key is read there
//! right before each use and dropped right after.

mod key_fields;
mod key_text;
mod place;

pub(crate) use crate::keys::key_fields::{key_value, take_key_fields};
pub(crate) use crate::keys::key_text::KeyText;
pub(crate) use crate::keys::place::{
    permission_place, place_text, read_json_file, read_key,
};
use crate::operations::v0_1::otty::host::keys::KeyPlace;

/// The host side of a key handle: where the key sits, never the key. The
/// host reads the key there right before each use.
#[derive(Debug, Clone)]
pub struct KeyHandle {
    place: KeyPlace,
}

impl KeyHandle {
    /// Where the key sits.
    pub fn place(&self) -> &KeyPlace {
        &self.place
    }

    /// The handle of the key at `place`.
    pub fn new(place: KeyPlace) -> Self {
        Self { place }
    }
}
