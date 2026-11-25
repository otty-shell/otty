//! The HTTPS request of the LiteLLM price table, and how its answer is
//! read.
//!
//! The request is behind [`HttpsGet`], so the sources are tested with a
//! mocked request and never reach the network. [`UreqClient`] makes the
//! real one.

use std::time::Duration;

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
