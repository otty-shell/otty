//! The limits of a ChatGPT login: the usage request to the ChatGPT server
//! with the login's access token, for the provider part `openai`, with the
//! token of an account's login in one of its Codex folders. The request, its headers and the fields of the answer
//! follow CodexBar (MIT licence),
//! <https://github.com/steipete/CodexBar/blob/main/Sources/CodexBarCore/Providers/Codex/CodexOAuth/CodexOAuthUsageFetcher.swift>.
//! The address is not documented by OpenAI, so the answer is read
//! tolerantly. When the answer says the credits of a workspace are on and
//! tells no number of them, the workspace is asked for them (see
//! `workspace_credits`).
//!
//! A login behind a gateway (`chatgpt_base_url`) is a login for the
//! gateway's host, not for OpenAI: this part does not serve it, and its
//! token never goes to `chatgpt.com`.

use otty_plugin_sdk::agents::ReadError;
use otty_plugin_sdk::http::Request;
use otty_plugin_sdk::{Key, Need};
use serde_json::Value;

use crate::individual_limit::individual_limit_in;
use crate::limits::Limits;
use crate::rate_limits::{
    WindowNumbers, codex_credits, codex_limits, millis_of_seconds, no_numbers,
};
use crate::requests::json_answer;
use crate::token_expiry::has_expired;
use crate::workspace_credits::{WorkspaceAsk, workspace_credits};

/// The ChatGPT backend the requests go to.
const BACKEND_API_URL: &str = "https://chatgpt.com/backend-api";
/// The path of the usage on the ChatGPT backend.
const USAGE_PATH: &str = "/wham/usage";

/// The limits of the ChatGPT login of the account `account_id`, asked of
/// ChatGPT with the login's access token `key`, with the credits of the
/// workspace when the usage answer says they are on and tells no number
/// of them. The token is never renewed: one whose claims say it expired
/// is not sent. An expired or refused login is
/// [`ReadError::NeedsUser`] with `login_need`, what the user does to log
/// in again where the token sits.
pub(crate) fn chatgpt_limits(
    key: &Key,
    account_id: &str,
    login_need: &Need,
) -> Result<Limits, ReadError> {
    let refused = || ReadError::NeedsUser(login_need.clone());
    if has_expired(key) {
        return Err(refused());
    }

    let url = format!("{BACKEND_API_URL}{USAGE_PATH}");
    let request = usage_request(&url, key, account_id);
    let answer = json_answer(&request, &url, refused)?;
    let limits = limits_from(&answer)?;

    let ask = WorkspaceAsk {
        backend_api_url: BACKEND_API_URL,
        account_id,
        key,
    };
    Ok(with_workspace_credits(limits, &answer, &ask))
}

/// A `GET` of `url` as the usage request sends it: the token of `key` and
/// the workspace `account_id`, nothing more.
pub(crate) fn usage_request<'key>(
    url: &str,
    key: &'key Key,
    account_id: &str,
) -> Request<'key> {
    Request::get(url)
        .header_with_key("authorization", "Bearer ", key)
        .header("accept", "application/json")
        .header("chatgpt-account-id", account_id)
}

/// `limits`, with the credits of the workspace when the usage answer says
/// they are on and tells no number of them. When the workspace requests
/// give no number, `limits` stay as they are.
fn with_workspace_credits(
    mut limits: Limits,
    answer: &Value,
    ask: &WorkspaceAsk<'_>,
) -> Limits {
    if limits.credits != Some(no_numbers()) {
        return limits;
    }
    let Some(credits) = workspace_credits(ask, answer) else {
        return limits;
    };

    limits.credits = Some(credits);
    limits
}

