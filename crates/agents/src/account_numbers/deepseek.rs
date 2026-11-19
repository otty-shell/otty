//! The money left on a DeepSeek account, `GET /user/balance` with its key,
//! as DeepSeek documents it:
//! <https://api-docs.deepseek.com/api/get-user-balance>. The answer lists
//! the balance in each currency of the account, `total_balance` as decimal
//! text.

use serde_json::Value;

use super::{ask_with_key, number_in};
use crate::account_key::AccountKey;
use crate::balance::{Amount, Balance};
use crate::error::{SourceError, Unexpected};
use crate::https::HttpsGet;

/// The address of the balance.
const BALANCE_URL: &str = "https://api.deepseek.com/user/balance";

/// The money left on the account of `key`: the total balance of the first
/// currency the answer lists.
pub(super) fn balance(
    client: &dyn HttpsGet,
    key: &AccountKey,
) -> Result<Balance, SourceError> {
    let answer = ask_with_key(client, BALANCE_URL, key, Vec::new())?;

    let left = answer
        .get("balance_infos")
        .and_then(Value::as_array)
        .and_then(|infos| infos.iter().find_map(amount_of))
        .ok_or(SourceError::UnexpectedAnswer(Unexpected::NotBalance))?;

    Ok(Balance::new(Some(left), None))
}

/// The total balance of one currency of the answer.
fn amount_of(info: &Value) -> Option<Amount> {
    let total = number_in(info.get("total_balance")?)?;
    let currency = info.get("currency")?.as_str()?;

    Amount::new(total, currency)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::admin_key::AdminKey;
    use crate::https::{HttpsAnswer, MockHttpsGet};
    use crate::testing::answering;

    /// A made-up key, never a real one.
    const MADE_UP_KEY: &str = "sk-made-up-deepseek-key-0000";

    fn key() -> AccountKey {
        AccountKey::new(AdminKey::made_up(MADE_UP_KEY))
    }

    #[test]
    fn the_request_sends_the_key_to_deepseek_only() {
        let mut client = MockHttpsGet::new();
        client
            .expect_get()
            .withf(|request| {
                request.url == BALANCE_URL
                    && request.headers.contains(&(
                        "Authorization",
                        format!("Bearer {MADE_UP_KEY}"),
                    ))
            })
            .times(1)
            .return_once(|_| {
                Ok(HttpsAnswer {
                    status: 200,
                    body: br#"{"is_available": true, "balance_infos": []}"#
                        .to_vec(),
                })
            });

        let _ = balance(&client, &key());
    }

    #[test]
    fn left_is_the_total_balance_in_its_currency() {
        let client = answering(
            200,
            r#"{"is_available": true, "balance_infos": [
                {"currency": "CNY", "total_balance": "110.00",
                 "granted_balance": "10.00", "topped_up_balance": "100.00"}
            ]}"#,
        );

        let read = balance(&client, &key());

        assert_eq!(read, Ok(Balance::new(Amount::new(110.0, "CNY"), None)));
    }

    #[test]
    fn an_answer_with_no_balance_is_unexpected() {
        for body in [r#"{"balance_infos": []}"#, r#"{"is_available": false}"#] {
            let client = answering(200, body);

            assert_eq!(
                balance(&client, &key()),
                Err(SourceError::UnexpectedAnswer(Unexpected::NotBalance)),
                "{body}"
            );
        }
    }

    #[test]
    fn a_refused_key_is_key_refused() {
        let client = answering(401, r#"{"error": "invalid key"}"#);

        assert_eq!(balance(&client, &key()), Err(SourceError::KeyRefused));
    }
}
