//! The first source of Codex limits: the usage request to the ChatGPT
//! server with the profile's own login token.
//!
//! The request, its headers and the fields of the answer follow CodexBar
//! (MIT licence),
//! <https://github.com/steipete/CodexBar/blob/main/Sources/CodexBarCore/Providers/Codex/CodexOAuth/CodexOAuthUsageFetcher.swift>.
//! The address is not documented by OpenAI, so the answer is read
//! tolerantly.

use std::path::Path;
use std::time::{Duration, SystemTime};

use serde::Deserialize;
use serde::de::IgnoredAny;
use serde_json::Value;

use super::base_url::usage_url;
use super::token_expiry::token_expires_at;
use crate::billing::{Billing, Credits, LimitWindow, UsageWindows};
use crate::error::SourceError;
use crate::https::{HttpsGet, HttpsRequest, usage_json};
use crate::json_file::read_json;
use crate::limits::{LimitSource, Limits};
use crate::profile::Profile;

/// The parts of `<config>/auth.json` the request needs.
#[derive(Debug, Deserialize)]
struct Auth {
    /// Only checked for being there, and never kept.
    #[serde(rename = "OPENAI_API_KEY")]
    api_key: Option<IgnoredAny>,
    tokens: Option<Tokens>,
}

/// The tokens of a ChatGPT login. The access token is read only to be
/// sent; it is never written, refreshed or kept after the request.
#[derive(Debug, Deserialize)]
struct Tokens {
    access_token: Option<String>,
    /// The ChatGPT workspace of the login, sent as `ChatGPT-Account-Id`
    /// so a team login gets the limits of its workspace.
    account_id: Option<String>,
}

impl Tokens {
    /// The access token, when there is one.
    fn access_token(&self) -> Option<&str> {
        filled(self.access_token.as_deref())
    }

    /// The ChatGPT workspace, when the file names one.
    fn account_id(&self) -> Option<&str> {
        filled(self.account_id.as_deref())
    }
}

/// The usage request with the token from `<config>/auth.json`.
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
    fn name(&self) -> &'static str {
        "usage request"
    }

    /// A missing or expired token is "not logged in", and so is a refused
    /// request. A login with an API key has no usage limits to read: it is
    /// "not available".
    fn limits(&self, profile: &Profile) -> Result<Limits, SourceError> {
        let config_dir = profile.config_dir();
        let tokens = usable_tokens(&config_dir.join("auth.json"))?;
        let token = tokens.access_token().unwrap_or_default();

        let mut headers = vec![
            ("Authorization", format!("Bearer {token}")),
            ("Accept", String::from("application/json")),
        ];
        if let Some(account_id) = tokens.account_id() {
            headers.push(("ChatGPT-Account-Id", account_id.to_string()));
        }
        let request = HttpsRequest {
            url: usage_url(config_dir),
            headers,
        };
        let answer = usage_json(self.client.get(&request))?;

        limits_from(&answer)
    }
}

/// The tokens in `auth_file`, when the access token may be sent.
fn usable_tokens(auth_file: &Path) -> Result<Tokens, SourceError> {
    let auth = read_json::<Auth>(auth_file).ok_or(SourceError::NotLoggedIn)?;

    let tokens = auth.tokens.filter(|tokens| tokens.access_token().is_some());
    let Some(tokens) = tokens else {
        if auth.api_key.is_some() {
            return Err(SourceError::NotAvailable(String::from(
                "a login with an API key has no usage limits",
            )));
        }
        return Err(SourceError::NotLoggedIn);
    };

    let expires_at = tokens.access_token().and_then(token_expires_at);
    if expires_at.is_some_and(|expires_at| expires_at <= SystemTime::now()) {
        return Err(SourceError::NotLoggedIn);
    }

    Ok(tokens)
}