/// The limits in the usage answer and the plan: the two windows of
/// `rate_limit`, or the credits when the answer has no window. The
/// individual limit of a workspace gives the credits their pool.
fn limits_from(answer: &Value) -> Result<Limits, ReadError> {
    let Some(fields) = answer.as_object() else {
        return Err(ReadError::Unexpected(
            "the usage answer is not a JSON object".to_string(),
        ));
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
    let minutes = window
        .get("limit_window_seconds")
        .and_then(Value::as_u64)
        .and_then(|seconds| u32::try_from(seconds / 60).ok());
    let resets_at_ms = window.get("reset_at").and_then(millis_of_seconds);

    Some(WindowNumbers {
        percent_used,
        minutes,
        resets_at_ms,
    })
}

#[cfg(test)]
mod tests {
    use otty_plugin_sdk::agents::{Credits, LimitWindow};
    use otty_plugin_sdk::test_host::TestHost;

    use super::*;
    use crate::testing::{TEN, host};

    /// A made-up token that expires in 2100.
    const VALID_TOKEN: &str =
        "e30.eyJleHAiOjQxMDI0NDQ4MDAsInN1YiI6Im1hZGUtdXAtYWNjZXNzIn0.c2ln";
    /// A made-up token that expired in 2023.
    const EXPIRED_TOKEN: &str =
        "e30.eyJleHAiOjE3MDAwMDAwMDAsInN1YiI6Im1hZGUtdXAtYWNjZXNzIn0.c2ln";

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

    /// A made-up need of the login the token is of.
    fn login_need() -> Need {
        Need {
            id: "login-expired".to_string(),
            title: "The login expired".to_string(),
            text: "Log in again.".to_string(),
            action: otty_plugin_sdk::NeedAction::OpenPage(
                "https://example.com".to_string(),
            ),
        }
    }
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

    /// A host at [`TEN`] whose server answers each address of `answers`
    /// with its status and body.
    fn server(answers: &[(&str, u16, &str)]) -> TestHost {
        let host = host().with_now_ms(TEN);
        for (url, status, body) in answers {
            host.answer("GET", url, *status, body.as_bytes());
        }
        host
    }

    /// The limits the login with `token` reads of `host`.
    fn limits_with(host: &TestHost, token: &str) -> Result<Limits, ReadError> {
        let key = host.key(token);

        host.as_provider("openai", || {
            chatgpt_limits(&key, ACCOUNT, &login_need())
        })
    }

    /// The limits the login with a token that expires in 2100 reads when
    /// the usage request answers `status` and `body`.
    fn limits_in(status: u16, body: &str) -> Result<Limits, ReadError> {
        let host = server(&[(USAGE_URL, status, body)]);

        limits_with(&host, VALID_TOKEN)
    }

    fn window(
        minutes: u32,
        percent_used: f64,
        reset: Option<i64>,
    ) -> LimitWindow {
        LimitWindow {
            length_minutes: Some(minutes),
            model: None,
            percent_used,
            resets_at_ms: reset,
            reset_text: None,
        }
    }

    fn credits(
        left: Option<f64>,
        used: Option<f64>,
        pool: Option<f64>,
        renews_at_ms: Option<i64>,
    ) -> Credits {
        Credits {
            left,
            used,
            pool,
            renews_at_ms,
            is_unlimited: false,
        }
    }

    fn limits(
        windows: Vec<LimitWindow>,
        credits: Option<Credits>,
        plan: Option<&str>,
    ) -> Limits {
        Limits {
            windows,
            credits,
            plan: plan.map(String::from),
            recorded_at_ms: None,
        }
    }

    /// The limits of an answer with one five-hour window and no plan.
    fn five_hour_limits() -> Limits {
        limits(vec![window(300, 42.0, Some(TEN + 3_600_000))], None, None)
    }

    #[test]
    fn a_full_answer_gives_both_windows_and_the_plan() {
        let host = server(&[(USAGE_URL, 200, FULL_ANSWER)]);

        let read = limits_with(&host, VALID_TOKEN);

        assert_eq!(
            read,
            Ok(limits(
                vec![
                    window(300, 42.0, Some(TEN + 3_600_000)),
                    window(10_080, 17.0, Some(TEN + 86_400_000)),
                ],
                None,
                Some("Plus"),
            ))
        );
        let requests = host.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].url(), USAGE_URL);
        assert_eq!(
            requests[0].header("authorization"),
            Some(format!("Bearer {VALID_TOKEN}").as_str())
        );
        assert_eq!(requests[0].header("chatgpt-account-id"), Some(ACCOUNT));
    }

    #[test]
    fn the_shorter_window_comes_first_and_odd_lengths_are_named() {
        let answer = r#"{"rate_limit": {
            "primary_window": {"used_percent": 70.5, "limit_window_seconds": 604800},
            "secondary_window": {"used_percent": 5, "limit_window_seconds": 3600, "reset_at": null}
        }}"#;

        let read = limits_in(200, answer).expect("the limits are read");

        assert_eq!(
            read.windows,
            [window(60, 5.0, None), window(10_080, 70.5, None)]
        );
    }

    #[test]
    fn a_missing_window_is_left_out() {
        let answer = r#"{"rate_limit": {
            "primary_window": {"used_percent": 42, "limit_window_seconds": 18000, "reset_at": 1794826800},
            "secondary_window": null
        }}"#;

        assert_eq!(limits_in(200, answer), Ok(five_hour_limits()));
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

        let read = limits_in(200, answer).expect("the limits are read");

        assert_eq!(read.windows, [window(2880, 9.0, None)]);
        assert_eq!(read.plan.as_deref(), Some("Free workspace"));
    }

    #[test]
    fn an_answer_with_credits_and_no_windows_is_credits() {
        let unlimited = r#"{"plan_type": "enterprise", "rate_limit": null, "credits": {"has_credits": true, "unlimited": true, "balance": null}}"#;
        let no_balance = r#"{"rate_limit": {"primary_window": null}, "credits": {"has_credits": true, "unlimited": false}}"#;

        let unlimited = limits_in(200, unlimited);
        let host =
            server(&[(USAGE_URL, 200, no_balance), (BALANCE_URL, 404, "{}")]);
        let no_balance = limits_with(&host, VALID_TOKEN);

        assert_eq!(
            unlimited,
            Ok(limits(
                Vec::new(),
                Some(Credits {
                    is_unlimited: true,
                    ..no_numbers()
                }),
                Some("Enterprise")
            ))
        );
        assert_eq!(
            no_balance,
            Ok(limits(Vec::new(), Some(no_numbers()), None))
        );
    }

    #[test]
    fn a_balance_with_no_total_has_no_used_credits_and_no_pool() {
        let number = r#"{"credits": {"has_credits": true, "unlimited": false, "balance": 3760}}"#;
        let text = r#"{"credits": {"has_credits": true, "unlimited": false, "balance": " 12.5 "}}"#;

        let number = limits_in(200, number).expect("the limits are read");
        let text = limits_in(200, text).expect("the limits are read");

        assert_eq!(
            number.credits,
            Some(credits(Some(3760.0), None, None, None))
        );
        assert_eq!(text.credits, Some(credits(Some(12.5), None, None, None)));
    }

    #[test]
    fn credits_on_with_no_balance_are_available_on_a_usage_based_plan() {
        let answer = r#"{"plan_type": "enterprise_cbp_usage_based", "rate_limit": null, "credits": {"has_credits": true, "unlimited": false, "balance": null}}"#;
        let host = server(&[(USAGE_URL, 200, answer), (BALANCE_URL, 403, "")]);

        let read = limits_with(&host, VALID_TOKEN);

        assert_eq!(
            read,
            Ok(limits(Vec::new(), Some(no_numbers()), Some("Enterprise")))
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

        let all_three = limits_in(200, &all_three).expect("the limits");
        let two = limits_in(200, &two).expect("the limits");
        let only_spend_control =
            limits_in(200, &only_spend_control).expect("the limits");

        let renews_at = Some(1_797_465_600_000);
        assert_eq!(
            all_three.credits,
            Some(credits(Some(400.0), Some(100.0), Some(500.0), renews_at))
        );
        assert_eq!(
            two.credits,
            Some(credits(
                Some(92_239.0),
                Some(7761.0),
                Some(100_000.0),
                renews_at
            ))
        );
        assert_eq!(
            only_spend_control.credits,
            Some(credits(Some(963.5), Some(36.5), Some(1000.0), renews_at))
        );
    }

    #[test]
    fn with_no_numbers_the_monthly_usage_of_the_workspace_gives_the_pool() {
        let host = server(&[
            (USAGE_URL, 200, SPEND_CONTROL_ANSWER),
            (MONTHLY_URL, 200, MONTHLY_ANSWER),
        ]);

        let read = limits_with(&host, VALID_TOKEN);

        assert_eq!(
            read,
            Ok(limits(
                Vec::new(),
                Some(credits(Some(3953.5), Some(3046.5), Some(7000.0), None)),
                Some("Enterprise"),
            ))
        );
        let requests = host.requests();
        assert_eq!(requests.len(), 2);
        for request in &requests {
            assert_eq!(
                request.header("authorization"),
                Some(format!("Bearer {VALID_TOKEN}").as_str())
            );
            assert_eq!(request.header("chatgpt-account-id"), Some(ACCOUNT));
        }
    }

    #[test]
    fn with_no_pool_the_balance_of_the_workspace_is_what_is_left() {
        let host = server(&[
            (USAGE_URL, 200, HIDDEN_BALANCE_ANSWER),
            (BALANCE_URL, 200, r#"{"balance": "1234"}"#),
        ]);

        let read = limits_with(&host, VALID_TOKEN);

        assert_eq!(
            read,
            Ok(limits(
                Vec::new(),
                Some(credits(Some(1234.0), None, None, None)),
                Some("Enterprise"),
            ))
        );
    }

    #[test]
    fn refused_workspace_requests_leave_the_credits_available() {
        let host = server(&[
            (USAGE_URL, 200, SPEND_CONTROL_ANSWER),
            (MONTHLY_URL, 404, r#"{"error": "not found"}"#),
            (BALANCE_URL, 403, r#"{"error": "owners only"}"#),
        ]);

        let read = limits_with(&host, VALID_TOKEN);

        assert_eq!(
            read,
            Ok(limits(Vec::new(), Some(no_numbers()), Some("Enterprise")))
        );
    }

    #[test]
    fn a_personal_plan_never_asks_for_the_monthly_usage() {
        let plus =
            SPEND_CONTROL_ANSWER.replace("enterprise_cbp_usage_based", "plus");
        let host = server(&[
            (USAGE_URL, 200, plus.as_str()),
            (BALANCE_URL, 500, "{}"),
        ]);

        let read = limits_with(&host, VALID_TOKEN).expect("the limits");

        assert_eq!(read.credits, Some(no_numbers()));
        assert!(host.requests().iter().all(|sent| sent.url() != MONTHLY_URL));
    }

    #[test]
    fn an_answer_about_another_account_asks_nothing_more() {
        let other = SPEND_CONTROL_ANSWER.replace(
            r#""account_id": "00000000-0000-4000-8000-000000000000""#,
            r#""account_id": "11111111-1111-4111-8111-111111111111""#,
        );
        let host = server(&[(USAGE_URL, 200, other.as_str())]);

        let read = limits_with(&host, VALID_TOKEN).expect("the limits");

        assert_eq!(read.credits, Some(no_numbers()));
        assert_eq!(host.requests().len(), 1);
    }

    #[test]
    fn an_answer_with_no_windows_and_no_credits_is_unexpected() {
        let none = r#"{"plan_type": "free", "credits": {"has_credits": false, "unlimited": false, "balance": "0"}}"#;

        for body in [none, "{}", "\"usage\"", "Service unavailable"] {
            let read = limits_in(200, body);

            assert!(
                matches!(read, Err(ReadError::Unexpected(_))),
                "{body}: {read:?}"
            );
        }
    }

    #[test]
    fn a_token_with_no_expiry_is_sent() {
        let host = server(&[(USAGE_URL, 200, FIVE_HOUR_ANSWER)]);

        let read = limits_with(&host, "opaque-token");

        assert_eq!(read, Ok(five_hour_limits()));
    }

    /// Issue 12 of spec 2026-11-19-agent-plugins: the provider part tries
    /// the login, and a token whose claims say it expired is not even sent
    /// and never renewed. Spec 2026-11-23-accounts-apart-from-profiles,
    /// Decision 9: either way the read needs the user to log in again
    /// where the token sits.
    #[test]
    fn a_refused_or_expired_login_needs_the_user_to_log_in_again() {
        let refused = limits_in(401, "");
        let host = server(&[(USAGE_URL, 200, FIVE_HOUR_ANSWER)]);
        let expired = limits_with(&host, EXPIRED_TOKEN);

        assert_eq!(refused, Err(ReadError::NeedsUser(login_need())));
        assert_eq!(expired, Err(ReadError::NeedsUser(login_need())));
        assert!(host.requests().is_empty());
    }

    #[test]
    fn a_server_error_fails_without_what_the_answer_said() {
        let read = limits_in(502, "made-up answer text");

        let Err(ReadError::Failed(text)) = read else {
            panic!("a failure: {read:?}");
        };
        assert!(text.contains("502"), "{text}");
        assert!(!text.contains("made-up answer text"), "{text}");
    }
}
