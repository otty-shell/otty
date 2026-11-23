//! The limits `codex app-server` gives for the login of a profile.
//!
//! The plugin starts `codex -s read-only -a never app-server` with the
//! profile's config folder and talks JSON-RPC with it over its input and
//! output, one JSON object on a line. Codex leaves out the
//! `"jsonrpc": "2.0"` field of JSON-RPC 2.0, so the plugin does too.
//!
//! Only the methods of [`Method`] are ever sent: `initialize`, the
//! `initialized` notice and `account/rateLimits/read`. The request for the
//! list of sessions is never sent: Codex repairs its own index of sessions
//! while it answers it, and on the owner's computer it deleted rows of
//! that index (spec 2026-11-19-agent-plugins, Decision 13).

use otty_plugin_sdk::agents::ReadError;
use otty_plugin_sdk::programs::Talk;
use otty_plugin_sdk::{HostError, clock};
use serde_json::{Value, json};

use crate::individual_limit::individual_limit_in;
use crate::limits::Limits;
use crate::rate_limits::{
    WindowNumbers, codex_credits, codex_limits, millis_of_seconds,
};

/// The program, looked up on `PATH`.
const PROGRAM: &str = "codex";
/// The arguments that start the app server: no command it might run can
/// write, and it never asks for an approval.
pub(crate) const APP_SERVER_ARGS: [&str; 5] =
    ["-s", "read-only", "-a", "never", "app-server"];
/// How long the whole talk may take, in milliseconds, from the start of
/// the program to the answer. The user's shell reads its start files
/// first, which can take a few seconds with a version manager.
const TIME_LIMIT_MS: i64 = 10_000;
/// The id of the `initialize` request.
const INITIALIZE_ID: u64 = 1;
/// The id of the `account/rateLimits/read` request.
const RATE_LIMITS_ID: u64 = 2;

/// The only messages the plugin sends to `codex app-server`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Method {
    /// The handshake.
    Initialize,
    /// The notice that the handshake is done.
    Initialized,
    /// The limits of the login.
    ReadRateLimits,
}

impl Method {
    /// The method's name in the protocol.
    fn name(self) -> &'static str {
        match self {
            Method::Initialize => "initialize",
            Method::Initialized => "initialized",
            Method::ReadRateLimits => "account/rateLimits/read",
        }
    }

    /// The message of the method: a request with `id`, or a notice.
    fn message(self) -> Value {
        match self {
            Method::Initialize => json!({
                "id": INITIALIZE_ID,
                "method": self.name(),
                "params": {"clientInfo": {
                    "name": "otty",
                    "title": "OTTY",
                    "version": env!("CARGO_PKG_VERSION"),
                }},
            }),
            Method::Initialized => json!({"method": self.name()}),
            Method::ReadRateLimits => {
                json!({"id": RATE_LIMITS_ID, "method": self.name()})
            },
        }
    }
}

/// The limits `codex app-server` gives, started with the variables `env`
/// that point it at the profile's config folder.
///
/// A program that can not start, refuses or does not answer in time, and
/// one that ends before it answers, fail the read. OTTY stops the
/// program when the talk ends.
pub(crate) fn read_limits(
    env: &[(String, String)],
) -> Result<Limits, ReadError> {
    let args: Vec<String> =
        APP_SERVER_ARGS.iter().map(|arg| arg.to_string()).collect();
    let until_ms = clock::now_ms() + TIME_LIMIT_MS;

    let talk = Talk::start(PROGRAM, &args, env).map_err(ReadError::Host)?;
    let answer = rate_limits_answer(&talk, until_ms)?;

    limits_from(&answer)
}

/// The `result` of `account/rateLimits/read`, after the handshake.
fn rate_limits_answer(talk: &Talk, until_ms: i64) -> Result<Value, ReadError> {
    send(talk, Method::Initialize)?;
    answer(talk, INITIALIZE_ID, until_ms)?;

    send(talk, Method::Initialized)?;
    send(talk, Method::ReadRateLimits)?;

    answer(talk, RATE_LIMITS_ID, until_ms)
}

