//! The first source of Claude Code limits: the usage request to Anthropic's
//! server with the profile's own login token.
//!
//! The request, its headers and the expiry of the token follow CodexBar
//! (MIT licence),
//! <https://github.com/steipete/CodexBar/blob/main/Sources/CodexBarCore/Providers/Claude/ClaudeOAuth/ClaudeOAuthUsageFetcher.swift>
//! and `ClaudeOAuthCredentialModels.swift` next to it. The answer is read
//! in `usage_answer.rs`.

use std::path::Path;
use std::time::{Duration, SystemTime};

use serde::Deserialize;

use super::plan::plan_name;
use super::usage_answer::limits_from;
use crate::error::SourceError;
use crate::https::{HttpsGet, HttpsRequest, usage_json};
use crate::json_file::read_json;
use crate::limits::{LimitSource, Limits};
use crate::profile::Profile;
use crate::source_id::SourceId;

/// The address of the usage request.
const USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";
/// The beta header the usage address needs.
const OAUTH_BETA: &str = "oauth-2025-04-20";
/// The scope a token needs to read the usage.
const PROFILE_SCOPE: &str = "user:profile";
/// Why an expired token is no reason to log in: the next source runs
/// Claude Code, which renews it.
const RENEWED_WHEN_RUN: &str = "Claude Code renews it when it runs";

/// The parts of `<config>/.credentials.json` the request needs.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Credentials {
    claude_ai_oauth: Option<OauthLogin>,
}

/// The login of a Claude account. The token is read only to be sent; it is
/// never written, refreshed or kept after the request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OauthLogin {
    access_token: Option<String>,
    /// When the token expires, in milliseconds since 1970.
    expires_at: Option<f64>,
    scopes: Option<Vec<String>>,
    subscription_type: Option<String>,
    rate_limit_tier: Option<String>,
}

impl OauthLogin {
    /// The token, when there is one.
    fn token(&self) -> Option<&str> {
        let token = self.access_token.as_deref()?.trim();

        (!token.is_empty()).then_some(token)
    }

    /// Whether the token expired before `now`. A token with no expiry
    /// time is tried: the server says whether it is still good.
    fn is_expired(&self, now: SystemTime) -> bool {
        let expires_at = self
            .expires_at
            .and_then(|millis| {
                Duration::try_from_secs_f64(millis / 1000.0).ok()
            })
            .and_then(|since_epoch| {
                SystemTime::UNIX_EPOCH.checked_add(since_epoch)
            });

        expires_at.is_some_and(|expires_at| expires_at <= now)
    }

    /// Whether the token may read the usage. A token made only to run
    /// the agent, such as one from `claude setup-token`, has no
    /// `user:profile` scope, and the server refuses it.
    fn may_read_usage(&self) -> bool {
        match &self.scopes {
            Some(scopes) => scopes.iter().any(|scope| scope == PROFILE_SCOPE),
            None => true,
        }
    }

    /// The plan as Claude Code names it, such as "Max 20x".
    fn plan(&self) -> Option<String> {
        plan_name(
            self.subscription_type.as_deref(),
            self.rate_limit_tier.as_deref(),
        )
    }
}

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
        let answer = usage_json(self.client.get(&request))?;

        limits_from(&answer, login.plan())
    }
}

/// The login of the profile in `config_dir`, when its token may be sent.
fn usable_login(config_dir: &Path) -> Result<OauthLogin, SourceError> {
    let login = read_json::<Credentials>(&config_dir.join(".credentials.json"))
        .and_then(|credentials| credentials.claude_ai_oauth)
        .filter(|login| login.token().is_some())
        .ok_or_else(|| {
            SourceError::NotAvailable(String::from(
                "no login token in .credentials.json",
            ))
        })?;

    if login.is_expired(SystemTime::now()) {
        return Err(SourceError::LoginExpired(String::from(RENEWED_WHEN_RUN)));
    }
    if !login.may_read_usage() {
        return Err(SourceError::NotAvailable(String::from(
            "the login token may not read the usage",
        )));
    }

    Ok(login)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::billing::{Billing, LimitWindow, UsageWindows};
    use crate::https::{HttpsAnswer, MockHttpsGet};
    use crate::testing::{TempDir, at, claude_profile, write_file};

    /// 2100-01-01, in milliseconds.
    const FAR_FUTURE_MS: u64 = 4_102_444_800_000;
    /// 2026-11-16T10:00:00Z.
    const NOW: u64 = 1_794_823_200;

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

    /// A request that answers `status` with `body` once.
    fn answering(status: u16, body: &str) -> MockHttpsGet {
        let body = body.as_bytes().to_vec();
        let mut client = MockHttpsGet::new();
        client
            .expect_get()
            .times(1)
            .return_once(move |_| Ok(HttpsAnswer { status, body }));
        client
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
                        LimitWindow::new("5 hours", 42.0, Some(five_hours)),
                        LimitWindow::new("7 days", 17.0, Some(seven_days)),
                        LimitWindow::new("7 days · Opus", 3.5, None),
                        LimitWindow::new(
                            "7 days · Sonnet",
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
        let no_token = Err(SourceError::NotAvailable(String::from(
            "no login token in .credentials.json",
        )));

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

        assert_eq!(
            failure,
            Err(SourceError::LoginExpired(String::from(
                "Claude Code renews it when it runs"
            )))
        );
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
            Err(SourceError::NotAvailable(String::from(
                "the login token may not read the usage"
            )))
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
        client
            .expect_get()
            .times(1)
            .return_once(|_| Err(String::from("dns lookup failed")));

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
            Err(SourceError::UnexpectedAnswer(String::from(
                "the answer is not JSON"
            )))
        );
        assert_eq!(
            not_an_object,
            Err(SourceError::UnexpectedAnswer(String::from(
                "no usage window in the answer"
            )))
        );
    }
}
