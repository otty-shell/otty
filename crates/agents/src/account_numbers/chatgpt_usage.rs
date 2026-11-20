//! The limits of a ChatGPT login: the usage request to the ChatGPT server
//! with the login's access token.
//!
//! It is kept in OTTY until the provider part `openai` of the OpenAI
//! plugin reads it with a key handle (spec 2026-11-19-agent-plugins, issue
//! 15); the Codex reader that also sent it is gone, and the plugin's
//! `codex` part reads the limits from `codex app-server` and the session
//! files instead. Today it reads the limits of a ChatGPT login that no
//! profile reads, such as one only Kilo or OpenCode keeps. The request,
//! its headers and the fields of the answer follow CodexBar (MIT licence),
//! <https://github.com/steipete/CodexBar/blob/main/Sources/CodexBarCore/Providers/Codex/CodexOAuth/CodexOAuthUsageFetcher.swift>.
//! The address is not documented by OpenAI, so the answer is read
//! tolerantly. When the answer says the credits of a workspace are on and
//! tells no number of them, the workspace is asked for them (see
//! `chatgpt_workspace_credits`).

use std::time::SystemTime;

use serde_json::Value;

use super::chatgpt_individual_limit::individual_limit_in;
use super::chatgpt_limits::{WindowNumbers, codex_credits, codex_limits};
use super::chatgpt_workspace_credits::{WorkspaceAsk, workspace_credits};
use super::token_expiry::token_expires_at;
use crate::error::{SourceError, Unexpected};
use crate::https::{HttpsGet, HttpsRequest, json_answer};
use crate::limits::{Billing, Limits};
use crate::timestamp::time_from_seconds;

/// The ChatGPT backend the requests go to.
const BACKEND_API_URL: &str = "https://chatgpt.com/backend-api";
/// The path of the usage on the ChatGPT backend.
const USAGE_PATH: &str = "/wham/usage";

/// The limits of the ChatGPT login of the account `account_id`, asked of
/// ChatGPT with the login's access token `token` (spec 2026-11-19,
/// Decision 5), with the credits of the workspace when the usage answer
/// says they are on and tells no number of them. The token is never
/// renewed: an expired one is "not logged in", and nothing is sent.
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
        url: format!("{BACKEND_API_URL}{USAGE_PATH}"),
        headers: vec![
            ("Authorization", format!("Bearer {token}")),
            ("Accept", String::from("application/json")),
            ("ChatGPT-Account-Id", account_id.to_string()),
        ],
    };
    let answer = json_answer(client.get(&request))?;
    let limits = limits_from(&answer)?;

    let ask = WorkspaceAsk {
        client,
        backend_api_url: BACKEND_API_URL,
        account_id,
        headers: &request.headers,
    };
    Ok(with_workspace_credits(limits, &answer, &ask))
}