/// Writes the message of `method` to the program as one line.
fn send(talk: &Talk, method: Method) -> Result<(), ReadError> {
    talk.send_line(&method.message().to_string())
        .map_err(ReadError::Host)
}

/// The `result` of the answer to the request `id`.
///
/// Lines that are not that answer are skipped: notices, the program's own
/// requests (they have a `method`), answers to other ids and anything that
/// is not JSON, such as what the user's shell prints when it starts.
fn answer(talk: &Talk, id: u64, until_ms: i64) -> Result<Value, ReadError> {
    loop {
        let left_ms = until_ms.saturating_sub(clock::now_ms()).max(1);
        let timeout_ms = u32::try_from(left_ms).unwrap_or(u32::MAX);

        let line = match talk.next_line(timeout_ms) {
            Ok(Some(line)) => line,
            Ok(None) => {
                return Err(ReadError::Failed(
                    "codex app-server ended before it answered".to_string(),
                ));
            },
            Err(HostError::TimedOut(_)) => {
                return Err(ReadError::Host(HostError::TimedOut(format!(
                    "codex app-server gave no answer in {} s",
                    TIME_LIMIT_MS / 1000
                ))));
            },
            Err(error) => return Err(ReadError::Host(error)),
        };

        let Ok(message) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let is_the_answer = message.get("id").and_then(Value::as_u64)
            == Some(id)
            && message.get("method").is_none();
        if !is_the_answer {
            continue;
        }

        if message.get("error").is_some() {
            return Err(ReadError::Failed(
                "codex app-server refused the request".to_string(),
            ));
        }
        return Ok(message.get("result").cloned().unwrap_or(Value::Null));
    }
}

/// The limits in the `result` of `account/rateLimits/read`: its
/// `rateLimits` has the `primary` and `secondary` windows, the `credits`
/// and the `planType`. Every field may be `null`.
fn limits_from(result: &Value) -> Result<Limits, ReadError> {
    let Some(rate_limits) = result.get("rateLimits").filter(|v| v.is_object())
    else {
        return Err(ReadError::Unexpected(
            "the answer has no rateLimits".to_string(),
        ));
    };

    let windows = ["primary", "secondary"]
        .iter()
        .filter_map(|field| window(rate_limits.get(*field)?))
        .collect();
    let credits = rate_limits.get("credits");
    let credits = codex_credits(
        credits.and_then(|credits| credits.get("unlimited")),
        credits.and_then(|credits| credits.get("hasCredits")),
        credits.and_then(|credits| credits.get("balance")),
        individual_limit_in(rate_limits),
    );
    let plan_type = rate_limits.get("planType").and_then(Value::as_str);

    codex_limits(windows, credits, plan_type)
}