/// The limits in the usage answer and the plan: the two windows of
/// `rate_limit`, the shorter one first, or the credits when the answer has
/// no window. An answer with neither is unexpected.
fn limits_from(answer: &Value) -> Result<Limits, SourceError> {
    let Some(fields) = answer.as_object() else {
        return Err(SourceError::UnexpectedAnswer(String::from(
            "the answer is not a JSON object",
        )));
    };

    let mut windows: Vec<(Option<u64>, LimitWindow)> =
        ["primary_window", "secondary_window"]
            .iter()
            .filter_map(|field| {
                usage_window(fields.get("rate_limit")?.get(*field)?)
            })
            .collect();
    windows.sort_by_key(|(seconds, _)| seconds.unwrap_or(u64::MAX));

    let windows: Vec<LimitWindow> =
        windows.into_iter().map(|(_, window)| window).collect();
    let credits = fields.get("credits").and_then(credits);
    let plan = fields
        .get("plan_type")
        .and_then(Value::as_str)
        .and_then(plan_name);

    let billing = if !windows.is_empty() {
        Billing::UsageWindows(UsageWindows::new(windows, None))
    } else if let Some(credits) = credits {
        Billing::Credits(credits)
    } else {
        return Err(SourceError::UnexpectedAnswer(String::from(
            "no usage window and no credits in the answer",
        )));
    };

    Ok(Limits::new(billing, plan))
}

/// One window with its length in seconds: `used_percent` is the percent
/// used, from 0 to 100, and `reset_at` a time in seconds since 1970.
fn usage_window(window: &Value) -> Option<(Option<u64>, LimitWindow)> {
    let percent_used = window.get("used_percent")?.as_f64()?;
    let seconds = window.get("limit_window_seconds").and_then(Value::as_u64);
    let resets_at =
        window
            .get("reset_at")
            .and_then(Value::as_u64)
            .and_then(|seconds| {
                SystemTime::UNIX_EPOCH.checked_add(Duration::from_secs(seconds))
            });

    let name = window_name(seconds);

    Some((seconds, LimitWindow::new(name, percent_used, resets_at)))
}

/// The name of a window from its length, such as "5 hours" or "7 days".
fn window_name(seconds: Option<u64>) -> String {
    const MINUTE: u64 = 60;
    const HOUR: u64 = 60 * MINUTE;
    const DAY: u64 = 24 * HOUR;

    let (count, one, many) = match seconds {
        Some(seconds) if seconds > 0 && seconds % DAY == 0 => {
            (seconds / DAY, "day", "days")
        },
        Some(seconds) if seconds > 0 && seconds % HOUR == 0 => {
            (seconds / HOUR, "hour", "hours")
        },
        Some(seconds) if seconds >= MINUTE => (seconds / MINUTE, "min", "min"),
        _ => return String::from("Limit"),
    };

    let unit = if count == 1 { one } else { many };
    format!("{count} {unit}")
}

/// The credits: unlimited, or the balance of an account that has credits.
/// The balance comes as a number or as text. The answer has no pool size,
/// so nothing is known of the credits used.
fn credits(credits: &Value) -> Option<Credits> {
    if credits.get("unlimited").and_then(Value::as_bool) == Some(true) {
        return Some(Credits::unlimited());
    }
    if credits.get("has_credits").and_then(Value::as_bool) != Some(true) {
        return None;
    }

    let balance =
        credits
            .get("balance")
            .and_then(|balance| match balance.as_str() {
                Some(text) => text.trim().parse().ok(),
                None => balance.as_f64(),
            });

    Some(Credits::new(balance, None, None, None))
}

/// The plan as shown, such as "Plus" for `plus` or "Free workspace" for
/// `free_workspace`.
fn plan_name(plan_type: &str) -> Option<String> {
    let words = plan_type.trim().replace('_', " ");
    let mut letters = words.chars();
    let first = letters.next()?;

    Some(format!("{}{}", first.to_uppercase(), letters.as_str()))
}

