//! The limits of a Claude login: the usage request to Anthropic's server
//! with the login's token, for the provider part `anthropic`. The request
//! and its headers follow CodexBar (MIT licence),
//! <https://github.com/steipete/CodexBar/blob/main/Sources/CodexBarCore/Providers/Claude/ClaudeOAuth/ClaudeOAuthUsageFetcher.swift>.
//! The answer is read in `usage_answer.rs`.
//!
//! The part never sees `.credentials.json`, so it does not know when the
//! token expires or whether it may read the usage (`user:profile`): it
//! sends the request (issue 12 of spec 2026-11-19-agent-plugins). The
//! token is never renewed. A refusal is only a refused login (issue 20):
//! the part does not know which agent keeps it, so OTTY asks the agent
//! part that found it what the user must do.

use otty_plugin_sdk::http::Request;
use otty_plugin_sdk::{Key, ReadError};

use crate::requests::json_answer;
use crate::usage_answer::{UsageNumbers, usage_numbers};

/// The address of the usage request.
const USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";
/// The beta header the usage address needs.
const OAUTH_BETA: &str = "oauth-2025-04-20";

/// The windows and the extra usage of the Claude login whose token is
/// `key`.
pub(crate) fn claude_usage(key: &Key) -> Result<UsageNumbers, ReadError> {
    let request = Request::get(USAGE_URL)
        .header_with_key("authorization", "Bearer ", key)
        .header("accept", "application/json")
        .header("anthropic-beta", OAUTH_BETA);

    let answer = json_answer(&request, USAGE_URL, || ReadError::LoginRefused)?;

    usage_numbers(&answer)
}

#[cfg(test)]
mod tests {
    use otty_plugin_sdk::test_host::TestHost;

    use super::*;
    use crate::testing::host;

    /// A made-up login token, never a real one.
    const TOKEN: &str = "made-up-access-token";

    fn usage_with(
        status: u16,
        body: &str,
    ) -> (TestHost, Result<UsageNumbers, ReadError>) {
        let host = host();
        host.answer("GET", USAGE_URL, status, body.as_bytes());
        let key = host.key(TOKEN);

        let usage = host.as_provider("anthropic", || claude_usage(&key));
        (host, usage)
    }

    #[test]
    fn the_request_sends_the_token_and_the_beta_header() {
        let (host, usage) = usage_with(
            200,
            r#"{"five_hour": {"utilization": 42.0, "resets_at": null}}"#,
        );

        assert_eq!(usage.map(|usage| usage.windows.len()), Ok(1));
        let requests = host.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].url(), USAGE_URL);
        assert_eq!(
            requests[0].header("authorization"),
            Some(format!("Bearer {TOKEN}").as_str())
        );
        assert_eq!(requests[0].header("anthropic-beta"), Some(OAUTH_BETA));
    }

    /// Issue 12: an expired token, or one that may not read the usage, is
    /// refused by the server; the part renews nothing. Issue 20: it only
    /// says the login was refused and names no agent to run.
    #[test]
    fn a_refused_request_is_only_a_refused_login() {
        for status in [401, 403] {
            let (_, usage) = usage_with(status, r#"{"error": "x"}"#);

            assert_eq!(usage, Err(ReadError::LoginRefused), "{status}");
        }
    }

    #[test]
    fn an_unreadable_answer_is_unexpected() {
        for body in ["<html></html>", "[1, 2]"] {
            let (_, usage) = usage_with(200, body);

            assert!(matches!(usage, Err(ReadError::Unexpected(_))), "{body}");
        }
    }

    #[test]
    fn a_server_error_fails_without_what_the_answer_said() {
        let (_, usage) = usage_with(500, "made-up answer text");

        let Err(ReadError::Failed(text)) = usage else {
            panic!("a failure: {usage:?}");
        };
        assert!(!text.contains("made-up answer text"), "{text}");
    }
}
