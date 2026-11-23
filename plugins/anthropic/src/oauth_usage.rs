//! The limits of a Claude login: the usage request to Anthropic's server
//! with the login's token, for the provider part `anthropic`. The request
//! and its headers follow CodexBar (MIT licence),
//! <https://github.com/steipete/CodexBar/blob/main/Sources/CodexBarCore/Providers/Claude/ClaudeOAuth/ClaudeOAuthUsageFetcher.swift>.
//! The answer is read in `usage_answer.rs`.
//!
//! The part does not know whether the token may read the usage
//! (`user:profile`): it sends the request (issue 12 of spec
//! 2026-11-19-agent-plugins). The token is never renewed: a refused one
//! is the need that runs Claude Code in the folder the token was read in,
//! which renews the login or asks to log in again.

use otty_plugin_sdk::agents::ReadError;
use otty_plugin_sdk::http::Request;
use otty_plugin_sdk::{Command, Key};

use crate::needs;
use crate::requests::json_answer;
use crate::usage_answer::{UsageNumbers, usage_numbers};

/// The address of the usage request.
const USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";
/// The beta header the usage address needs.
const OAUTH_BETA: &str = "oauth-2025-04-20";

/// The windows and the extra usage of the Claude login whose token is
/// `key`. A refused token is the need that runs `claude`, Claude Code in
/// the folder of the token.
pub(crate) fn claude_usage(
    key: &Key,
    claude: &Command,
) -> Result<UsageNumbers, ReadError> {
    let request = Request::get(USAGE_URL)
        .header_with_key("authorization", "Bearer ", key)
        .header("accept", "application/json")
        .header("anthropic-beta", OAUTH_BETA);

    let answer = json_answer(&request, USAGE_URL, || {
        ReadError::NeedsUser(needs::login_expired(claude.clone()))
    })?;

    usage_numbers(&answer)
}

#[cfg(test)]
mod tests {
    use otty_plugin_sdk::NeedAction;
    use otty_plugin_sdk::test_host::TestHost;

    use super::*;
    use crate::testing::host;

    /// A made-up login token, never a real one.
    const TOKEN: &str = "made-up-access-token";

    /// Claude Code in a made-up config folder.
    fn claude() -> Command {
        Command {
            program: "claude".to_string(),
            args: Vec::new(),
            env: vec![(
                "CLAUDE_CONFIG_DIR".to_string(),
                "/home/dev/.claude-work".to_string(),
            )],
        }
    }

    fn usage_with(
        status: u16,
        body: &str,
    ) -> (TestHost, Result<UsageNumbers, ReadError>) {
        let host = host();
        host.answer("GET", USAGE_URL, status, body.as_bytes());
        let key = host.key(TOKEN);

        let usage =
            host.as_provider("anthropic", || claude_usage(&key, &claude()));
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
    /// refused by the server; the part renews nothing. Spec
    /// 2026-11-23-accounts-apart-from-profiles, Decision 9: the need runs
    /// Claude Code in the folder of the token.
    #[test]
    fn a_refused_request_needs_claude_in_the_folder_of_the_token() {
        for status in [401, 403] {
            let (_, usage) = usage_with(status, r#"{"error": "x"}"#);

            let Err(ReadError::NeedsUser(need)) = usage else {
                panic!("a need: {usage:?}");
            };
            assert_eq!(need.id, "login-expired", "{status}");
            assert_eq!(need.action, NeedAction::Run(claude()), "{status}");
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
