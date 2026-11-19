//! The claims of a Codex login token, a JWT. The token is only read, never
//! checked for its signature.

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD_INDIFFERENT;
use serde::de::DeserializeOwned;

/// The claims of the middle part of `token`; `None` when the token is not a
/// JWT or its claims are not a `T`.
///
/// The middle part is base64url, the alphabet of a JWT, read with or
/// without the `=` padding.
pub(super) fn jwt_claims<T: DeserializeOwned>(token: &str) -> Option<T> {
    let parts: Vec<&str> = token.split('.').collect();
    let [_, payload, _] = parts.as_slice() else {
        return None;
    };

    let bytes = URL_SAFE_NO_PAD_INDIFFERENT.decode(payload).ok()?;

    serde_json::from_slice(&bytes).ok()
}