/// `limits`, with the credits of the workspace when the usage answer says
/// they are on and tells no number of them. The requests about the
/// workspace send the headers of the usage request and nothing more; when
/// they give no number, `limits` stay as they are.
fn with_workspace_credits(
    limits: Limits,
    answer: &Value,
    ask: &WorkspaceAsk<'_>,
) -> Limits {
    let has_no_number = matches!(
        limits.billing(),
        Billing::Credits(credits) if credits.is_only_available()
    );
    if !has_no_number {
        return limits;
    }
    let Some(credits) = workspace_credits(ask, answer) else {
        return limits;
    };

    Limits::new(Billing::Credits(credits), limits.plan().map(String::from))
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
    use super::*;
    use crate::https::{HttpsAnswer, MockHttpsGet};
    use crate::limits::{
        Billing, Credits, LimitWindow, UsageWindows, WindowId,
    };
    use crate::testing::{NOW, answering, at, made_up_jwt};

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

    /// The limits the login with a token that expires in 2100 reads with
    /// `client`.
    fn limits_in(client: MockHttpsGet) -> Result<Limits, SourceError> {
        chatgpt_limits(&client, &valid_token(), ACCOUNT)
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

        let limits = limits_in(client).expect("the limits are read");

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
        let answer = r#"{"rate_limit": {
            "primary_window": {"used_percent": 70.5, "limit_window_seconds": 604800},
            "secondary_window": {"used_percent": 5, "limit_window_seconds": 3600, "reset_at": null}
        }}"#;

        let limits =
            limits_in(answering(200, answer)).expect("the limits are read");

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
        let answer = r#"{"rate_limit": {
            "primary_window": {"used_percent": 42, "limit_window_seconds": 18000, "reset_at": 1794826800},
            "secondary_window": null
        }}"#;

        let limits =
            limits_in(answering(200, answer)).expect("the limits are read");

        assert_eq!(limits, five_hour_limits());
    }

    #[test]
    fn unknown_fields_and_a_window_of_another_shape_are_skipped() {
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

        let limits =
            limits_in(answering(200, answer)).expect("the limits are read");

        assert_eq!(
            windows(&limits),
            [LimitWindow::new(WindowId::days(2), 9.0, None)]
        );
        assert_eq!(limits.plan(), Some("Free workspace"));
    }

    #[test]
    fn an_answer_with_credits_and_no_windows_is_credits() {
        let unlimited = r#"{"plan_type": "enterprise", "rate_limit": null, "credits": {"has_credits": true, "unlimited": true, "balance": null}}"#;
        let no_balance = r#"{"rate_limit": {"primary_window": null}, "credits": {"has_credits": true, "unlimited": false}}"#;

        let unlimited =
            limits_in(answering(200, unlimited)).expect("the limits are read");
        let no_balance = limits_in(server(&[
            (USAGE_URL, 200, no_balance),
            (BALANCE_URL, 404, "{}"),
        ]))
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
        let number = r#"{"credits": {"has_credits": true, "unlimited": false, "balance": 3760}}"#;
        let text = r#"{"credits": {"has_credits": true, "unlimited": false, "balance": " 12.5 "}}"#;

        let number =
            limits_in(answering(200, number)).expect("the limits are read");
        let text =
            limits_in(answering(200, text)).expect("the limits are read");

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
        let answer = r#"{"plan_type": "enterprise_cbp_usage_based", "rate_limit": null, "credits": {"has_credits": true, "unlimited": false, "balance": null}}"#;

        let limits = limits_in(server(&[
            (USAGE_URL, 200, answer),
            (BALANCE_URL, 403, ""),
        ]));

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
        let spend_control = r#""spend_control": {"individual_limit": {"limit": "1000", "used": "36.5", "remaining_percent": 96, "reset_at": 1797465600}}"#;
        let rate_limit = r#""rate_limit": {"primary_window": null, "secondary_window": null, "individual_limit": {"limit": 100000, "used": 7761, "resets_at": 1797465600}}"#;
        let root = r#""individual_limit": {"limit": 500, "used": 100, "resets_at": 1797465600}"#;
        let credits_on = r#""credits": {"has_credits": true, "unlimited": false, "balance": null}"#;
        let all_three =
            format!("{{{root}, {rate_limit}, {spend_control}, {credits_on}}}");
        let two = format!("{{{rate_limit}, {spend_control}, {credits_on}}}");
        let only_spend_control = format!("{{{spend_control}, {credits_on}}}");

        let all_three =
            limits_in(answering(200, &all_three)).expect("the limits are read");
        let two = limits_in(answering(200, &two)).expect("the limits are read");
        let only_spend_control = limits_in(answering(200, &only_spend_control))
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
        let answer = r#"{
            "plan_type": "enterprise",
            "credits": {"has_credits": true, "unlimited": false, "balance": "0"},
            "spendControl": {"individualLimit": {"limit": "80000", "used": "12000.5", "remainingPercent": 85, "resetsAt": "1797465600"}}
        }"#;

        let limits =
            limits_in(answering(200, answer)).expect("the limits are read");

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

        let limits = limits_in(client);

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
        let client = server(&[
            (USAGE_URL, 200, HIDDEN_BALANCE_ANSWER),
            (BALANCE_URL, 200, r#"{"balance": "1234"}"#),
        ]);

        let limits = limits_in(client);

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
        let off = r#"{"current_month_usage": 12, "effective_monthly_limit": {"limit": 7000, "enforcement_mode": "disabled"}}"#;
        let client = server(&[
            (USAGE_URL, 200, SPEND_CONTROL_ANSWER),
            (MONTHLY_URL, 200, off),
            (BALANCE_URL, 200, r#"{"balance": 0}"#),
        ]);

        let limits = limits_in(client).expect("the limits are read");

        assert_eq!(credits(&limits), Credits::new(Some(0.0), None, None, None));
    }

    #[test]
    fn refused_workspace_requests_leave_the_credits_available() {
        let client = server(&[
            (USAGE_URL, 200, SPEND_CONTROL_ANSWER),
            (MONTHLY_URL, 404, r#"{"error": "not found"}"#),
            (BALANCE_URL, 403, r#"{"error": "owners only"}"#),
        ]);

        let limits = limits_in(client);

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

        let limits = limits_in(client).expect("the limits are read");

        assert_eq!(credits(&limits), Credits::available());
    }

    #[test]
    fn a_personal_plan_never_asks_for_the_monthly_usage() {
        let plus =
            SPEND_CONTROL_ANSWER.replace("enterprise_cbp_usage_based", "plus");
        let client = server(&[
            (USAGE_URL, 200, plus.as_str()),
            (BALANCE_URL, 500, "{}"),
        ]);

        let limits = limits_in(client).expect("the limits are read");

        assert_eq!(credits(&limits), Credits::available());
    }

    #[test]
    fn an_answer_about_another_account_asks_nothing_more() {
        let other = SPEND_CONTROL_ANSWER.replace(
            r#""account_id": "00000000-0000-4000-8000-000000000000""#,
            r#""account_id": "11111111-1111-4111-8111-111111111111""#,
        );

        let limits = limits_in(server(&[(USAGE_URL, 200, other.as_str())]))
            .expect("the limits are read");

        assert_eq!(credits(&limits), Credits::available());
    }

    #[test]
    fn no_credits_with_a_zero_balance_are_not_credits() {
        let only_no_credits = r#"{"plan_type": "prolite", "credits": {"has_credits": false, "balance": "0"}}"#;
        let with_windows = r#"{"plan_type": "prolite", "rate_limit": {
            "primary_window": {"used_percent": 42, "limit_window_seconds": 18000, "reset_at": 1794826800}
        }, "credits": {"has_credits": false, "balance": "0"}}"#;
        let unlimited = r#"{"plan_type": "enterprise", "credits": {"has_credits": true, "unlimited": true}}"#;

        let only_no_credits = limits_in(answering(200, only_no_credits));
        let with_windows = limits_in(answering(200, with_windows))
            .expect("the limits are read");
        let unlimited =
            limits_in(answering(200, unlimited)).expect("the limits are read");

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
        let none = r#"{"plan_type": "free", "credits": {"has_credits": false, "unlimited": false, "balance": "0"}}"#;

        let none = limits_in(answering(200, none));
        let empty = limits_in(answering(200, "{}"));

        let unexpected =
            Err(SourceError::UnexpectedAnswer(Unexpected::NoLimits));
        assert_eq!(none, unexpected);
        assert_eq!(empty, unexpected);
    }

    #[test]
    fn a_token_with_no_expiry_is_sent() {
        let client = answering(200, FIVE_HOUR_ANSWER);

        let limits = chatgpt_limits(&client, "opaque-token", ACCOUNT);

        assert_eq!(limits, Ok(five_hour_limits()));
    }

    #[test]
    fn a_refused_request_is_not_logged_in() {
        let failure = limits_in(answering(401, ""));

        assert_eq!(failure, Err(SourceError::NotLoggedIn));
    }

    #[test]
    fn no_connection_is_a_network_failure() {
        let mut client = MockHttpsGet::new();
        client.expect_get().times(1).return_once(|_| {
            Err(SourceError::Network(String::from("connection refused")))
        });

        let failure = limits_in(client);

        assert_eq!(
            failure,
            Err(SourceError::Network(String::from("connection refused")))
        );
    }

    #[test]
    fn an_unreadable_answer_is_unexpected() {
        let not_json = limits_in(answering(200, "Service unavailable"));
        let not_an_object = limits_in(answering(200, "\"usage\""));
        let server_error = limits_in(answering(502, "{}"));

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
