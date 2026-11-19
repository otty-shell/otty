//! The first source of Codex limits: the usage request to the ChatGPT
//! server with the profile's own login token.
//!
//! The request, its headers and the fields of the answer follow CodexBar
//! (MIT licence),
//! <https://github.com/steipete/CodexBar/blob/main/Sources/CodexBarCore/Providers/Codex/CodexOAuth/CodexOAuthUsageFetcher.swift>.
//! The address is not documented by OpenAI, so the answer is read
//! tolerantly. When the answer says the credits of a workspace are on and
//! tells no number of them, the workspace is asked for them (see
//! `workspace_credits`).

use std::path::Path;
use std::time::SystemTime;

use serde_json::Value;

use super::auth_file::{Tokens, read_auth_file};
use super::base_url::{backend_api_url, chatgpt_usage_url, usage_url};
use super::individual_limit::individual_limit_in;
use super::rate_limits::{WindowNumbers, codex_credits, codex_limits};
use super::token_expiry::token_expires_at;
use super::workspace_credits::{WorkspaceAsk, workspace_credits};
use crate::error::{SourceError, Unavailable, Unexpected};
use crate::https::{HttpsGet, HttpsRequest, json_answer};
use crate::limits::{Billing, LimitSource, Limits};
use crate::profile::Profile;
use crate::source_id::SourceId;
use crate::timestamp::time_from_seconds;

/// The usage request with the token from `<config>/auth.json`.
pub(super) struct UsageRequest {
    client: Box<dyn HttpsGet>,
}

impl UsageRequest {
    /// The source that sends its request with `client`.
    pub(super) fn new(client: Box<dyn HttpsGet>) -> Self {
        Self { client }
    }

    /// `limits`, with the credits of the workspace when the usage answer
    /// says they are on and tells no number of them. The requests about the
    /// workspace of the profile in `config_dir` send the headers of the
    /// usage `request` and nothing more; when they give no number, `limits`
    /// stay as they are.
    fn with_workspace_credits(
        &self,
        limits: Limits,
        answer: &Value,
        tokens: &Tokens,
        request: &HttpsRequest,
        config_dir: &Path,
    ) -> Limits {
        let has_no_number = matches!(
            limits.billing(),
            Billing::Credits(credits) if credits.is_only_available()
        );
        if !has_no_number {
            return limits;
        }
        let Some(account_id) = tokens.account_id() else {
            return limits;
        };
        let Some(backend_api_url) = backend_api_url(config_dir) else {
            return limits;
        };

        let ask = WorkspaceAsk {
            client: self.client.as_ref(),
            backend_api_url: &backend_api_url,
            account_id,
            headers: &request.headers,
        };
        let Some(credits) = workspace_credits(&ask, answer) else {
            return limits;
        };

        Limits::new(Billing::Credits(credits), limits.plan().map(String::from))
    }
}

impl LimitSource for UsageRequest {
    fn id(&self) -> SourceId {
        SourceId::ChatGptUsageApi
    }

    /// A missing or expired token is "not logged in", and so is a refused
    /// request. A login with an API key has no usage limits to read: it is
    /// "not available".
    fn limits(&self, profile: &Profile) -> Result<Limits, SourceError> {
        let config_dir = profile.config_dir();
        let tokens = usable_tokens(config_dir)?;
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
        let answer = json_answer(self.client.get(&request))?;
        let limits = limits_from(&answer)?;

        Ok(self.with_workspace_credits(
            limits, &answer, &tokens, &request, config_dir,
        ))
    }
}

