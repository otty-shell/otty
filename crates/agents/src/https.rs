//! The HTTPS requests of the sources that ask a server, such as the usage
//! requests, the cost report and the LiteLLM price table, and how their
//! answers are read.
//!
//! The request is behind [`HttpsGet`], so the sources are tested with a
//! mocked request and never reach the network. [`UreqClient`] makes the
//! real one.

use std::time::Duration;

use serde_json::Value;

use crate::error::{SourceError, Unexpected};

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
    /// The answer of the server, whatever its status. No answer at all,
    /// such as no connection, is a [`SourceError::Network`] that says why.
    fn get(&self, request: &HttpsRequest) -> Result<HttpsAnswer, SourceError>;
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
    fn get(&self, request: &HttpsRequest) -> Result<HttpsAnswer, SourceError> {
        let mut call = self.agent.get(&request.url);
        for (name, value) in &request.headers {
            call = call.header(*name, value);
        }

        let network =
            |error: ureq::Error| SourceError::Network(error.to_string());
        let mut response = call.call().map_err(network)?;
        let status = response.status().as_u16();
        let body = response.body_mut().read_to_vec().map_err(network)?;

        Ok(HttpsAnswer { status, body })
    }
}

/// The body of the answer to a request that needs no login, such as the
/// LiteLLM price table.
///
/// No answer at all is a network failure. Any status but 200 is an
/// unexpected answer.
pub(crate) fn ok_body(
    answer: Result<HttpsAnswer, SourceError>,
) -> Result<Vec<u8>, SourceError> {
    let answer = answer?;

    match answer.status {
        200 => Ok(answer.body),
        status => Err(SourceError::UnexpectedAnswer(Unexpected::ServerStatus(
            status,
        ))),
    }
}

/// The JSON of the answer to a request sent with a login or a key, such as
/// a usage request or the cost report.
///
/// No answer at all is a network failure. A refused request (401 or 403)
/// means the login is not accepted. Any other status but 200, or a body
/// that is not JSON, is an unexpected answer.
pub(crate) fn json_answer(
    answer: Result<HttpsAnswer, SourceError>,
) -> Result<Value, SourceError> {
    let answer = answer?;
    if matches!(answer.status, 401 | 403) {
        return Err(SourceError::NotLoggedIn);
    }

    let body = ok_body(Ok(answer))?;
    serde_json::from_slice(&body)
        .map_err(|_| SourceError::UnexpectedAnswer(Unexpected::NotJson))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn answer(status: u16, body: &str) -> Result<HttpsAnswer, SourceError> {
        Ok(HttpsAnswer {
            status,
            body: body.as_bytes().to_vec(),
        })
    }

    #[test]
    fn a_json_answer_is_read() {
        let json = json_answer(answer(200, r#"{"five_hour": null}"#));

        assert_eq!(json, Ok(serde_json::json!({"five_hour": null})));
    }

    #[test]
    fn a_refused_request_is_not_logged_in() {
        assert_eq!(json_answer(answer(401, "")), Err(SourceError::NotLoggedIn));
        assert_eq!(
            json_answer(answer(403, "{}")),
            Err(SourceError::NotLoggedIn)
        );
    }

    #[test]
    fn no_answer_is_a_network_failure() {
        let failure = json_answer(Err(SourceError::Network(String::from(
            "connection refused",
        ))));

        assert_eq!(
            failure,
            Err(SourceError::Network(String::from("connection refused")))
        );
    }

    #[test]
    fn another_status_or_a_body_that_is_not_json_is_unexpected() {
        assert_eq!(
            json_answer(answer(500, "{}")),
            Err(SourceError::UnexpectedAnswer(Unexpected::ServerStatus(500)))
        );
        assert_eq!(
            json_answer(answer(200, "<html>sign in</html>")),
            Err(SourceError::UnexpectedAnswer(Unexpected::NotJson))
        );
    }

    #[test]
    fn a_body_is_given_only_with_status_200() {
        assert_eq!(ok_body(answer(200, "{")), Ok(b"{".to_vec()));
        assert_eq!(
            ok_body(answer(403, "")),
            Err(SourceError::UnexpectedAnswer(Unexpected::ServerStatus(403)))
        );
    }

    #[test]
    fn the_user_agent_names_only_otty_and_its_version() {
        assert_eq!(USER_AGENT, format!("otty/{}", env!("CARGO_PKG_VERSION")));
    }
}
