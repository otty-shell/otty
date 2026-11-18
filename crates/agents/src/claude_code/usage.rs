//! The first source of Claude Code limits: the usage request to Anthropic's
//! server with the profile's own login token.
//!
//! The request, its headers and the expiry of the token follow CodexBar
//! (MIT licence),
//! <https://github.com/steipete/CodexBar/blob/main/Sources/CodexBarCore/Providers/Claude/ClaudeOAuth/ClaudeOAuthUsageFetcher.swift>
//! and `ClaudeOAuthCredentialModels.swift` next to it. The answer is read
//! in `usage_answer.rs`.

use std::path::Path;
use std::time::SystemTime;

use super::agent::DESCRIPTOR;
use super::credentials::{OauthLogin, read_login};
use super::usage_answer::limits_from;
use crate::error::{SourceError, Unavailable};
use crate::https::{HttpsGet, HttpsRequest, json_answer};
use crate::limits::{LimitSource, Limits};
use crate::profile::Profile;
use crate::source_id::SourceId;

/// The address of the usage request.
const USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";
/// The beta header the usage address needs.
const OAUTH_BETA: &str = "oauth-2025-04-20";

/// The usage request with the token from `<config>/.credentials.json`.
pub(super) struct UsageRequest {
    client: Box<dyn HttpsGet>,
}

impl UsageRequest {
    /// The source that sends its request with `client`.
    pub(super) fn new(client: Box<dyn HttpsGet>) -> Self {
        Self { client }
    }
}

impl LimitSource for UsageRequest {
    fn id(&self) -> SourceId {
        SourceId::AnthropicUsageApi
    }

    /// An expired token is "login expired": Claude Code renews it when it
    /// runs, as the next source does, so it is no reason to log in yet. A
    /// refused request is "not logged in". OTTY never renews the token
    /// itself. A missing token file, or one with no token, is "not
    /// available": the
    /// login can be kept somewhere else, as on macOS. A token without the
    /// scope to read the usage is "not available" too. The plan comes from
    /// the credentials file.
    fn limits(&self, profile: &Profile) -> Result<Limits, SourceError> {
        let login = usable_login(profile.config_dir())?;
        let token = login.token().unwrap_or_default();

        let request = HttpsRequest {
            url: String::from(USAGE_URL),
            headers: vec![
                ("Authorization", format!("Bearer {token}")),
                ("Accept", String::from("application/json")),
                ("anthropic-beta", String::from(OAUTH_BETA)),
            ],
        };
        let answer = json_answer(self.client.get(&request))?;

        limits_from(&answer, login.plan())
    }
}

