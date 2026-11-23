//! The credits of a team or enterprise workspace, asked of the ChatGPT
//! server when the usage answer says the credits are on and tells no
//! number of them.
//!
//! Two requests may be sent, each at most once, with the same token and
//! `ChatGPT-Account-Id` as the usage request:
//!
//! - `GET <backend>/accounts/<account>/spend-controls/current-user/monthly-usage`:
//!   the credits the user spent this month against the workspace limit,
//!   asked only when the usage answer has a `spend_control` part and the
//!   plan is not a personal one;
//! - `GET <backend>/accounts/<account>/remaining_balance`: the balance of
//!   the workspace, asked when the first one gave no pool.
//!
//! Any failure of them leaves the credits as the usage answer told them:
//! they add numbers, they never break the read. The addresses, the gates
//! and the fields follow CodexBar (MIT licence):
//! <https://github.com/steipete/CodexBar/blob/main/Sources/CodexBarCore/Providers/Codex/CodexOAuth/CodexOAuthUsageFetcher.swift>,
//! <https://github.com/steipete/CodexBar/blob/main/Sources/CodexBarCore/Providers/Codex/CodexSpendControlsMonthlyUsage.swift>
//! and `applyingWorkspaceRemainingBalance` /
//! `applyingSpendControlsMonthlyLimit` in
//! <https://github.com/steipete/CodexBar/blob/main/Sources/CodexBarCore/Providers/Codex/CodexProviderDescriptor.swift>.

use otty_plugin_sdk::Key;
use otty_plugin_sdk::agents::{Credits, ReadError};
use serde_json::Value;

use crate::chatgpt_usage::usage_request;
use crate::rate_limits::{codex_number, no_numbers};
use crate::requests::{json_answer, percent_encoded};

/// The path of the monthly usage of the user, after the account.
const MONTHLY_USAGE_PATH: &str = "/spend-controls/current-user/monthly-usage";
/// The path of the balance of the workspace, after the account.
const BALANCE_PATH: &str = "/remaining_balance";
/// The plans of one person, which have no workspace limit to ask for.
const PERSONAL_PLANS: [&str; 5] = ["guest", "free", "go", "plus", "pro"];
/// The enforcement modes of a monthly limit that is turned off.
const LIMIT_OFF_MODES: [&str; 4] = ["none", "off", "disabled", "no_limit"];

/// What the requests about the workspace need. A plain parameter bag.
pub(crate) struct WorkspaceAsk<'a> {
    /// The ChatGPT backend, such as `https://chatgpt.com/backend-api`.
    pub(crate) backend_api_url: &'a str,
    /// The workspace of the login.
    pub(crate) account_id: &'a str,
    /// The login token, sent as the usage request sends it.
    pub(crate) key: &'a Key,
}

/// The credits of the workspace, from the monthly usage of the user, else
/// from the balance of the workspace; `None` when neither request gave a
/// number, or when the usage answer is about another workspace than the
/// one asked for.
pub(crate) fn workspace_credits(
    ask: &WorkspaceAsk<'_>,
    usage_answer: &Value,
) -> Option<Credits> {
    if names_another_account(usage_answer, ask.account_id) {
        return None;
    }

    if has_monthly_limit(usage_answer) {
        let monthly = ask_account(ask, MONTHLY_USAGE_PATH);
        if let Some(credits) = monthly.as_ref().and_then(monthly_usage_credits)
        {
            return Some(credits);
        }
    }

    ask_account(ask, BALANCE_PATH)
        .as_ref()
        .and_then(balance_credits)
}

/// Whether the usage answer is about another workspace than `account_id`:
/// then the workspace requests would mix the numbers of two accounts.
fn names_another_account(usage_answer: &Value, account_id: &str) -> bool {
    ["account_id", "accountId"]
        .iter()
        .filter_map(|field| usage_answer.get(*field)?.as_str())
        .map(str::trim)
        .find(|named| !named.is_empty())
        .is_some_and(|named| named != account_id)
}

/// Whether the workspace may have a monthly limit: the usage answer has a
/// `spend_control` part, even an empty one, and the plan is not one of a
/// single person. An unknown or missing plan may have one.
fn has_monthly_limit(usage_answer: &Value) -> bool {
    let has_spend_control = usage_answer.get("spend_control").is_some()
        || usage_answer.get("spendControl").is_some();
    let plan = usage_answer.get("plan_type").and_then(Value::as_str);
    let is_personal = plan.is_some_and(|plan| PERSONAL_PLANS.contains(&plan));

    has_spend_control && !is_personal
}

/// The JSON answer to `GET <backend>/accounts/<account><path>`, sent with
/// the headers of the usage request; `None` for no answer, a refusal, any
/// other status or a body that is not JSON.
fn ask_account(ask: &WorkspaceAsk<'_>, path: &str) -> Option<Value> {
    let account = percent_encoded(ask.account_id);
    let url = format!("{}/accounts/{account}{path}", ask.backend_api_url);
    let request = usage_request(&url, ask.key, ask.account_id);

    let refused = || ReadError::Failed(format!("{url} refused the login"));

    json_answer(&request, &url, refused).ok()
}

