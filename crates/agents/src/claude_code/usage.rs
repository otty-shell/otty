//! The first source of Claude Code limits: the usage request to Anthropic's
//! server with the profile's own login token.
//!
//! The request, its headers, the fields of the answer and the expiry of the
//! token follow CodexBar (MIT licence),
//! <https://github.com/steipete/CodexBar/blob/main/Sources/CodexBarCore/Providers/Claude/ClaudeOAuth/ClaudeOAuthUsageFetcher.swift>
//! and `ClaudeOAuthCredentialModels.swift` next to it. The address is not
//! documented by Anthropic, so the answer is read tolerantly.

use std::path::Path;
use std::time::{Duration, SystemTime};

use serde::Deserialize;
use serde_json::{Map, Value};

use super::plan::plan_name;
use crate::billing::{Billing, ExtraUsage, LimitWindow, UsageWindows};
use crate::error::SourceError;
use crate::https::{HttpsGet, HttpsRequest, usage_json};
use crate::json_file::read_json;
use crate::limits::{LimitSource, Limits};
use crate::profile::Profile;
use crate::source_id::SourceId;
use crate::timestamp::parse_rfc3339;

/// The address of the usage request.
const USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";
/// The beta header the usage address needs.
const OAUTH_BETA: &str = "oauth-2025-04-20";
/// The scope a token needs to read the usage.
const PROFILE_SCOPE: &str = "user:profile";
/// The windows of the answer, by their field, with the names shown.
const WINDOWS: [(&str, &str); 4] = [
    ("five_hour", "5 hours"),
    ("seven_day", "7 days"),
    ("seven_day_opus", "7 days · Opus"),
    ("seven_day_sonnet", "7 days · Sonnet"),
];
/// Cents in one unit of the currency: the answer counts extra usage in
/// cents.
const CENTS: f64 = 100.0;

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

    /// An expired token is "not logged in", and so is a refused request.
    /// A missing token file, or one with no token, is "not available": the
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
        return Err(SourceError::NotLoggedIn);
    }
    if !login.may_read_usage() {
        return Err(SourceError::NotAvailable(String::from(
            "the login token may not read the usage",
        )));
    }

    Ok(login)
}

/// The limits in the usage answer: the windows and the extra usage. A
/// window that is missing, `null` or of another shape is left out; an
/// answer with no window and no extra usage is unexpected, as in CodexBar.
fn limits_from(
    answer: &Value,
    plan: Option<String>,
) -> Result<Limits, SourceError> {
    let no_fields = Map::new();
    let fields = answer.as_object().unwrap_or(&no_fields);

    let windows: Vec<LimitWindow> = WINDOWS
        .iter()
        .filter_map(|(field, name)| usage_window(fields.get(*field)?, name))
        .collect();
    let extra_usage = fields.get("extra_usage").and_then(extra_usage);

    if windows.is_empty() && extra_usage.is_none() {
        return Err(SourceError::UnexpectedAnswer(String::from(
            "no usage window in the answer",
        )));
    }

    let usage = UsageWindows::new(windows, extra_usage);
    Ok(Limits::new(Billing::UsageWindows(usage), plan))
}

/// One window: `utilization` is the percent used, from 0 to 100, and
/// `resets_at` an RFC 3339 time.
fn usage_window(window: &Value, name: &str) -> Option<LimitWindow> {
    let percent_used = window.get("utilization")?.as_f64()?;
    let resets_at = window
        .get("resets_at")
        .and_then(Value::as_str)
        .and_then(parse_rfc3339);

    Some(LimitWindow::new(name, percent_used, resets_at))
}

