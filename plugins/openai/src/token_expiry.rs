//! When a Codex login token expires.
//!
//! `auth.json` keeps no expiry time of its own: the access token is a JWT,
//! and its `exp` claim says when it expires. CodexBar reads it the same
//! way (`expirationFromJWT` in
//! <https://github.com/steipete/CodexBar/blob/main/Sources/CodexBarCore/Providers/Codex/CodexOAuth/CodexOAuthCredentials.swift>,
//! MIT licence). The plugin never sees the token: OTTY gives it the claims
//! of the token's handle.

use otty_plugin_sdk::{Key, clock};
use serde_json::Value;

/// Whether `token` expired, as its claims say. A token that says nothing,
/// or that is not at its place, is not taken as expired.
pub(crate) fn has_expired(token: &Key) -> bool {
    let claims = token.jwt_claims().ok().flatten();
    let expires_at_ms = claims.as_deref().and_then(expires_at_ms);

    expires_at_ms.is_some_and(|expires_at_ms| expires_at_ms <= clock::now_ms())
}

/// When the token whose claims are `claims`, JSON text, expires, from its
/// `exp` claim (seconds since 1970), in milliseconds since the Unix
/// epoch. `None` when the claims say nothing.
pub(crate) fn expires_at_ms(claims: &str) -> Option<i64> {
    let claims: Value = serde_json::from_str(claims).ok()?;
    let seconds = i64::try_from(claims.get("exp")?.as_u64()?).ok()?;

    seconds.checked_mul(1000)
}

#[cfg(test)]
mod tests {
    use otty_plugin_sdk::test_host::TestHost;
    use otty_plugin_sdk::{JsonField, KeyPlace};

    use super::*;
    use crate::auth_file::ACCESS_TOKEN_FIELD;
    use crate::testing::{TEN, as_provider, host, path};

    /// A handle to the access token of the login of the config folder
    /// `name`, read at each use.
    fn token_of(host: &TestHost, name: &str) -> Key {
        host.key_at(&KeyPlace::JsonField(JsonField {
            file: path(host, &format!("{name}/auth.json")),
            pointer: ACCESS_TOKEN_FIELD.to_string(),
        }))
    }

    #[test]
    fn a_token_whose_claims_say_so_expired() {
        let host = host().with_now_ms(TEN);
        let expired = token_of(&host, ".codex-expired");

        assert!(as_provider(&host, || has_expired(&expired)));
    }

    #[test]
    fn a_token_that_does_not_tell_its_expiry_is_used() {
        let host = host().with_now_ms(TEN);
        let opaque = token_of(&host, ".codex");
        let fresh = token_of(&host, ".codex-plus");
        let missing = token_of(&host, ".codex-empty");

        let expired = as_provider(&host, || {
            [
                has_expired(&opaque),
                has_expired(&fresh),
                has_expired(&missing),
            ]
        });

        assert_eq!(expired, [false, false, false]);
    }

    #[test]
    fn the_expiry_is_the_exp_claim_of_the_token() {
        assert_eq!(
            expires_at_ms(r#"{"exp": 1794823200, "sub": "made-up"}"#),
            Some(1_794_823_200_000)
        );
    }

    #[test]
    fn claims_that_say_nothing_have_no_expiry() {
        for claims in [
            r#"{"sub": "made-up"}"#,
            r#"{"exp": -5}"#,
            r#"{"exp": "1794823200"}"#,
            "not json",
            "[]",
        ] {
            assert_eq!(expires_at_ms(claims), None, "{claims}");
        }
    }

    #[test]
    fn a_far_expiry_is_kept() {
        assert_eq!(
            expires_at_ms(r#"{"exp": 4102444800}"#),
            Some(4_102_444_800_000)
        );
    }
}
