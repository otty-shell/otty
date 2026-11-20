//! The limits of a Claude login: the usage request to Anthropic's server
//! with the login's token.
//!
//! It is kept in OTTY until the provider part `anthropic` of the Anthropic
//! plugin reads it with a key handle (spec 2026-11-19-agent-plugins, issue
//! 15); the Claude Code reader that sent it is gone, and its plugin reads
//! the limits from the `/usage` panel instead. The request and its headers
//! follow CodexBar (MIT licence),
//! <https://github.com/steipete/CodexBar/blob/main/Sources/CodexBarCore/Providers/Claude/ClaudeOAuth/ClaudeOAuthUsageFetcher.swift>.
//! The answer is read in `claude_usage_answer.rs`.

use super::claude_usage_answer::limits_from;
use crate::error::SourceError;
use crate::https::{HttpsGet, HttpsRequest, json_answer};
use crate::limits::Limits;

/// The address of the usage request.
const USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";
/// The beta header the usage address needs.
const OAUTH_BETA: &str = "oauth-2025-04-20";

/// The limits of the Claude login whose token is `token`, on the plan
/// `plan`, asked with `client`. A refused request is "not logged in": the
/// token expired or may not read the usage. OTTY never renews the token.
pub(super) fn claude_limits(
    client: &dyn HttpsGet,
    token: &str,
    plan: Option<String>,
) -> Result<Limits, SourceError> {
    let request = HttpsRequest {
        url: String::from(USAGE_URL),
        headers: vec![
            ("Authorization", format!("Bearer {token}")),
            ("Accept", String::from("application/json")),
            ("anthropic-beta", String::from(OAUTH_BETA)),
        ],
    };
    let answer = json_answer(client.get(&request))?;

    limits_from(&answer, plan)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::error::Unexpected;
    use crate::https::{HttpsAnswer, MockHttpsGet};
    use crate::limits::{Billing, LimitWindow, UsageWindows, WindowId};
    use crate::testing::{NOW, answering, at};

    const FULL_ANSWER: &str = r#"{
        "five_hour": {"utilization": 42.0, "resets_at": "2026-11-16T15:00:00.123456+00:00"},
        "seven_day": {"utilization": 17, "resets_at": "2026-11-20T10:00:00+00:00"},
        "seven_day_opus": {"utilization": 3.5, "resets_at": null},
        "seven_day_sonnet": {"utilization": 0.0, "resets_at": "2026-11-20T10:00:00Z"},
        "extra_usage": {"is_enabled": false, "monthly_limit": null, "used_credits": null, "utilization": null}
    }"#;

    fn limits_with(client: MockHttpsGet) -> Result<Limits, SourceError> {
        claude_limits(
            &client,
            "made-up-access-token",
            Some(String::from("Max 20x")),
        )
    }

    #[test]
    fn a_full_answer_gives_every_window_with_its_reset_and_the_plan() {
        let mut client = MockHttpsGet::new();
        client
            .expect_get()
            .withf(|request| {
                request.url == "https://api.anthropic.com/api/oauth/usage"
                    && request.headers
                        == [
                            (
                                "Authorization",
                                String::from("Bearer made-up-access-token"),
                            ),
                            ("Accept", String::from("application/json")),
                            (
                                "anthropic-beta",
                                String::from("oauth-2025-04-20"),
                            ),
                        ]
            })
            .times(1)
            .return_once(|_| {
                Ok(HttpsAnswer {
                    status: 200,
                    body: FULL_ANSWER.as_bytes().to_vec(),
                })
            });

        let limits = limits_with(client).expect("the limits are read");

        let five_hours = at(NOW + 5 * 3600) + Duration::from_micros(123_456);
        let seven_days = at(NOW + 4 * 24 * 3600);
        assert_eq!(
            limits,
            Limits::new(
                Billing::UsageWindows(UsageWindows::new(
                    vec![
                        LimitWindow::new(
                            WindowId::hours(5),
                            42.0,
                            Some(five_hours)
                        ),
                        LimitWindow::new(
                            WindowId::days(7),
                            17.0,
                            Some(seven_days)
                        ),
                        LimitWindow::new(
                            WindowId::days(7).for_model("Opus"),
                            3.5,
                            None
                        ),
                        LimitWindow::new(
                            WindowId::days(7).for_model("Sonnet"),
                            0.0,
                            Some(seven_days)
                        ),
                    ],
                    None,
                )),
                Some(String::from("Max 20x")),
            )
        );
    }

    #[test]
    fn a_refused_request_is_not_logged_in() {
        let failure = limits_with(answering(401, r#"{"error": "x"}"#));

        assert_eq!(failure, Err(SourceError::NotLoggedIn));
    }

    #[test]
    fn no_connection_is_a_network_failure() {
        let mut client = MockHttpsGet::new();
        client.expect_get().times(1).return_once(|_| {
            Err(SourceError::Network(String::from("dns lookup failed")))
        });

        let failure = limits_with(client);

        assert_eq!(
            failure,
            Err(SourceError::Network(String::from("dns lookup failed")))
        );
    }

    #[test]
    fn an_unreadable_answer_is_unexpected() {
        let not_json = limits_with(answering(200, "<html></html>"));
        let not_an_object = limits_with(answering(200, "[1, 2]"));

        assert_eq!(
            not_json,
            Err(SourceError::UnexpectedAnswer(Unexpected::NotJson))
        );
        assert_eq!(
            not_an_object,
            Err(SourceError::UnexpectedAnswer(Unexpected::NoLimits))
        );
    }
}
