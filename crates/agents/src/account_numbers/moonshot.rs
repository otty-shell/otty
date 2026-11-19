//! The money left on a Moonshot AI account, `GET /v1/users/me/balance` with
//! its key, as Moonshot documents it:
//! <https://platform.moonshot.ai/docs/api/balance>. The international
//! platform tells the balance in US dollars, `data.available_balance`.

use super::{ask_with_key, number_in};
use crate::account_key::AccountKey;
use crate::balance::{Amount, Balance};
use crate::error::{SourceError, Unexpected};
use crate::https::HttpsGet;

/// The address of the balance on the international platform.
const BALANCE_URL: &str = "https://api.moonshot.ai/v1/users/me/balance";

/// The money left on the account of `key`.
pub(super) fn balance(
    client: &dyn HttpsGet,
    key: &AccountKey,
) -> Result<Balance, SourceError> {
    let answer = ask_with_key(client, BALANCE_URL, key, Vec::new())?;

    let left = answer
        .get("data")
        .and_then(|data| data.get("available_balance"))
        .and_then(number_in)
        .and_then(Amount::dollars)
        .ok_or(SourceError::UnexpectedAnswer(Unexpected::NotBalance))?;

    Ok(Balance::new(Some(left), None))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::admin_key::AdminKey;
    use crate::https::{HttpsAnswer, MockHttpsGet};
    use crate::testing::answering;

    fn key() -> AccountKey {
        AccountKey::new(AdminKey::made_up("sk-made-up-moonshot-key-0000"))
    }

    #[test]
    fn the_request_goes_to_the_balance_of_moonshot() {
        let mut client = MockHttpsGet::new();
        client
            .expect_get()
            .withf(|request| request.url == BALANCE_URL)
            .times(1)
            .return_once(|_| {
                Ok(HttpsAnswer {
                    status: 500,
                    body: Vec::new(),
                })
            });

        let read = balance(&client, &key());

        assert_eq!(
            read,
            Err(SourceError::UnexpectedAnswer(Unexpected::ServerStatus(500)))
        );
    }

    #[test]
    fn left_is_the_available_balance_in_dollars() {
        let client = answering(
            200,
            r#"{"code": 0, "data": {"available_balance": 49.58894,
            "voucher_balance": 46.58893, "cash_balance": 3.00001},
            "scode": "0x0", "status": true}"#,
        );

        let read = balance(&client, &key());

        assert_eq!(read, Ok(Balance::new(Amount::dollars(49.58894), None)));
    }

    #[test]
    fn an_answer_with_no_balance_is_unexpected() {
        let client = answering(200, r#"{"code": 0, "data": {}}"#);

        assert_eq!(
            balance(&client, &key()),
            Err(SourceError::UnexpectedAnswer(Unexpected::NotBalance))
        );
    }
}