/// The text, when it is there and not blank.
fn filled(text: Option<&str>) -> Option<&str> {
    let text = text?.trim();

    (!text.is_empty()).then_some(text)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::https::{HttpsAnswer, MockHttpsGet};
    use crate::testing::{TempDir, at, codex_profile, made_up_jwt, write_file};

    /// 2026-11-16T10:00:00Z.
    const NOW: u64 = 1_794_823_200;

    const FULL_ANSWER: &str = r#"{
        "plan_type": "plus",
        "rate_limit": {
            "allowed": true,
            "limit_reached": false,
            "primary_window": {"used_percent": 42, "limit_window_seconds": 18000, "reset_after_seconds": 3600, "reset_at": 1794826800},
            "secondary_window": {"used_percent": 17, "limit_window_seconds": 604800, "reset_after_seconds": 86400, "reset_at": 1794909600}
        },
        "credits": {"has_credits": true, "unlimited": false, "balance": "12.5"}
    }"#;

    const FIVE_HOUR_ANSWER: &str = r#"{"rate_limit": {
        "primary_window": {"used_percent": 42, "limit_window_seconds": 18000, "reset_at": 1794826800}
    }}"#;

    /// A token that expires in 2100.
    fn valid_token() -> String {
        made_up_jwt(r#"{"exp": 4102444800}"#)
    }

    fn auth(token: &str, account_id: &str) -> String {
        format!(
            r#"{{
                "OPENAI_API_KEY": null,
                "tokens": {{
                    "id_token": "made-up-id-token",
                    "access_token": "{token}",
                    "refresh_token": "made-up-refresh-token",
                    "account_id": {account_id}
                }},
                "last_refresh": "2026-11-16T09:00:00Z"
            }}"#
        )
    }

    /// A config folder with a token that expires in 2100.
    fn logged_in(temp: &TempDir) -> PathBuf {
        let dir = temp.path().join(".codex-work");
        write_file(
            &dir.join("auth.json"),
            &auth(&valid_token(), r#""00000000-0000-4000-8000-000000000000""#),
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
        UsageRequest::new(Box::new(client)).limits(&codex_profile(dir))
    }

    /// The windows of `limits`; any other billing fails the test.
    fn windows(limits: &Limits) -> &[LimitWindow] {
        match limits.billing() {
            Billing::UsageWindows(usage) => usage.windows(),
            other => panic!("expected usage windows, got {other:?}"),
        }
    }

    /// The limits of an answer with one five-hour window and no plan.
    fn five_hour_limits() -> Limits {
        let window = LimitWindow::new("5 hours", 42.0, Some(at(NOW + 3600)));

        Limits::new(
            Billing::UsageWindows(UsageWindows::new(vec![window], None)),
            None,
        )
    }

    #[test]
    fn a_full_answer_gives_both_windows_and_the_plan() {
        let temp = TempDir::new();
        let dir = logged_in(&temp);
        let bearer = format!("Bearer {}", valid_token());
        let mut client = MockHttpsGet::new();
        client
            .expect_get()
            .withf(move |request| {
                request.url == "https://chatgpt.com/backend-api/wham/usage"
                    && request.headers
                        == [
                            ("Authorization", bearer.clone()),
                            ("Accept", String::from("application/json")),
                            (
                                "ChatGPT-Account-Id",
                                String::from(
                                    "00000000-0000-4000-8000-000000000000",
                                ),
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

        assert_eq!(
            limits,
            Limits::new(
                Billing::UsageWindows(UsageWindows::new(
                    vec![
                        LimitWindow::new("5 hours", 42.0, Some(at(NOW + 3600))),
                        LimitWindow::new(
                            "7 days",
                            17.0,
                            Some(at(NOW + 86_400))
                        ),
                    ],
                    None,
                )),
                Some(String::from("Plus")),
            )
        );
    }

    #[test]
    fn the_shorter_window_comes_first_and_odd_lengths_are_named() {
        let temp = TempDir::new();
        let dir = logged_in(&temp);
        let answer = r#"{"rate_limit": {
            "primary_window": {"used_percent": 70.5, "limit_window_seconds": 604800},
            "secondary_window": {"used_percent": 5, "limit_window_seconds": 3600, "reset_at": null}
        }}"#;

        let limits = limits_in(&dir, answering(200, answer))
            .expect("the limits are read");

        assert_eq!(
            windows(&limits),
            [
                LimitWindow::new("1 hour", 5.0, None),
                LimitWindow::new("7 days", 70.5, None),
            ]
        );
    }

    #[test]
    fn a_missing_window_is_left_out() {
        let temp = TempDir::new();
        let dir = logged_in(&temp);
        let answer = r#"{"rate_limit": {
            "primary_window": {"used_percent": 42, "limit_window_seconds": 18000, "reset_at": 1794826800},
            "secondary_window": null
        }}"#;

        let limits = limits_in(&dir, answering(200, answer))
            .expect("the limits are read");

        assert_eq!(limits, five_hour_limits());
    }

    #[test]
    fn unknown_fields_and_a_window_of_another_shape_are_skipped() {
        let temp = TempDir::new();
        let dir = logged_in(&temp);
        let answer = r#"{
            "account_id": "made-up",
            "plan_type": "free_workspace",
            "rate_limit": {
                "primary_window": {"used_percent": "a lot", "limit_window_seconds": 18000},
                "secondary_window": {"used_percent": 9, "limit_window_seconds": 172800, "reset_at": "tomorrow"},
                "individual_limit": {"limit": 10}
            },
            "additional_rate_limits": [{"limit_name": "Spark"}],
            "spend_control": {"anything": true}
        }"#;

        let limits = limits_in(&dir, answering(200, answer))
            .expect("the limits are read");

        assert_eq!(windows(&limits), [LimitWindow::new("2 days", 9.0, None)]);
        assert_eq!(limits.plan(), Some("Free workspace"));
    }

    #[test]
    fn an_answer_with_credits_and_no_windows_is_credits() {
        let temp = TempDir::new();
        let dir = logged_in(&temp);
        let unlimited = r#"{"plan_type": "enterprise", "rate_limit": null, "credits": {"has_credits": true, "unlimited": true, "balance": null}}"#;
        let no_balance = r#"{"rate_limit": {"primary_window": null}, "credits": {"has_credits": true, "unlimited": false}}"#;

        let unlimited = limits_in(&dir, answering(200, unlimited))
            .expect("the limits are read");
        let no_balance = limits_in(&dir, answering(200, no_balance))
            .expect("the limits are read");

        assert_eq!(
            unlimited,
            Limits::new(
                Billing::Credits(Credits::unlimited()),
                Some(String::from("Enterprise"))
            )
        );
        assert_eq!(
            no_balance,
            Limits::new(
                Billing::Credits(Credits::new(None, None, None, None)),
                None
            )
        );
    }

    #[test]
    fn a_balance_with_no_total_has_no_used_credits_and_no_pool() {
        let temp = TempDir::new();
        let dir = logged_in(&temp);
        let number = r#"{"credits": {"has_credits": true, "unlimited": false, "balance": 3760}}"#;
        let text = r#"{"credits": {"has_credits": true, "unlimited": false, "balance": " 12.5 "}}"#;

        let number = limits_in(&dir, answering(200, number))
            .expect("the limits are read");
        let text =
            limits_in(&dir, answering(200, text)).expect("the limits are read");

        let Billing::Credits(credits) = number.billing() else {
            panic!("expected credits, got {:?}", number.billing());
        };
        assert_eq!(credits.left(), Some(3760.0));
        assert_eq!(credits.used(), None);
        assert_eq!(credits.pool(), None);
        assert_eq!(credits.renews_at(), None);
        assert!(!credits.is_unlimited());
        assert_eq!(
            text.billing(),
            &Billing::Credits(Credits::new(Some(12.5), None, None, None))
        );
    }

    #[test]
    fn an_answer_with_no_windows_and_no_credits_is_unexpected() {
        let temp = TempDir::new();
        let dir = logged_in(&temp);
        let none = r#"{"plan_type": "free", "credits": {"has_credits": false, "unlimited": false, "balance": "0"}}"#;

        let none = limits_in(&dir, answering(200, none));
        let empty = limits_in(&dir, answering(200, "{}"));

        let unexpected = Err(SourceError::UnexpectedAnswer(String::from(
            "no usage window and no credits in the answer",
        )));
        assert_eq!(none, unexpected);
        assert_eq!(empty, unexpected);
    }

    #[test]
    fn the_base_url_of_the_config_file_is_asked_and_no_account_no_header() {
        let temp = TempDir::new();
        let dir = temp.path().join(".codex-work");
        write_file(&dir.join("auth.json"), &auth(&valid_token(), "null"));
        write_file(
            &dir.join("config.toml"),
            "chatgpt_base_url = \"https://gateway.example.com/backend-api\"",
        );
        let mut client = MockHttpsGet::new();
        client
            .expect_get()
            .withf(|request| {
                request.url
                    == "https://gateway.example.com/backend-api/wham/usage"
                    && request
                        .headers
                        .iter()
                        .all(|(name, _)| *name != "ChatGPT-Account-Id")
            })
            .times(1)
            .return_once(|_| {
                Ok(HttpsAnswer {
                    status: 200,
                    body: FIVE_HOUR_ANSWER.as_bytes().to_vec(),
                })
            });

        let limits = limits_in(&dir, client);

        assert_eq!(limits, Ok(five_hour_limits()));
    }

    #[test]
    fn a_missing_token_file_is_not_logged_in_and_sends_nothing() {
        let temp = TempDir::new();
        let dir = temp.path().join(".codex-work");
        std::fs::create_dir_all(&dir).expect("create config folder");

        let missing = limits_in(&dir, never_sent());
        write_file(&dir.join("auth.json"), &auth("", "null"));
        let empty = limits_in(&dir, never_sent());
        write_file(&dir.join("auth.json"), "{\"tokens\": ");
        let broken = limits_in(&dir, never_sent());

        assert_eq!(missing, Err(SourceError::NotLoggedIn));
        assert_eq!(empty, Err(SourceError::NotLoggedIn));
        assert_eq!(broken, Err(SourceError::NotLoggedIn));
    }

    #[test]
    fn an_api_key_login_has_no_limits_to_read() {
        let temp = TempDir::new();
        let dir = temp.path().join(".codex-work");
        write_file(
            &dir.join("auth.json"),
            r#"{"OPENAI_API_KEY": "sk-made-up-key", "tokens": null}"#,
        );

        let failure = limits_in(&dir, never_sent());

        assert_eq!(
            failure,
            Err(SourceError::NotAvailable(String::from(
                "a login with an API key has no usage limits"
            )))
        );
    }

    #[test]
    fn an_expired_token_is_not_logged_in_and_sends_nothing() {
        let temp = TempDir::new();
        let dir = temp.path().join(".codex-work");
        let expired = made_up_jwt(r#"{"exp": 1000}"#);
        write_file(&dir.join("auth.json"), &auth(&expired, "null"));

        let failure = limits_in(&dir, never_sent());

        assert_eq!(failure, Err(SourceError::NotLoggedIn));
    }

    #[test]
    fn a_token_with_no_expiry_is_sent() {
        let temp = TempDir::new();
        let dir = temp.path().join(".codex-work");
        write_file(&dir.join("auth.json"), &auth("opaque-token", "null"));

        let limits = limits_in(&dir, answering(200, FIVE_HOUR_ANSWER));

        assert_eq!(limits, Ok(five_hour_limits()));
    }

    #[test]
    fn a_refused_request_is_not_logged_in() {
        let temp = TempDir::new();
        let dir = logged_in(&temp);

        let failure = limits_in(&dir, answering(401, ""));

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
            .return_once(|_| Err(String::from("connection refused")));

        let failure = limits_in(&dir, client);

        assert_eq!(
            failure,
            Err(SourceError::Network(String::from("connection refused")))
        );
    }

    #[test]
    fn an_unreadable_answer_is_unexpected() {
        let temp = TempDir::new();
        let dir = logged_in(&temp);

        let not_json = limits_in(&dir, answering(200, "Service unavailable"));
        let not_an_object = limits_in(&dir, answering(200, "\"usage\""));
        let server_error = limits_in(&dir, answering(502, "{}"));

        assert_eq!(
            not_json,
            Err(SourceError::UnexpectedAnswer(String::from(
                "the answer is not JSON"
            )))
        );
        assert_eq!(
            not_an_object,
            Err(SourceError::UnexpectedAnswer(String::from(
                "the answer is not a JSON object"
            )))
        );
        assert_eq!(
            server_error,
            Err(SourceError::UnexpectedAnswer(String::from(
                "the server answered 502"
            )))
        );
    }
}