/// The limits of the ChatGPT login of the account `account_id`, asked of
/// ChatGPT with the login's access token `token`, for a login another agent
/// keeps, such as Kilo or OpenCode (spec 2026-11-19, Decision 5). The
/// token is never renewed: an expired one is "not logged in", and nothing
/// is sent.
pub(crate) fn chatgpt_limits(
    client: &dyn HttpsGet,
    token: &str,
    account_id: &str,
) -> Result<Limits, SourceError> {
    let expires_at = token_expires_at(token);
    if expires_at.is_some_and(|expires_at| expires_at <= SystemTime::now()) {
        return Err(SourceError::NotLoggedIn);
    }

    let request = HttpsRequest {
        url: chatgpt_usage_url(),
        headers: vec![
            ("Authorization", format!("Bearer {token}")),
            ("Accept", String::from("application/json")),
            ("ChatGPT-Account-Id", account_id.to_string()),
        ],
    };
    let answer = json_answer(client.get(&request))?;

    limits_from(&answer)
}

/// The tokens in the login file of the profile in `config_dir`, when the
/// access token may be sent.
fn usable_tokens(config_dir: &Path) -> Result<Tokens, SourceError> {
    let auth = read_auth_file(config_dir).ok_or(SourceError::NotLoggedIn)?;

    let has_api_key = auth.has_api_key();
    let Some(tokens) = auth.into_chatgpt_tokens() else {
        if has_api_key {
            return Err(SourceError::NotAvailable(Unavailable::ApiKeyLogin));
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
/// `rate_limit`, or the credits when the answer has no window. The
/// individual limit of a workspace gives the credits their pool.
fn limits_from(answer: &Value) -> Result<Limits, SourceError> {
    let Some(fields) = answer.as_object() else {
        return Err(SourceError::UnexpectedAnswer(Unexpected::NotJsonObject));
    };

    let windows = ["primary_window", "secondary_window"]
        .iter()
        .filter_map(|field| {
            usage_window(fields.get("rate_limit")?.get(*field)?)
        })
        .collect();
    let credits = fields.get("credits");
    let credits = codex_credits(
        credits.and_then(|credits| credits.get("unlimited")),
        credits.and_then(|credits| credits.get("has_credits")),
        credits.and_then(|credits| credits.get("balance")),
        individual_limit(answer),
    );
    let plan_type = fields.get("plan_type").and_then(Value::as_str);

    codex_limits(windows, credits, plan_type)
}

/// The individual limit of a workspace in the usage answer: at its root,
/// else under `rate_limit`, else under `spend_control`, where team and
/// enterprise workspaces keep it. The first place that has one wins, as in
/// CodexBar (`resolvedIndividualLimit`).
fn individual_limit(answer: &Value) -> Option<&Value> {
    let spend_control = ["spend_control", "spendControl"]
        .iter()
        .find_map(|field| answer.get(*field).filter(|part| part.is_object()));

    individual_limit_in(answer)
        .or_else(|| individual_limit_in(answer.get("rate_limit")?))
        .or_else(|| individual_limit_in(spend_control?))
}

/// One window: `used_percent` is the percent used, from 0 to 100,
/// `limit_window_seconds` its length and `reset_at` a time in seconds
/// since 1970.
fn usage_window(window: &Value) -> Option<WindowNumbers> {
    let percent_used = window.get("used_percent")?.as_f64()?;
    let seconds = window.get("limit_window_seconds").and_then(Value::as_u64);
    let resets_at = window
        .get("reset_at")
        .and_then(Value::as_u64)
        .and_then(time_from_seconds);

    Some(WindowNumbers {
        percent_used,
        seconds,
        resets_at,
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::https::{HttpsAnswer, MockHttpsGet};
    use crate::limits::{
        Billing, Credits, LimitWindow, UsageWindows, WindowId,
    };
    use crate::testing::{
        NOW, TempDir, answering, at, codex_profile, made_up_jwt, write_file,
    };

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

    const ACCOUNT: &str = "00000000-0000-4000-8000-000000000000";
    const USAGE_URL: &str = "https://chatgpt.com/backend-api/wham/usage";
    const MONTHLY_URL: &str = "https://chatgpt.com/backend-api/accounts/00000000-0000-4000-8000-000000000000/spend-controls/current-user/monthly-usage";
    const BALANCE_URL: &str = "https://chatgpt.com/backend-api/accounts/00000000-0000-4000-8000-000000000000/remaining_balance";

    /// The usage answer of an enterprise workspace that keeps its limit
    /// in spend controls and tells no number of it.
    const SPEND_CONTROL_ANSWER: &str = r#"{
        "plan_type": "enterprise_cbp_usage_based",
        "account_id": "00000000-0000-4000-8000-000000000000",
        "rate_limit": null,
        "credits": {"has_credits": true, "unlimited": false, "balance": null},
        "spend_control": {"reached": false, "individual_limit": null}
    }"#;

    /// The usage answer of a business workspace with a hidden balance and
    /// no spend controls.
    const HIDDEN_BALANCE_ANSWER: &str = r#"{
        "plan_type": "business",
        "account_id": "00000000-0000-4000-8000-000000000000",
        "rate_limit": {"primary_window": null, "secondary_window": null},
        "credits": {"has_credits": true, "unlimited": false, "balance": null}
    }"#;

    const MONTHLY_ANSWER: &str = r#"{
        "current_month_usage": "3046.5",
        "effective_monthly_limit": {"limit": 7000, "enforcement_mode": "HARD_CAP", "limit_mode": "amount_credits"}
    }"#;

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

    /// A server that answers each address of `answers` once with its
    /// status and body. Any other request, or a second one, fails the test.
    fn server(answers: &[(&str, u16, &str)]) -> MockHttpsGet {
        let mut client = MockHttpsGet::new();
        for (url, status, body) in answers {
            let url = url.to_string();
            let status = *status;
            let body = body.as_bytes().to_vec();
            client
                .expect_get()
                .withf(move |request| request.url == url)
                .times(1)
                .return_once(move |_| Ok(HttpsAnswer { status, body }));
        }
        client
    }

    /// The credits of `limits`; any other billing fails the test.
    fn credits(limits: &Limits) -> Credits {
        match limits.billing() {
            Billing::Credits(credits) => *credits,
            other => panic!("expected credits, got {other:?}"),
        }
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
        let window =
            LimitWindow::new(WindowId::hours(5), 42.0, Some(at(NOW + 3600)));

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
                        LimitWindow::new(
                            WindowId::hours(5),
                            42.0,
                            Some(at(NOW + 3600))
                        ),
                        LimitWindow::new(
                            WindowId::days(7),
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
                LimitWindow::new(WindowId::hours(1), 5.0, None),
                LimitWindow::new(WindowId::days(7), 70.5, None),
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

        assert_eq!(
            windows(&limits),
            [LimitWindow::new(WindowId::days(2), 9.0, None)]
        );
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
        let no_balance = limits_in(
            &dir,
            server(&[(USAGE_URL, 200, no_balance), (BALANCE_URL, 404, "{}")]),
        )
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
    fn credits_on_with_no_balance_are_available_on_a_usage_based_plan() {
        let temp = TempDir::new();
        let dir = logged_in(&temp);
        let answer = r#"{"plan_type": "enterprise_cbp_usage_based", "rate_limit": null, "credits": {"has_credits": true, "unlimited": false, "balance": null}}"#;

        let limits = limits_in(
            &dir,
            server(&[(USAGE_URL, 200, answer), (BALANCE_URL, 403, "")]),
        );

        assert_eq!(
            limits,
            Ok(Limits::new(
                Billing::Credits(Credits::available()),
                Some(String::from("Enterprise")),
            ))
        );
    }

    #[test]
    fn the_individual_limit_is_read_from_three_places_in_order() {
        let temp = TempDir::new();
        let dir = logged_in(&temp);
        let spend_control = r#""spend_control": {"individual_limit": {"limit": "1000", "used": "36.5", "remaining_percent": 96, "reset_at": 1797465600}}"#;
        let rate_limit = r#""rate_limit": {"primary_window": null, "secondary_window": null, "individual_limit": {"limit": 100000, "used": 7761, "resets_at": 1797465600}}"#;
        let root = r#""individual_limit": {"limit": 500, "used": 100, "resets_at": 1797465600}"#;
        let credits_on = r#""credits": {"has_credits": true, "unlimited": false, "balance": null}"#;
        let all_three =
            format!("{{{root}, {rate_limit}, {spend_control}, {credits_on}}}");
        let two = format!("{{{rate_limit}, {spend_control}, {credits_on}}}");
        let only_spend_control = format!("{{{spend_control}, {credits_on}}}");

        let all_three = limits_in(&dir, answering(200, &all_three))
            .expect("the limits are read");
        let two =
            limits_in(&dir, answering(200, &two)).expect("the limits are read");
        let only_spend_control =
            limits_in(&dir, answering(200, &only_spend_control))
                .expect("the limits are read");

        let renews_at = Some(at(1_797_465_600));
        assert_eq!(
            credits(&all_three),
            Credits::new(Some(400.0), Some(100.0), Some(500.0), renews_at)
        );
        assert_eq!(
            credits(&two),
            Credits::new(
                Some(92_239.0),
                Some(7761.0),
                Some(100_000.0),
                renews_at
            )
        );
        assert_eq!(
            credits(&only_spend_control),
            Credits::new(Some(963.5), Some(36.5), Some(1000.0), renews_at)
        );
    }

    #[test]
    fn the_individual_limit_beats_a_zero_balance_and_the_camel_case_is_read() {
        let temp = TempDir::new();
        let dir = logged_in(&temp);
        let answer = r#"{
            "plan_type": "enterprise",
            "credits": {"has_credits": true, "unlimited": false, "balance": "0"},
            "spendControl": {"individualLimit": {"limit": "80000", "used": "12000.5", "remainingPercent": 85, "resetsAt": "1797465600"}}
        }"#;

        let limits = limits_in(&dir, answering(200, answer))
            .expect("the limits are read");

        assert_eq!(
            limits,
            Limits::new(
                Billing::Credits(Credits::new(
                    Some(67_999.5),
                    Some(12_000.5),
                    Some(80_000.0),
                    Some(at(1_797_465_600)),
                )),
                Some(String::from("Enterprise")),
            )
        );
    }

    #[test]
    fn with_no_numbers_the_monthly_usage_of_the_workspace_gives_the_pool() {
        let temp = TempDir::new();
        let dir = logged_in(&temp);
        let headers = vec![
            ("Authorization", format!("Bearer {}", valid_token())),
            ("Accept", String::from("application/json")),
            ("ChatGPT-Account-Id", String::from(ACCOUNT)),
        ];
        let mut client = MockHttpsGet::new();
        for (url, body) in [
            (USAGE_URL, SPEND_CONTROL_ANSWER),
            (MONTHLY_URL, MONTHLY_ANSWER),
        ] {
            let headers = headers.clone();
            client
                .expect_get()
                .withf(move |request| {
                    request.url == url && request.headers == headers
                })
                .times(1)
                .return_once(move |_| {
                    Ok(HttpsAnswer {
                        status: 200,
                        body: body.as_bytes().to_vec(),
                    })
                });
        }

        let limits = limits_in(&dir, client);

        assert_eq!(
            limits,
            Ok(Limits::new(
                Billing::Credits(Credits::new(
                    Some(3953.5),
                    Some(3046.5),
                    Some(7000.0),
                    None,
                )),
                Some(String::from("Enterprise")),
            ))
        );
    }

    #[test]
    fn with_no_pool_the_balance_of_the_workspace_is_what_is_left() {
        let temp = TempDir::new();
        let dir = logged_in(&temp);
        let client = server(&[
            (USAGE_URL, 200, HIDDEN_BALANCE_ANSWER),
            (BALANCE_URL, 200, r#"{"balance": "1234"}"#),
        ]);

        let limits = limits_in(&dir, client);

        assert_eq!(
            limits,
            Ok(Limits::new(
                Billing::Credits(Credits::new(Some(1234.0), None, None, None)),
                Some(String::from("Enterprise")),
            ))
        );
    }

    #[test]
    fn a_monthly_limit_that_is_turned_off_asks_for_the_balance_next() {
        let temp = TempDir::new();
        let dir = logged_in(&temp);
        let off = r#"{"current_month_usage": 12, "effective_monthly_limit": {"limit": 7000, "enforcement_mode": "disabled"}}"#;
        let client = server(&[
            (USAGE_URL, 200, SPEND_CONTROL_ANSWER),
            (MONTHLY_URL, 200, off),
            (BALANCE_URL, 200, r#"{"balance": 0}"#),
        ]);

        let limits = limits_in(&dir, client).expect("the limits are read");

        assert_eq!(credits(&limits), Credits::new(Some(0.0), None, None, None));
    }

    #[test]
    fn refused_workspace_requests_leave_the_credits_available() {
        let temp = TempDir::new();
        let dir = logged_in(&temp);
        let client = server(&[
            (USAGE_URL, 200, SPEND_CONTROL_ANSWER),
            (MONTHLY_URL, 404, r#"{"error": "not found"}"#),
            (BALANCE_URL, 403, r#"{"error": "owners only"}"#),
        ]);

        let limits = limits_in(&dir, client);

        assert_eq!(
            limits,
            Ok(Limits::new(
                Billing::Credits(Credits::available()),
                Some(String::from("Enterprise")),
            ))
        );
    }

    #[test]
    fn no_connection_or_an_odd_answer_of_the_workspace_is_no_failure() {
        let temp = TempDir::new();
        let dir = logged_in(&temp);
        let mut client = server(&[
            (USAGE_URL, 200, SPEND_CONTROL_ANSWER),
            (BALANCE_URL, 200, "<html>sign in</html>"),
        ]);
        client
            .expect_get()
            .withf(|request| request.url == MONTHLY_URL)
            .times(1)
            .return_once(|_| {
                Err(SourceError::Network(String::from("connection reset")))
            });

        let limits = limits_in(&dir, client).expect("the limits are read");

        assert_eq!(credits(&limits), Credits::available());
    }

    #[test]
    fn a_personal_plan_never_asks_for_the_monthly_usage() {
        let temp = TempDir::new();
        let dir = logged_in(&temp);
        let plus =
            SPEND_CONTROL_ANSWER.replace("enterprise_cbp_usage_based", "plus");
        let client = server(&[
            (USAGE_URL, 200, plus.as_str()),
            (BALANCE_URL, 500, "{}"),
        ]);

        let limits = limits_in(&dir, client).expect("the limits are read");

        assert_eq!(credits(&limits), Credits::available());
    }

    #[test]
    fn no_account_another_account_or_another_server_asks_nothing_more() {
        let temp = TempDir::new();
        let no_account = temp.path().join(".codex-no-account");
        write_file(
            &no_account.join("auth.json"),
            &auth(&valid_token(), "null"),
        );
        let other_account = logged_in(&temp);
        let other_server = temp.path().join(".codex-proxy");
        write_file(
            &other_server.join("auth.json"),
            &auth(&valid_token(), &format!("\"{ACCOUNT}\"")),
        );
        write_file(
            &other_server.join("config.toml"),
            "chatgpt_base_url = \"https://proxy.example.com/codex\"",
        );
        let other = SPEND_CONTROL_ANSWER.replace(
            r#""account_id": "00000000-0000-4000-8000-000000000000""#,
            r#""account_id": "11111111-1111-4111-8111-111111111111""#,
        );

        let no_account = limits_in(
            &no_account,
            server(&[(USAGE_URL, 200, SPEND_CONTROL_ANSWER)]),
        );
        let other_account = limits_in(
            &other_account,
            server(&[(USAGE_URL, 200, other.as_str())]),
        );
        let other_server = limits_in(
            &other_server,
            server(&[(
                "https://proxy.example.com/codex/api/codex/usage",
                200,
                SPEND_CONTROL_ANSWER,
            )]),
        );

        for limits in [no_account, other_account, other_server] {
            let limits = limits.expect("the limits are read");
            assert_eq!(credits(&limits), Credits::available());
        }
    }

    #[test]
    fn no_credits_with_a_zero_balance_are_not_credits() {
        let temp = TempDir::new();
        let dir = logged_in(&temp);
        let only_no_credits = r#"{"plan_type": "prolite", "credits": {"has_credits": false, "balance": "0"}}"#;
        let with_windows = r#"{"plan_type": "prolite", "rate_limit": {
            "primary_window": {"used_percent": 42, "limit_window_seconds": 18000, "reset_at": 1794826800}
        }, "credits": {"has_credits": false, "balance": "0"}}"#;
        let unlimited = r#"{"plan_type": "enterprise", "credits": {"has_credits": true, "unlimited": true}}"#;

        let only_no_credits = limits_in(&dir, answering(200, only_no_credits));
        let with_windows = limits_in(&dir, answering(200, with_windows))
            .expect("the limits are read");
        let unlimited = limits_in(&dir, answering(200, unlimited))
            .expect("the limits are read");

        assert_eq!(
            only_no_credits,
            Err(SourceError::UnexpectedAnswer(Unexpected::NoLimits))
        );
        assert_eq!(with_windows.billing(), five_hour_limits().billing());
        assert_eq!(with_windows.plan(), Some("Pro 100"));
        assert_eq!(
            unlimited.billing(),
            &Billing::Credits(Credits::unlimited())
        );
    }

    #[test]
    fn an_answer_with_no_windows_and_no_credits_is_unexpected() {
        let temp = TempDir::new();
        let dir = logged_in(&temp);
        let none = r#"{"plan_type": "free", "credits": {"has_credits": false, "unlimited": false, "balance": "0"}}"#;

        let none = limits_in(&dir, answering(200, none));
        let empty = limits_in(&dir, answering(200, "{}"));

        let unexpected =
            Err(SourceError::UnexpectedAnswer(Unexpected::NoLimits));
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
            Err(SourceError::NotAvailable(Unavailable::ApiKeyLogin))
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
        client.expect_get().times(1).return_once(|_| {
            Err(SourceError::Network(String::from("connection refused")))
        });

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
            Err(SourceError::UnexpectedAnswer(Unexpected::NotJson))
        );
        assert_eq!(
            not_an_object,
            Err(SourceError::UnexpectedAnswer(Unexpected::NotJsonObject))
        );
        assert_eq!(
            server_error,
            Err(SourceError::UnexpectedAnswer(Unexpected::ServerStatus(502)))
        );
    }

    /// Spec 2026-11-19: the limits of a ChatGPT login Kilo keeps are asked
    /// of ChatGPT with that login's own token and account.
    #[test]
    fn the_limits_of_a_login_of_another_agent_are_asked_with_its_token() {
        let token = valid_token();
        let expected = format!("Bearer {token}");
        let mut client = MockHttpsGet::new();
        client
            .expect_get()
            .withf(move |request| {
                request.url == USAGE_URL
                    && request
                        .headers
                        .contains(&("Authorization", expected.clone()))
                    && request.headers.contains(&(
                        "ChatGPT-Account-Id",
                        String::from(ACCOUNT),
                    ))
            })
            .times(1)
            .return_once(|_| {
                Ok(HttpsAnswer {
                    status: 200,
                    body: FIVE_HOUR_ANSWER.as_bytes().to_vec(),
                })
            });

        let limits = chatgpt_limits(&client, &token, ACCOUNT);

        assert!(
            matches!(
                limits.as_ref().map(Limits::billing),
                Ok(Billing::UsageWindows(_))
            ),
            "{limits:?}"
        );
    }

    #[test]
    fn an_expired_token_of_another_agent_is_not_sent() {
        let expired = made_up_jwt(r#"{"exp": 1}"#);
        let mut client = MockHttpsGet::new();
        client.expect_get().never();

        let limits = chatgpt_limits(&client, &expired, ACCOUNT);

        assert_eq!(limits, Err(SourceError::NotLoggedIn));
    }
}
