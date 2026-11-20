//! Key handles: what the host gives a plugin in place of a key.

use crate::operations::v0_1::otty::host::keys::KeyPlace;

/// The host side of a key handle: where the key sits, never the key. The
/// host reads the key there right before each use (issue 05).
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
