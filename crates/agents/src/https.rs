//! The HTTPS requests the limit sources make, and how their answers are
//! read.
//!
//! The request is behind [`HttpsGet`], so the sources are tested with a
//! mocked request and never reach the network. [`UreqClient`] makes the
//! real one.

use std::time::Duration;

use serde_json::Value;

use crate::error::SourceError;

/// The `User-Agent` of every request: OTTY and its version, nothing about
/// the user.
const USER_AGENT: &str = concat!("otty/", env!("CARGO_PKG_VERSION"));
/// How long one request may take, from the connection to the last byte.
const TIMEOUT: Duration = Duration::from_secs(30);

/// One `GET` request: the address and the headers.
///
/// A plain parameter bag. The headers may hold a login token: the request
/// is dropped as soon as it was sent, and it is never logged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HttpsRequest {
    pub(crate) url: String,
    pub(crate) headers: Vec<(&'static str, String)>,
}

/// What the server answered: the status code and the body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HttpsAnswer {
    pub(crate) status: u16,
    pub(crate) body: Vec<u8>,
}

/// Sends one `GET` request.
///
/// It is a trait because it crosses into the network: the sources are
/// tested with a mock and no connection.
#[cfg_attr(test, mockall::automock)]
pub(crate) trait HttpsGet: Send {
    /// The answer of the server, whatever its status. The error says why
    /// there was no answer at all, such as no connection.
    fn get(&self, request: &HttpsRequest) -> Result<HttpsAnswer, String>;
}

/// The real request, made with `ureq` over TLS from `rustls`.
///
/// It follows no redirect, so the login token goes only to the address
/// asked for, and it sends no header but the ones of the request, the
/// `User-Agent` and what HTTP itself needs.
pub(crate) struct UreqClient {
    agent: ureq::Agent,
}

impl UreqClient {
    /// A client for the requests of one read.
    pub(crate) fn new() -> Self {
        let config = ureq::Agent::config_builder()
            .timeout_global(Some(TIMEOUT))
            .http_status_as_error(false)
            .max_redirects(0)
            .user_agent(USER_AGENT)
            .build();

        Self {
            agent: config.into(),
        }
    }
}

impl HttpsGet for UreqClient {
    fn get(&self, request: &HttpsRequest) -> Result<HttpsAnswer, String> {
        let mut call = self.agent.get(&request.url);
        for (name, value) in &request.headers {
            call = call.header(*name, value);
        }

        let mut response = call.call().map_err(|error| error.to_string())?;
        let status = response.status().as_u16();
        let body = response
            .body_mut()
            .read_to_vec()
            .map_err(|error| error.to_string())?;

        Ok(HttpsAnswer { status, body })
    }
}

/// The JSON of the answer of a usage request.
///
/// No answer at all is a network failure. A refused request (401 or 403)
/// means the login is not accepted. Any other status, or a body that is not
/// JSON, is an unexpected answer.
pub(crate) fn usage_json(
    answer: Result<HttpsAnswer, String>,
) -> Result<Value, SourceError> {
    let answer = answer.map_err(SourceError::Network)?;

    match answer.status {
        200 => serde_json::from_slice(&answer.body).map_err(|_| {
            SourceError::UnexpectedAnswer(String::from(
                "the answer is not JSON",
            ))
        }),
        401 | 403 => Err(SourceError::NotLoggedIn),
        status => Err(SourceError::UnexpectedAnswer(format!(
            "the server answered {status}"
        ))),
    }
}

/// `text` written so it can be one part of an address, a path part or a
/// query value: letters, digits and `-._~` as they are, any other byte as
/// `%XX`. So a text from a file or an answer can not change the address
/// around it.
pub(crate) fn percent_encoded(text: &str) -> String {
    let mut encoded = String::with_capacity(text.len());

    for byte in text.bytes() {
        let is_plain = byte.is_ascii_alphanumeric()
            || matches!(byte, b'-' | b'.' | b'_' | b'~');
        if is_plain {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }

    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    fn answer(status: u16, body: &str) -> Result<HttpsAnswer, String> {
        Ok(HttpsAnswer {
            status,
            body: body.as_bytes().to_vec(),
        })
    }

    #[test]
    fn a_json_answer_is_read() {
        let json = usage_json(answer(200, r#"{"five_hour": null}"#));

        assert_eq!(json, Ok(serde_json::json!({"five_hour": null})));
    }

    #[test]
    fn a_refused_request_is_not_logged_in() {
        assert_eq!(usage_json(answer(401, "")), Err(SourceError::NotLoggedIn));
        assert_eq!(
            usage_json(answer(403, "{}")),
            Err(SourceError::NotLoggedIn)
        );
    }

    #[test]
    fn no_answer_is_a_network_failure() {
        let failure = usage_json(Err(String::from("connection refused")));

        assert_eq!(
            failure,
            Err(SourceError::Network(String::from("connection refused")))
        );
    }

    #[test]
    fn another_status_or_a_body_that_is_not_json_is_unexpected() {
        assert_eq!(
            usage_json(answer(500, "{}")),
            Err(SourceError::UnexpectedAnswer(String::from(
                "the server answered 500"
            )))
        );
        assert_eq!(
            usage_json(answer(200, "<html>sign in</html>")),
            Err(SourceError::UnexpectedAnswer(String::from(
                "the answer is not JSON"
            )))
        );
    }

    #[test]
    fn only_letters_digits_and_four_marks_are_kept_as_they_are() {
        assert_eq!(
            percent_encoded("00000000-0000-4000-8000-000000000000"),
            "00000000-0000-4000-8000-000000000000"
        );
        assert_eq!(percent_encoded("acct/a b?#%"), "acct%2Fa%20b%3F%23%25");
        assert_eq!(percent_encoded("ä.~_"), "%C3%A4.~_");
        assert_eq!(percent_encoded("page=2&x"), "page%3D2%26x");
    }

    #[test]
    fn the_user_agent_names_only_otty_and_its_version() {
        assert_eq!(USER_AGENT, format!("otty/{}", env!("CARGO_PKG_VERSION")));
    }
}