/// The login of the profile in `config_dir`, when its token may be sent.
fn usable_login(config_dir: &Path) -> Result<OauthLogin, SourceError> {
    let login = read_login(config_dir)
        .filter(|login| login.token().is_some())
        .ok_or(SourceError::NotAvailable(Unavailable::NoLoginToken))?;

    if login.is_expired(SystemTime::now()) {
        // No reason to log in: the next source runs Claude Code, which
        // renews the token.
        return Err(SourceError::LoginExpired(DESCRIPTOR.name()));
    }
    if !login.may_read_usage() {
        return Err(SourceError::NotAvailable(
            Unavailable::LoginMayNotReadUsage,
        ));
    }

    Ok(login)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::error::Unexpected;
    use crate::https::{HttpsAnswer, MockHttpsGet};
    use crate::limits::{Billing, LimitWindow, UsageWindows, WindowId};
    use crate::testing::{
        NOW, TempDir, answering, at, claude_profile, write_file,
    };

    /// 2100-01-01, in milliseconds.
    const FAR_FUTURE_MS: u64 = 4_102_444_800_000;
    const FULL_ANSWER: &str = r#"{
        "five_hour": {"utilization": 42.0, "resets_at": "2026-11-16T15:00:00.123456+00:00"},
        "seven_day": {"utilization": 17, "resets_at": "2026-11-20T10:00:00+00:00"},
        "seven_day_opus": {"utilization": 3.5, "resets_at": null},
        "seven_day_sonnet": {"utilization": 0.0, "resets_at": "2026-11-20T10:00:00Z"},
        "extra_usage": {"is_enabled": false, "monthly_limit": null, "used_credits": null, "utilization": null}
    }"#;

    fn credentials(expires_at: u64, scopes: &str) -> String {
        format!(
            r#"{{"claudeAiOauth": {{
                "accessToken": "made-up-access-token",
                "refreshToken": "made-up-refresh-token",
                "expiresAt": {expires_at},
                "scopes": {scopes},
                "subscriptionType": "max",
                "rateLimitTier": "default_claude_max_20x"
            }}}}"#
        )
    }

    /// A config folder with a token that expires in 2100.
    fn logged_in(temp: &TempDir) -> std::path::PathBuf {
        let dir = temp.path().join(".claude-work");
        write_file(
            &dir.join(".credentials.json"),
            &credentials(
                FAR_FUTURE_MS,
                r#"["user:inference", "user:profile"]"#,
            ),
        );
        dir
    }

    /// A request that must never be sent.
    fn never_sent() -> MockHttpsGet {
        let mut client = MockHttpsGet::new();
        client.expect_get().never();
        client
    }

    fn limits_in(
        dir: &Path,
        client: MockHttpsGet,
    ) -> Result<Limits, SourceError> {
        UsageRequest::new(Box::new(client)).limits(&claude_profile(dir))
    }

    #[test]
    fn a_full_answer_gives_every_window_with_its_reset_and_the_plan() {
        let temp = TempDir::new();
        let dir = logged_in(&temp);
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

        let limits = limits_in(&dir, client).expect("the limits are read");

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

    /// The login can be kept somewhere else than the token file, as on
    /// macOS, so a profile with no token there may still be logged in.
    #[test]
    fn a_missing_or_empty_token_file_is_not_available_and_sends_nothing() {
        let temp = TempDir::new();
        let dir = temp.path().join(".claude-work");
        std::fs::create_dir_all(&dir).expect("create config folder");
        let no_token =
            Err(SourceError::NotAvailable(Unavailable::NoLoginToken));

        let missing = limits_in(&dir, never_sent());
        write_file(&dir.join(".credentials.json"), "{}");
        let empty = limits_in(&dir, never_sent());
        write_file(
            &dir.join(".credentials.json"),
            r#"{"claudeAiOauth": {"accessToken": " "}}"#,
        );
        let blank = limits_in(&dir, never_sent());
        write_file(&dir.join(".credentials.json"), "{\"claudeAiOauth\": ");
        let broken = limits_in(&dir, never_sent());

        assert_eq!(missing, no_token);
        assert_eq!(empty, no_token);
        assert_eq!(blank, no_token);
        assert_eq!(broken, no_token);
    }

    /// Owner, issue 29: Claude Code renews an expired token when it runs,
    /// and the next source runs it, so the user is not told to log in.
    #[test]
    fn an_expired_token_is_a_login_expired_and_sends_nothing() {
        let temp = TempDir::new();
        let dir = temp.path().join(".claude-work");
        write_file(
            &dir.join(".credentials.json"),
            &credentials(1_000, r#"["user:profile"]"#),
        );

        let failure = limits_in(&dir, never_sent());

        assert_eq!(failure, Err(SourceError::LoginExpired("Claude Code")));
    }

    #[test]
    fn a_token_without_the_profile_scope_can_not_read_the_usage() {
        let temp = TempDir::new();
        let dir = temp.path().join(".claude-work");
        write_file(
            &dir.join(".credentials.json"),
            &credentials(FAR_FUTURE_MS, r#"["user:inference"]"#),
        );

        let failure = limits_in(&dir, never_sent());

        assert_eq!(
            failure,
            Err(SourceError::NotAvailable(Unavailable::LoginMayNotReadUsage))
        );
    }

    #[test]
    fn a_refused_request_is_not_logged_in() {
        let temp = TempDir::new();
        let dir = logged_in(&temp);

        let failure = limits_in(&dir, answering(401, r#"{"error": "x"}"#));

        assert_eq!(failure, Err(SourceError::NotLoggedIn));
    }

    #[test]
    fn no_connection_is_a_network_failure() {
        let temp = TempDir::new();
        let dir = logged_in(&temp);
        let mut client = MockHttpsGet::new();
        client.expect_get().times(1).return_once(|_| {
            Err(SourceError::Network(String::from("dns lookup failed")))
        });

        let failure = limits_in(&dir, client);

        assert_eq!(
            failure,
            Err(SourceError::Network(String::from("dns lookup failed")))
        );
    }

    #[test]
    fn an_unreadable_answer_is_unexpected() {
        let temp = TempDir::new();
        let dir = logged_in(&temp);

        let not_json = limits_in(&dir, answering(200, "<html></html>"));
        let not_an_object = limits_in(&dir, answering(200, "[1, 2]"));

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