/// One window: `usedPercent` from 0 to 100, `windowDurationMins` its length
/// in minutes and `resetsAt` a time in seconds since 1970.
fn window(window: &Value) -> Option<WindowNumbers> {
    let percent_used = window.get("usedPercent")?.as_f64()?;
    let minutes = window
        .get("windowDurationMins")
        .and_then(Value::as_u64)
        .and_then(|minutes| u32::try_from(minutes).ok());
    let resets_at_ms = window.get("resetsAt").and_then(millis_of_seconds);

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
    use crate::rate_limits::no_numbers;
    use crate::testing::{as_provider, as_provider_in, host, path};

    /// The only methods the plugin may ever send to `codex app-server`.
    const ALLOWED_METHODS: [&str; 3] =
        ["initialize", "initialized", "account/rateLimits/read"];

    const INITIALIZE_ANSWER: &str = r#"{"id":1,"result":{"userAgent":"codex_cli_rs/0.169.0 (Linux; x86_64) otty/0.1.0"}}"#;
    const LIMITS_ANSWER: &str = r#"{"id":2,"result":{"rateLimits":{"limitId":"codex","limitName":null,"primary":{"usedPercent":42,"windowDurationMins":300,"resetsAt":1794826800},"secondary":{"usedPercent":17,"windowDurationMins":10080,"resetsAt":1794909600},"credits":{"hasCredits":true,"unlimited":false,"balance":"12.5"},"planType":"plus"},"rateLimitsByLimitId":{"codex":{"limitId":"codex","primary":null}}}}"#;

    /// 2026-11-16T10:00:00Z.
    const NOW: i64 = 1_794_823_200_000;

    /// What one read did: the messages it sent and what it returned.
    struct Asked {
        sent: Vec<Value>,
        result: Result<Limits, ReadError>,
    }

    impl Asked {
        /// The method of each message sent, in order.
        fn methods(&self) -> Vec<&str> {
            self.sent
                .iter()
                .filter_map(|message| message.get("method")?.as_str())
                .collect()
        }
    }

    /// Reads the limits from an app server that writes `lines`, then
    /// ends. Every message sent must be one JSON object of an allowed
    /// method.
    fn ask(lines: &[&str]) -> Asked {
        let host = host();
        host.answer_talk(PROGRAM, &APP_SERVER_ARGS, lines);

        let result = as_provider(&host, || read_limits(&[]));

        asked(&host, result)
    }

    fn asked(host: &TestHost, result: Result<Limits, ReadError>) -> Asked {
        let sent: Vec<Value> = host
            .sent()
            .iter()
            .map(|line| serde_json::from_str(line).expect("one JSON a line"))
            .collect();
        let asked = Asked { sent, result };
        for method in asked.methods() {
            assert!(ALLOWED_METHODS.contains(&method), "sent {method}");
        }

        asked
    }

    fn windows() -> Vec<LimitWindow> {
        vec![
            LimitWindow {
                length_minutes: Some(300),
                model: None,
                percent_used: 42.0,
                resets_at_ms: Some(NOW + 3_600_000),
                reset_text: None,
            },
            LimitWindow {
                length_minutes: Some(10_080),
                model: None,
                percent_used: 17.0,
                resets_at_ms: Some(NOW + 86_400_000),
                reset_text: None,
            },
        ]
    }

    fn credits_of(lines: &[&str]) -> Option<Credits> {
        ask(lines).result.expect("the limits are read").credits
    }

    #[test]
    fn the_answer_gives_both_windows_and_the_plan() {
        let asked = ask(&[INITIALIZE_ANSWER, LIMITS_ANSWER]);

        assert_eq!(
            asked.result,
            Ok(Limits {
                windows: windows(),
                credits: None,
                plan: Some("Plus".to_string()),
                recorded_at_ms: None,
            })
        );
    }

    #[test]
    fn it_starts_codex_app_server_read_only_with_the_variables_given() {
        let host = host();
        host.answer_talk(
            PROGRAM,
            &APP_SERVER_ARGS,
            &[INITIALIZE_ANSWER, LIMITS_ANSWER],
        );
        let env = [("CODEX_HOME".to_string(), path(&host, ".codex-work"))];

        let result = as_provider_in(&host, ".codex-work", || read_limits(&env));

        assert!(result.is_ok(), "{result:?}");
        let started = host.started();
        assert_eq!(started.len(), 1);
        assert_eq!(started[0].program(), "codex");
        assert_eq!(started[0].args(), APP_SERVER_ARGS);
        assert_eq!(started[0].env(), env);
    }

    #[test]
    fn only_the_handshake_and_the_limits_request_are_sent() {
        let asked = ask(&[INITIALIZE_ANSWER, LIMITS_ANSWER]);

        assert_eq!(
            asked.sent,
            [
                json!({
                    "id": 1,
                    "method": "initialize",
                    "params": {"clientInfo": {
                        "name": "otty",
                        "title": "OTTY",
                        "version": env!("CARGO_PKG_VERSION"),
                    }},
                }),
                json!({"method": "initialized"}),
                json!({"id": 2, "method": "account/rateLimits/read"}),
            ]
        );
    }

    /// The issue's method list: every method the plugin can send is one
    /// of the three, whatever the program answers.
    #[test]
    fn no_method_but_the_three_is_ever_written() {
        let methods = [
            Method::Initialize,
            Method::Initialized,
            Method::ReadRateLimits,
        ];

        for method in methods {
            let message = method.message();
            let name = message.get("method").and_then(Value::as_str);
            assert!(
                name.is_some_and(|name| ALLOWED_METHODS.contains(&name)),
                "{message}"
            );
        }
        for lines in [
            &[][..],
            &[INITIALIZE_ANSWER][..],
            &[INITIALIZE_ANSWER, LIMITS_ANSWER][..],
            &[r#"{"id":1,"error":{"message":"no"}}"#][..],
        ] {
            ask(lines);
        }
    }

    #[test]
    fn notices_requests_and_other_lines_between_the_answers_are_skipped() {
        let asked = ask(&[
            "Welcome to the user's shell",
            r#"{"method":"account/rateLimits/updated","params":{"rateLimits":{}}}"#,
            INITIALIZE_ANSWER,
            r#"{"id":2,"method":"item/tool/requestUserInput","params":{}}"#,
            r#"{"id":7,"result":{"rateLimits":{}}}"#,
            "{\"id\":2,",
            "[2]",
            LIMITS_ANSWER,
        ]);

        let limits = asked.result.expect("the limits are read");
        assert_eq!(limits.plan.as_deref(), Some("Plus"));
    }

    #[test]
    fn an_answer_with_credits_and_no_windows_is_credits() {
        let only_credits = r#"{"id":2,"result":{"rateLimits":{"primary":null,"secondary":null,"credits":{"hasCredits":true,"unlimited":false,"balance":3760},"planType":"enterprise"}}}"#;

        let asked = ask(&[INITIALIZE_ANSWER, only_credits]);

        assert_eq!(
            asked.result,
            Ok(Limits {
                windows: Vec::new(),
                credits: Some(Credits {
                    left: Some(3760.0),
                    ..no_numbers()
                }),
                plan: Some("Enterprise".to_string()),
                recorded_at_ms: None,
            })
        );
    }

    #[test]
    fn the_three_shapes_of_credits_with_no_windows() {
        let available = r#"{"id":2,"result":{"rateLimits":{"primary":null,"secondary":null,"credits":{"hasCredits":true,"unlimited":false,"balance":null},"planType":"enterprise_cbp_usage_based"}}}"#;
        let unlimited = r#"{"id":2,"result":{"rateLimits":{"primary":null,"secondary":null,"credits":{"hasCredits":true,"unlimited":true},"planType":"enterprise"}}}"#;
        let no_credits = r#"{"id":2,"result":{"rateLimits":{"primary":null,"secondary":null,"credits":{"hasCredits":false,"balance":"0"},"planType":"prolite"}}}"#;

        assert_eq!(
            credits_of(&[INITIALIZE_ANSWER, available]),
            Some(no_numbers())
        );
        assert_eq!(
            credits_of(&[INITIALIZE_ANSWER, unlimited]),
            Some(Credits {
                is_unlimited: true,
                ..no_numbers()
            })
        );
        assert!(matches!(
            ask(&[INITIALIZE_ANSWER, no_credits]).result,
            Err(ReadError::Unexpected(_))
        ));
    }

    #[test]
    fn the_individual_limit_of_a_workspace_is_a_pool_of_credits() {
        let workspace = r#"{"id":2,"result":{"rateLimits":{"primary":null,"secondary":null,"credits":{"hasCredits":true,"unlimited":false,"balance":null},"individualLimit":{"limit":"100000","used":7761,"remainingPercent":92.239,"resetsAt":1797465600},"planType":"enterprise_cbp_usage_based"}}}"#;

        assert_eq!(
            credits_of(&[INITIALIZE_ANSWER, workspace]),
            Some(Credits {
                left: Some(92_239.0),
                used: Some(7761.0),
                pool: Some(100_000.0),
                renews_at_ms: Some(1_797_465_600_000),
                is_unlimited: false,
            })
        );
    }

    #[test]
    fn no_credits_next_to_a_window_is_the_window() {
        let prolite = LIMITS_ANSWER
            .replace(
                r#""hasCredits":true,"unlimited":false,"balance":"12.5""#,
                r#""hasCredits":false,"balance":"0""#,
            )
            .replace(r#""planType":"plus""#, r#""planType":"prolite""#);

        let limits = ask(&[INITIALIZE_ANSWER, &prolite])
            .result
            .expect("the limits are read");

        assert_eq!(limits.windows, windows());
        assert_eq!(limits.plan.as_deref(), Some("Pro 100"));
    }

    #[test]
    fn an_answer_with_no_limits_is_unexpected() {
        for answer in [
            r#"{"id":2,"result":{"rateLimits":{"primary":null,"secondary":null,"credits":null,"planType":null}}}"#,
            r#"{"id":2,"result":{"rateLimitsByLimitId":{}}}"#,
            r#"{"id":2}"#,
        ] {
            let asked = ask(&[INITIALIZE_ANSWER, answer]);

            assert!(
                matches!(asked.result, Err(ReadError::Unexpected(_))),
                "{answer}"
            );
        }
    }

    /// The program's message is not shown: it could name the account.
    #[test]
    fn an_error_answer_fails() {
        let refused = r#"{"id":2,"error":{"code":-32600,"message":"codex account authentication required to read rate limits"}}"#;

        let asked = ask(&[INITIALIZE_ANSWER, refused]);

        assert_eq!(
            asked.result,
            Err(ReadError::Failed(
                "codex app-server refused the request".to_string()
            ))
        );
    }

    #[test]
    fn an_error_answer_to_initialize_sends_nothing_more() {
        let refused = r#"{"id":1,"error":{"code":-32600,"message":"Already initialized"}}"#;

        let asked = ask(&[refused]);

        assert!(matches!(asked.result, Err(ReadError::Failed(_))));
        assert_eq!(asked.methods(), ["initialize"]);
    }

    #[test]
    fn a_program_that_ends_before_it_answers_fails() {
        let asked = ask(&[INITIALIZE_ANSWER]);

        assert_eq!(
            asked.result,
            Err(ReadError::Failed(
                "codex app-server ended before it answered".to_string()
            ))
        );
        assert_eq!(
            asked.methods(),
            ["initialize", "initialized", "account/rateLimits/read"]
        );
    }

    #[test]
    fn a_program_that_can_not_start_fails_with_the_reason_of_the_host() {
        let host = host();

        let result = as_provider(&host, || read_limits(&[]));

        assert!(matches!(result, Err(ReadError::Host(_))), "{result:?}");
        assert!(host.sent().is_empty());
    }

    /// Spec 2026-11-23-accounts-apart-from-profiles, Decision 5:
    /// `CODEX_HOME` reaches `codex app-server` only set to a login folder
    /// of the call.
    #[test]
    fn the_config_folder_of_another_profile_is_refused() {
        let host = host();
        host.answer_talk(PROGRAM, &APP_SERVER_ARGS, &[INITIALIZE_ANSWER]);
        let env = [("CODEX_HOME".to_string(), path(&host, ".codex-team"))];

        let result = as_provider_in(&host, ".codex-work", || read_limits(&env));

        assert!(
            matches!(result, Err(ReadError::Host(HostError::NotAllowed(_)))),
            "{result:?}"
        );
        assert!(host.started().is_empty());
    }

    #[test]
    fn a_variable_the_part_does_not_list_is_refused() {
        let host = host();
        host.answer_talk(PROGRAM, &APP_SERVER_ARGS, &[INITIALIZE_ANSWER]);
        let env = [("HOME".to_string(), "/elsewhere".to_string())];

        let result = as_provider(&host, || read_limits(&env));

        assert!(
            matches!(result, Err(ReadError::Host(HostError::NotAllowed(_)))),
            "{result:?}"
        );
    }
}