/// The credits of a monthly usage answer: the pool is the workspace limit
/// of the month, the used part what the user spent, never below zero.
/// `None` when the limit is missing, not above zero or turned off, or when
/// a number can not be read: then nothing is known for sure.
fn monthly_usage_credits(answer: &Value) -> Option<Credits> {
    let limit = answer.get("effective_monthly_limit")?;
    let pool = codex_number(limit.get("limit")?)?;
    if pool <= 0.0 {
        return None;
    }

    match limit.get("enforcement_mode") {
        None | Some(Value::Null) => {},
        Some(Value::String(mode)) => {
            let mode = mode.to_lowercase();
            if LIMIT_OFF_MODES.contains(&mode.as_str()) {
                return None;
            }
        },
        Some(_) => return None,
    }
    let used = match answer.get("current_month_usage") {
        None | Some(Value::Null) => 0.0,
        Some(usage) => codex_number(usage)?,
    };

    let used = used.max(0.0);
    let left = (pool - used).max(0.0);
    Some(Credits {
        left: Some(left),
        used: Some(used),
        pool: Some(pool),
        ..no_numbers()
    })
}

/// The credits of a balance answer: what is left of the workspace, never
/// below zero; `None` when the balance is missing or not a number.
fn balance_credits(answer: &Value) -> Option<Credits> {
    let balance = codex_number(answer.get("balance")?)?;

    Some(Credits {
        left: Some(balance.max(0.0)),
        ..no_numbers()
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn credits(left: f64, used: Option<f64>, pool: Option<f64>) -> Credits {
        Credits {
            left: Some(left),
            used,
            pool,
            ..no_numbers()
        }
    }

    fn monthly(usage: Value, limit: Value) -> Value {
        json!({
            "current_month_usage": usage,
            "effective_monthly_limit": {
                "limit": limit,
                "enforcement_mode": "HARD_CAP",
                "limit_mode": "amount_credits"
            }
        })
    }

    #[test]
    fn the_monthly_usage_is_the_used_part_of_the_workspace_limit() {
        let answer = monthly(json!(3046.5), json!(7000));

        let read = monthly_usage_credits(&answer);

        assert_eq!(read, Some(credits(3953.5, Some(3046.5), Some(7000.0))));
    }

    #[test]
    fn monthly_numbers_may_come_as_text_and_a_negative_usage_is_zero() {
        let answer = monthly(json!("-20"), json!(" 7000 "));

        let read = monthly_usage_credits(&answer);

        assert_eq!(read, Some(credits(7000.0, Some(0.0), Some(7000.0))));
    }

    #[test]
    fn an_active_or_unknown_enforcement_mode_keeps_the_limit() {
        for mode in [json!("HARD_CAP"), json!("SOFT_CAP"), json!("future_mode")]
        {
            let answer = json!({
                "current_month_usage": 12,
                "effective_monthly_limit": {"limit": 7000, "enforcement_mode": mode}
            });

            assert!(monthly_usage_credits(&answer).is_some(), "{mode}");
        }
        let no_mode = json!({
            "current_month_usage": 12,
            "effective_monthly_limit": {"limit": 7000}
        });
        assert!(monthly_usage_credits(&no_mode).is_some());
    }

    #[test]
    fn a_limit_that_is_turned_off_is_no_pool_in_any_letter_case() {
        for mode in ["NONE", "disabled", "OFF", "no_limit"] {
            let answer = json!({
                "current_month_usage": 12,
                "effective_monthly_limit": {"limit": 7000, "enforcement_mode": mode}
            });

            assert_eq!(monthly_usage_credits(&answer), None, "{mode}");
        }
    }

    #[test]
    fn a_missing_zero_or_unreadable_monthly_number_is_no_pool() {
        let cases = [
            monthly(json!(12), json!(0)),
            monthly(json!(12), json!({"amount": 7000})),
            monthly(json!(12), json!("Infinity")),
            monthly(json!({"amount": 12}), json!(7000)),
            monthly(json!("NaN"), json!(7000)),
            json!({"current_month_usage": 12, "effective_monthly_limit": {"enforcement_mode": "HARD_CAP"}}),
            json!({"current_month_usage": 12, "effective_monthly_limit": {"limit": 7000, "enforcement_mode": {"mode": "HARD_CAP"}}}),
            json!({"current_month_usage": 12}),
            json!({"effective_monthly_limit": "7000"}),
        ];

        for answer in cases {
            assert_eq!(monthly_usage_credits(&answer), None, "{answer}");
        }
    }

    #[test]
    fn a_missing_monthly_usage_is_nothing_used() {
        let answer = json!({"effective_monthly_limit": {"limit": 7000}});

        let read = monthly_usage_credits(&answer);

        assert_eq!(read, Some(credits(7000.0, Some(0.0), Some(7000.0))));
    }

    #[test]
    fn the_balance_may_come_as_text_and_is_never_below_zero() {
        assert_eq!(
            balance_credits(&json!({"balance": "1234"})),
            Some(credits(1234.0, None, None))
        );
        assert_eq!(
            balance_credits(&json!({"balance": -4})),
            Some(credits(0.0, None, None))
        );
    }

    #[test]
    fn a_missing_or_unreadable_balance_is_no_number() {
        for answer in [
            json!({}),
            json!({"balance": null}),
            json!({"balance": "NaN"}),
            json!({"balance": "Infinity"}),
            json!({"balance": {"amount": 14}}),
            json!({"balance": true}),
        ] {
            assert_eq!(balance_credits(&answer), None, "{answer}");
        }
    }
}
