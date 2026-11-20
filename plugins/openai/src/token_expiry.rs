//! When a Codex login token expires.
//!
//! `auth.json` keeps no expiry time of its own: the access token is a JWT,
//! and its `exp` claim says when it expires. CodexBar reads it the same
//! way (`expirationFromJWT` in
//! <https://github.com/steipete/CodexBar/blob/main/Sources/CodexBarCore/Providers/Codex/CodexOAuth/CodexOAuthCredentials.swift>,
//! MIT licence). The plugin never sees the token: OTTY gives it the claims
//! of the token's handle.

use serde_json::Value;

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
    use super::*;

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