/// The extra usage the account may buy, while it is turned on: how much
/// was spent this month and the monthly limit. The answer counts both in
/// cents; they are kept in whole units of the currency.
fn extra_usage(extra: &Value) -> Option<ExtraUsage> {
    if extra.get("is_enabled").and_then(Value::as_bool) != Some(true) {
        return None;
    }

    let amount = |field: &str| {
        let cents = extra.get(field)?.as_f64()?;
        Some(cents / CENTS)
    };
    let currency = extra
        .get("currency")
        .and_then(Value::as_str)
        .map(String::from);

    Some(ExtraUsage::new(
        amount("used_credits"),
        amount("monthly_limit"),
        currency,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
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

    /// The usage windows of `limits`; any other billing fails the test.
    fn usage(limits: &Limits) -> &UsageWindows {
        match limits.billing() {
            Billing::UsageWindows(usage) => usage,
            other => panic!("expected usage windows, got {other:?}"),
        }
    }

    fn names(limits: &Limits) -> Vec<&str> {
        usage(limits)
            .windows()
            .iter()
            .map(LimitWindow::name)
            .collect()
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

    #[test]
    fn a_missing_window_is_left_out() {
        let temp = TempDir::new();
        let dir = logged_in(&temp);
        let answer = r#"{
            "five_hour": {"utilization": 42.0, "resets_at": null},
            "seven_day_opus": null,
            "seven_day_sonnet": {"utilization": null, "resets_at": null}
        }"#;

        let limits = limits_in(&dir, answering(200, answer))
            .expect("the limits are read");

        assert_eq!(names(&limits), ["5 hours"]);
    }

    #[test]
    fn unknown_fields_and_a_window_of_another_shape_are_skipped() {
        let temp = TempDir::new();
        let dir = logged_in(&temp);
        let answer = r#"{
            "five_hour": {"utilization": 42.0, "resets_at": "soon", "kind": "session"},
            "seven_day": "full",
            "seven_day_oauth_apps": {"utilization": 80.0, "resets_at": null},
            "iguana_necktie": {"anything": [1, 2, 3]},
            "limits": [{"scope": {"model": {"display_name": "Fable"}}}]
        }"#;

        let limits = limits_in(&dir, answering(200, answer))
            .expect("the limits are read");

        assert_eq!(
            usage(&limits).windows(),
            [LimitWindow::new("5 hours", 42.0, None)]
        );
    }

    #[test]
    fn the_extra_usage_is_kept_in_money_next_to_the_windows() {
        let temp = TempDir::new();
        let dir = logged_in(&temp);
        let answer = r#"{
            "five_hour": {"utilization": 42.0, "resets_at": null},
            "extra_usage": {"is_enabled": true, "monthly_limit": 5000, "used_credits": 1250, "utilization": 25.0, "currency": "USD"}
        }"#;

        let limits = limits_in(&dir, answering(200, answer))
            .expect("the limits are read");

        assert_eq!(
            limits.billing(),
            &Billing::UsageWindows(UsageWindows::new(
                vec![LimitWindow::new("5 hours", 42.0, None)],
                Some(ExtraUsage::new(
                    Some(12.5),
                    Some(50.0),
                    Some(String::from("USD"))
                )),
            ))
        );
    }

    #[test]
    fn an_account_with_only_extra_usage_has_no_windows() {
        let temp = TempDir::new();
        let dir = logged_in(&temp);
        let answer = r#"{
            "five_hour": null,
            "seven_day": null,
            "extra_usage": {"is_enabled": true, "monthly_limit": null, "used_credits": 830, "utilization": null}
        }"#;

        let limits = limits_in(&dir, answering(200, answer))
            .expect("the limits are read");

        assert!(usage(&limits).windows().is_empty());
        assert_eq!(
            usage(&limits).extra_usage(),
            Some(&ExtraUsage::new(Some(8.3), None, None))
        );
    }

    #[test]
    fn extra_usage_that_is_turned_off_is_left_out() {
        let temp = TempDir::new();
        let dir = logged_in(&temp);
        let answer = r#"{
            "five_hour": {"utilization": 42.0, "resets_at": null},
            "extra_usage": {"is_enabled": false, "monthly_limit": 5000, "used_credits": 0}
        }"#;

        let limits = limits_in(&dir, answering(200, answer))
            .expect("the limits are read");

        assert_eq!(usage(&limits).extra_usage(), None);
    }

    #[test]
    fn an_answer_with_no_window_at_all_is_unexpected() {
        let temp = TempDir::new();
        let dir = logged_in(&temp);
        let answer =
            r#"{"five_hour": null, "extra_usage": {"is_enabled": false}}"#;

        let failure = limits_in(&dir, answering(200, answer));

        assert_eq!(
            failure,
            Err(SourceError::UnexpectedAnswer(String::from(
                "no usage window in the answer"
            )))
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

    #[test]
    fn an_expired_token_is_not_logged_in_and_sends_nothing() {
        let temp = TempDir::new();
        let dir = temp.path().join(".claude-work");
        write_file(
            &dir.join(".credentials.json"),
            &credentials(1_000, r#"["user:profile"]"#),
        );

        let failure = limits_in(&dir, never_sent());

        assert_eq!(failure, Err(SourceError::NotLoggedIn));
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
