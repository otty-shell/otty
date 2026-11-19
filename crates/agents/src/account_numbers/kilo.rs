//! The money left on a Kilo account, `GET /api/profile/balance` with the
//! token of the Kilo login, as Kilo's own gateway asks for it
//! (`packages/kilo-gateway/src/api/profile.ts`, MIT licence): a team is
//! named in `x-kilocode-organizationid`, and the answer's `balance` is in
//! US dollars.

use super::{ask_with_key, number_in};
use crate::account_key::AccountKey;
use crate::balance::{Amount, Balance};
use crate::error::{SourceError, Unexpected};
use crate::https::HttpsGet;

/// The address of the balance.
const BALANCE_URL: &str = "https://api.kilo.ai/api/profile/balance";
/// The header that names the team of the login.
const ORGANIZATION_HEADER: &str = "x-kilocode-organizationid";

/// The money left on the Kilo account of `key`, of its team when the login
/// names one.
pub(super) fn balance(
    client: &dyn HttpsGet,
    key: &AccountKey,
) -> Result<Balance, SourceError> {
    let headers = key
        .organization_id()
        .map(|team| (ORGANIZATION_HEADER, team.to_string()))
        .into_iter()
        .collect();

    let answer = ask_with_key(client, BALANCE_URL, key, headers)?;

    let left = answer
        .get("balance")
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
        AccountKey::new(AdminKey::made_up("made-up-kilo-token-0000"))
    }

    fn answer() -> Result<HttpsAnswer, SourceError> {
        Ok(HttpsAnswer {
            status: 200,
            body: br#"{"balance": 7.5}"#.to_vec(),
        })
    }

    #[test]
    fn left_is_the_balance_in_dollars() {
        let client = answering(200, r#"{"balance": 7.5}"#);

        assert_eq!(
            balance(&client, &key()),
            Ok(Balance::new(Amount::dollars(7.5), None))
        );
    }

    #[test]
    fn a_team_login_names_its_team() {
        let mut client = MockHttpsGet::new();
        client
            .expect_get()
            .withf(|request| {
                request.url == BALANCE_URL
                    && request.headers.contains(&(
                        ORGANIZATION_HEADER,
                        String::from("made-up-team"),
                    ))
            })
            .times(1)
            .return_once(|_| answer());
        let team =
            key().with_organization_id(Some(String::from("made-up-team")));

        assert!(balance(&client, &team).is_ok());
    }

    #[test]
    fn a_personal_login_names_no_team() {
        let mut client = MockHttpsGet::new();
        client
            .expect_get()
            .withf(|request| {
                request
                    .headers
                    .iter()
                    .all(|(name, _)| *name != ORGANIZATION_HEADER)
            })
            .times(1)
            .return_once(|_| answer());

        assert!(balance(&client, &key()).is_ok());
    }

    #[test]
    fn an_answer_with_no_balance_is_unexpected() {
        let client = answering(200, "{}");

        assert_eq!(
            balance(&client, &key()),
            Err(SourceError::UnexpectedAnswer(Unexpected::NotBalance))
        );
    }
}
