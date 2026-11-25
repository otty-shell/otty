//! The requests to GitHub: the releases list and the release files, with
//! no login.
//!
//! The request is behind [`HttpClient`], so installs and updates are
//! tested with a mocked GitHub and never reach the network. [`UreqClient`]
//! makes the real one.

use std::time::Duration;

/// The `User-Agent` of every request: OTTY and its version, nothing about
/// the user.
pub const USER_AGENT: &str = concat!("OTTY/", env!("CARGO_PKG_VERSION"));
/// How long one request may take, from the connection to the last byte.
const TIMEOUT: Duration = Duration::from_secs(60);
/// How many redirects a download follows: GitHub sends release files from
/// another host. No request carries a login, so following them is safe.
const MAX_REDIRECTS: u32 = 5;

/// Why a request gave no body.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HttpError {
    /// GitHub refuses more requests without a login for now.
    #[error("GitHub refuses more requests from this computer for now")]
    RateLimited,
    /// Nothing at the address: a private or a missing repository or file.
    #[error("nothing is there")]
    NotFound,
    /// The body is longer than the limit of the request.
    #[error("it is larger than {0} bytes")]
    TooLarge(u64),
    /// Another status than 200.
    #[error("the server answered {0}")]
    Status(u16),
    /// No answer at all, such as no connection.
    #[error("no connection: {0}")]
    Network(String),
}

/// Sends one `GET` request with no login.
///
/// It is a trait because it crosses into the network: installs and
/// updates are tested with a mock and no connection.
#[cfg_attr(test, mockall::automock)]
pub trait HttpClient: Send {
    /// The body at `url`, at most `limit` bytes.
    fn get(&self, url: &str, limit: u64) -> Result<Vec<u8>, HttpError>;
}

/// The real request, made with `ureq` over TLS from `rustls`.
pub struct UreqClient {
    agent: ureq::Agent,
}

impl UreqClient {
    /// A client for GitHub's API and release files.
    pub fn new() -> Self {
        let config = ureq::Agent::config_builder()
            .timeout_global(Some(TIMEOUT))
            .http_status_as_error(false)
            .max_redirects(MAX_REDIRECTS)
            .user_agent(USER_AGENT)
            .build();

        Self {
            agent: config.into(),
        }
    }
}

impl Default for UreqClient {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpClient for UreqClient {
    fn get(&self, url: &str, limit: u64) -> Result<Vec<u8>, HttpError> {
        let mut response = self
            .agent
            .get(url)
            .header("Accept", "application/vnd.github+json, */*")
            .call()
            .map_err(|error| HttpError::Network(error.to_string()))?;

        let status = response.status().as_u16();
        let rate_limited = response
            .headers()
            .get("x-ratelimit-remaining")
            .is_some_and(|remaining| remaining == "0");
        match status {
            200 => {},
            404 => return Err(HttpError::NotFound),
            429 => return Err(HttpError::RateLimited),
            403 if rate_limited => return Err(HttpError::RateLimited),
            _ => return Err(HttpError::Status(status)),
        }

        response
            .body_mut()
            .with_config()
            .limit(limit)
            .read_to_vec()
            .map_err(|error| match error {
                ureq::Error::BodyExceedsLimit(limit) => {
                    HttpError::TooLarge(limit)
                },
                _ => HttpError::Network(error.to_string()),
            })
    }
}
