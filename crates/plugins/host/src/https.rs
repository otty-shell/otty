//! The HTTPS requests plugins send through `http.send`, made by the host
//! with `ureq` over TLS from `rustls` (moved from `https.rs` of
//! `otty-agents`).
//!
//! The request is behind [`HttpsSend`], so the host is tested with a
//! mocked client and never reaches the network. [`UreqClient`] makes the
//! real one.

use std::fmt;
use std::time::Duration;

use crate::operations::v0_1::otty::host::http::Response;
use crate::operations::v0_1::otty::host::types::HostError;

/// The `User-Agent` of every request: OTTY and its version, nothing about
/// the user or the plugin.
pub(crate) const USER_AGENT: &str = concat!("otty/", env!("CARGO_PKG_VERSION"));
/// How long one request may take, from the connection to the last byte.
const TIMEOUT: Duration = Duration::from_secs(30);
/// The most bytes of an answer's body.
const MOST_ANSWER_BYTES: u64 = 8 * 1024 * 1024;

/// One request as the host sends it, keys filled in.
///
/// A plain parameter bag. A header may hold a key: the request is dropped
/// as soon as it was sent, and its `Debug` output shows no header value.
pub(crate) struct HttpsRequest {
    pub(crate) method: String,
    pub(crate) url: String,
    pub(crate) headers: Vec<(String, String)>,
    pub(crate) body: Option<Vec<u8>>,
}

impl fmt::Debug for HttpsRequest {
    /// The method, the URL and the header names; never a header value or
    /// the body.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let names: Vec<&str> =
            self.headers.iter().map(|(name, _)| name.as_str()).collect();

        formatter
            .debug_struct("HttpsRequest")
            .field("method", &self.method)
            .field("url", &self.url)
            .field("headers", &names)
            .field("body", &self.body.as_ref().map(Vec::len))
            .finish()
    }
}

/// Sends one HTTPS request.
///
/// It is a trait because it crosses into the network: the host operations
/// are tested with a mock and no connection.
#[cfg_attr(test, mockall::automock)]
pub(crate) trait HttpsSend: Send + Sync {
    /// The answer of the server, whatever its status: no redirect is
    /// followed, the body is at most 8 MiB, the whole request takes at
    /// most 30 s. The error never holds a header value.
    fn send(&self, request: &HttpsRequest) -> Result<Response, HostError>;
}

/// The real client, made with `ureq` over TLS from `rustls`.
///
/// It follows no redirect, so a key goes only to the address asked for,
/// and it sends no header but the ones of the request, OTTY's
/// `User-Agent` and what HTTP itself needs.
pub(crate) struct UreqClient {
    agent: ureq::Agent,
}

impl UreqClient {
    /// The client of every plugin.
    pub(crate) fn new() -> Self {
        let config = ureq::Agent::config_builder()
            .https_only(true)
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

impl HttpsSend for UreqClient {
    fn send(&self, request: &HttpsRequest) -> Result<Response, HostError> {
        let mut builder = ureq::http::Request::builder()
            .method(request.method.as_str())
            .uri(request.url.as_str());
        for (name, value) in &request.headers {
            builder = builder.header(name.as_str(), value.as_str());
        }

        // The error of a bad header would name the header, never its
        // value; still, only a plain text is given.
        let not_http = |_: ureq::http::Error| {
            HostError::Failed(format!(
                "the request to {} is not valid HTTP",
                request.url
            ))
        };
        let answer = match &request.body {
            Some(body) => self
                .agent
                .run(builder.body(body.as_slice()).map_err(not_http)?),
            None => self.agent.run(builder.body(()).map_err(not_http)?),
        };
        let mut response = answer.map_err(|error| failure(&error, request))?;

        let status = response.status().as_u16();
        let headers = response
            .headers()
            .iter()
            .map(|(name, value)| {
                let value = String::from_utf8_lossy(value.as_bytes());

                (name.as_str().to_string(), value.into_owned())
            })
            .collect();
        let body = response
            .body_mut()
            .with_config()
            .limit(MOST_ANSWER_BYTES)
            .read_to_vec()
            .map_err(|error| failure(&error, request))?;

        Ok(Response {
            status,
            headers,
            body,
        })
    }
}

/// A failure of `ureq` in the words of a host error. Its text says what
/// went wrong with the connection; `ureq` puts no header value in it.
fn failure(error: &ureq::Error, request: &HttpsRequest) -> HostError {
    let url = &request.url;

    match error {
        ureq::Error::Timeout(_) => {
            HostError::TimedOut(format!("{url} did not answer within 30 s"))
        },
        ureq::Error::BodyExceedsLimit(_) => {
            HostError::TooBig(format!("the answer of {url} is over 8 MiB"))
        },
        _ => HostError::Failed(format!("{url}: {error}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_user_agent_names_only_otty_and_its_version() {
        assert_eq!(USER_AGENT, format!("otty/{}", env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn the_debug_output_of_a_request_holds_no_header_value() {
        let request = HttpsRequest {
            method: "GET".to_string(),
            url: "https://api.example.com/v1/me".to_string(),
            headers: vec![(
                "authorization".to_string(),
                "Bearer ex-made-up-key".to_string(),
            )],
            body: Some(b"ex-made-up-body".to_vec()),
        };

        let debug = format!("{request:?}");

        assert!(debug.contains("authorization"), "{debug}");
        assert!(!debug.contains("ex-made-up-key"), "{debug}");
        assert!(!debug.contains("ex-made-up-body"), "{debug}");
    }
}
