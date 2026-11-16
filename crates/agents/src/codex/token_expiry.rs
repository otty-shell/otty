//! When a Codex login token expires.
//!
//! `auth.json` keeps no expiry time of its own: the access token is a JWT,
//! and its `exp` claim says when it expires. CodexBar reads it the same
//! way (`expirationFromJWT` in
//! <https://github.com/steipete/CodexBar/blob/main/Sources/CodexBarCore/Providers/Codex/CodexOAuth/CodexOAuthCredentials.swift>,
//! MIT licence). The token is only read, never checked for its signature.

use std::time::{Duration, SystemTime};

use serde_json::Value;

/// When `token` expires, from the `exp` claim (seconds since 1970) of its
/// middle part. `None` when the token is not a JWT or says nothing.
pub(super) fn token_expires_at(token: &str) -> Option<SystemTime> {
    let parts: Vec<&str> = token.split('.').collect();
    let [_, payload, _] = parts.as_slice() else {
        return None;
    };

    let bytes = decode_base64url(payload)?;
    let claims: Value = serde_json::from_slice(&bytes).ok()?;
    let seconds = claims.get("exp")?.as_u64()?;

    SystemTime::UNIX_EPOCH.checked_add(Duration::from_secs(seconds))
}

/// `text` in base64url, the alphabet of a JWT, with or without the `=`
/// padding. The letters of plain base64, `+` and `/`, are read too.
fn decode_base64url(text: &str) -> Option<Vec<u8>> {
    let mut bytes = Vec::new();
    let mut bits: u32 = 0;
    let mut bit_count = 0;

    for letter in text.trim_end_matches('=').bytes() {
        let value = match letter {
            b'A'..=b'Z' => letter - b'A',
            b'a'..=b'z' => letter - b'a' + 26,
            b'0'..=b'9' => letter - b'0' + 52,
            b'-' | b'+' => 62,
            b'_' | b'/' => 63,
            _ => return None,
        };
        bits = (bits << 6) | u32::from(value);
        bit_count += 6;

        if bit_count >= 8 {
            bit_count -= 8;
            bytes.push(u8::try_from((bits >> bit_count) & 0xff).ok()?);
            bits &= (1 << bit_count) - 1;
        }
    }

    Some(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{at, made_up_jwt};

    #[test]
    fn the_expiry_is_the_exp_claim_of_the_token() {
        let token = made_up_jwt(r#"{"exp": 1794823200, "sub": "made-up"}"#);

        assert_eq!(token_expires_at(&token), Some(at(1_794_823_200)));
    }

    #[test]
    fn a_part_with_or_without_padding_is_read() {
        let unpadded = "e30.eyJleHAiOjEwfQ.c2ln";
        let padded = "e30.eyJleHAiOjEwfQ==.c2ln";

        assert_eq!(token_expires_at(unpadded), Some(at(10)));
        assert_eq!(token_expires_at(padded), Some(at(10)));
    }

    #[test]
    fn a_token_that_says_nothing_has_no_expiry() {
        let no_exp = made_up_jwt(r#"{"sub": "made-up"}"#);
        let negative = made_up_jwt(r#"{"exp": -5}"#);

        for token in [
            "opaque-token",
            "a.b",
            "a.b.c.d",
            "e30.not*base64.c2ln",
            "e30.bm90IGpzb24.c2ln",
            no_exp.as_str(),
            negative.as_str(),
        ] {
            assert_eq!(token_expires_at(token), None, "{token}");
        }
    }

    #[test]
    fn a_far_expiry_is_kept() {
        let token = made_up_jwt(r#"{"exp": 4102444800}"#);

        assert_eq!(
            token_expires_at(&token),
            SystemTime::UNIX_EPOCH
                .checked_add(Duration::from_secs(4_102_444_800))
        );
    }
}
