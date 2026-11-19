//! The limit of an OpenRouter key, `GET /api/v1/key` with the key itself,
//! as OpenRouter documents it:
//! <https://openrouter.ai/docs/api/reference/limits.md>. `data.usage` is
//! what the key used and `data.limit` its limit, `null` for none, both in
//! US dollars.

use serde_json::Value;

use super::{ask_with_key, number_in};
use crate::account_key::AccountKey;
use crate::balance::{Amount, Balance, KeyLimit};
use crate::error::{SourceError, Unexpected};
use crate::https::HttpsGet;

/// The address of the key's own numbers.
const KEY_URL: &str = "https://openrouter.ai/api/v1/key";

/// The limit of `key` and what of it was used.
pub(super) fn balance(
    client: &dyn HttpsGet,
    key: &AccountKey,
) -> Result<Balance, SourceError> {
    let answer = ask_with_key(client, KEY_URL, key, Vec::new())?;
    let not_balance = SourceError::UnexpectedAnswer(Unexpected::NotBalance);

    let data = answer.get("data").ok_or(not_balance.clone())?;
    let used = data
        .get("usage")
        .and_then(number_in)
        .and_then(Amount::dollars)
        .ok_or(not_balance)?;
    let limit = data
        .get("limit")
        .filter(|limit| !Value::is_null(limit))
        .and_then(number_in)
        .and_then(Amount::dollars);

    Ok(Balance::new(None, Some(KeyLimit::new(used, limit))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::admin_key::AdminKey;
    use crate::https::{HttpsAnswer, MockHttpsGet};
    use crate::testing::answering;

    fn key() -> AccountKey {
        AccountKey::new(AdminKey::made_up("sk-or-v1-made-up-key-0000"))
    }

    #[test]
    fn the_request_asks_openrouter_for_the_key() {
        let mut client = MockHttpsGet::new();
        client
            .expect_get()
            .withf(|request| request.url == KEY_URL)
            .times(1)
            .return_once(|_| {
                Ok(HttpsAnswer {
                    status: 403,
                    body: Vec::new(),
                })
            });

        assert_eq!(balance(&client, &key()), Err(SourceError::KeyRefused));
    }

    #[test]
    fn the_key_limit_is_the_usage_of_the_limit() {
        let client = answering(
            200,
            r#"{"data": {"label": "sk-or-v1-abc", "usage": 12.3,
            "limit": 50, "limit_remaining": 37.7, "is_free_tier": false}}"#,
        );

        let read = balance(&client, &key());

        assert_eq!(
            read,
            Ok(Balance::new(
                None,
                Some(KeyLimit::new(
                    Amount::dollars(12.3).expect("a number"),
                    Amount::dollars(50.0)
                ))
            ))
        );
    }

    #[test]
    fn a_key_with_no_limit_has_only_its_usage() {
        let client = answering(
            200,
            r#"{"data": {"usage": 0.5, "limit": null, "limit_remaining": null}}"#,
        );

        let read = balance(&client, &key()).expect("a balance");
        let limit = read.key_limit().expect("a key limit");

        assert_eq!(limit.used().value(), 0.5);
        assert_eq!(limit.limit(), None);
    }

    #[test]
    fn an_answer_with_no_usage_is_unexpected() {
        let client = answering(200, r#"{"data": {"limit": 50}}"#);

        assert_eq!(
            balance(&client, &key()),
            Err(SourceError::UnexpectedAnswer(Unexpected::NotBalance))
        );
    }
}
