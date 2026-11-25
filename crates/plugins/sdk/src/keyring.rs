//! The system keyring entries the called part declares in `keyring`, once
//! the user allowed OTTY to read the keyring.

use crate::{HostError, JsonWithKeys};

/// The entry of `service` and `user` as JSON, its key fields replaced by
/// handles; `None` when there is no such entry.
pub fn read(
    service: &str,
    user: &str,
) -> Result<Option<JsonWithKeys>, HostError> {
    #[cfg(target_arch = "wasm32")]
    {
        crate::bindings::host::otty::host::keyring::read(service, user)
            .map(|entry| entry.map(JsonWithKeys::from_wasm))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        crate::test_host::read_keyring(service, user)
    }
}
