//! The second source of Codex limits: `codex app-server`, asked for the
//! limits of its login.
//!
//! The source starts `codex -s read-only -a never app-server` with the
//! profile's config folder and talks JSON-RPC with it over its input and
//! output, one JSON object on a line. Codex leaves out the
//! `"jsonrpc": "2.0"` field of JSON-RPC 2.0, so OTTY does too.
//!
//! Only `initialize`, the `initialized` notice and `account/rateLimits/read`
//! are ever sent. `thread/list` is never sent: Codex repairs its own index of
//! sessions while it answers it, and on the owner's computer it deleted
//! rows of that index (spec Decision 5).

use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use super::agent::start_command;
use super::rate_limits::{
    WindowNumbers, codex_credits, codex_limits, time_from_seconds,
};
use crate::command::AgentCommand;
use crate::error::SourceError;
use crate::limits::{LimitSource, Limits};
use crate::profile::Profile;
use crate::program::{ProgramOutput, RunningProgram, StartProgram};

/// How long the whole talk may take, from the start of the program to the
/// answer. The user's shell reads its start files first, which can take a
/// few seconds with a version manager.
const TIME_LIMIT: Duration = Duration::from_secs(10);
/// The id of the `initialize` request.
const INITIALIZE_ID: u64 = 1;
/// The id of the `account/rateLimits/read` request.
const RATE_LIMITS_ID: u64 = 2;

/// Asks `codex app-server` of the profile for its limits.
pub(super) struct AppServer {
    programs: Arc<dyn StartProgram>,
}

impl AppServer {
    /// The source that starts `codex app-server` with `programs`.
    pub(super) fn new(programs: Arc<dyn StartProgram>) -> Self {
        Self { programs }
    }
}

impl LimitSource for AppServer {
    fn name(&self) -> &'static str {
        "app-server"
    }

    /// A program that can not start, ends, refuses or does not answer in
    /// time is "not available". The program is stopped in every case.
    fn limits(&self, profile: &Profile) -> Result<Limits, SourceError> {
        let command = app_server_command(profile);

        let mut program = self.programs.start(&command).map_err(|reason| {
            let name = command.program();
            SourceError::NotAvailable(format!("can not start {name}: {reason}"))
        })?;
        let until = Instant::now() + TIME_LIMIT;
        let answer = rate_limits_answer(program.as_mut(), until);
        program.stop();

        limits_from(&answer?)
    }
}

/// `codex -s read-only -a never app-server` of `profile`: no command it
/// might run can write, and it never asks for an approval.
fn app_server_command(profile: &Profile) -> AgentCommand {
    start_command(profile)
        .with_arg("-s")
        .with_arg("read-only")
        .with_arg("-a")
        .with_arg("never")
        .with_arg("app-server")
}

/// The `result` of `account/rateLimits/read`, after the handshake.
fn rate_limits_answer(
    program: &mut dyn RunningProgram,
    until: Instant,
) -> Result<Value, SourceError> {
    let client_info = json!({
        "name": "otty",
        "title": "OTTY",
        "version": env!("CARGO_PKG_VERSION"),
    });

    send(
        program,
        &json!({
            "id": INITIALIZE_ID,
            "method": "initialize",
            "params": {"clientInfo": client_info},
        }),
    )?;
    answer(program, INITIALIZE_ID, until)?;

    send(program, &json!({"method": "initialized"}))?;
    send(
        program,
        &json!({"id": RATE_LIMITS_ID, "method": "account/rateLimits/read"}),
    )?;

    answer(program, RATE_LIMITS_ID, until)
}

/// Writes `message` to the program as one line.
fn send(
    program: &mut dyn RunningProgram,
    message: &Value,
) -> Result<(), SourceError> {
    program.send_line(&message.to_string()).map_err(|reason| {
        SourceError::NotAvailable(format!(
            "can not write to codex app-server: {reason}"
        ))
    })
}

/// The `result` of the answer to the request `id`.
///
/// Lines that are not that answer are skipped: notices, the program's own
/// requests (they have a `method`), answers to other ids and anything that
/// is not JSON, such as what the user's shell prints when it starts.
fn answer(
    program: &mut dyn RunningProgram,
    id: u64,
    until: Instant,
) -> Result<Value, SourceError> {
    loop {
        let line = match program.next_line(until) {
            ProgramOutput::Line(line) => line,
            ProgramOutput::TimedOut => {
                let seconds = TIME_LIMIT.as_secs();
                return Err(SourceError::NotAvailable(format!(
                    "codex app-server did not answer in {seconds} seconds"
                )));
            },
            _ => {
                return Err(SourceError::NotAvailable(String::from(
                    "codex app-server ended before it answered",
                )));
            },
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

        if let Some(error) = message.get("error") {
            let reason = error
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("no reason given");
            return Err(SourceError::NotAvailable(format!(
                "codex app-server refused: {reason}"
            )));
        }
        return Ok(message.get("result").cloned().unwrap_or(Value::Null));
    }
}

/// The limits in the `result` of `account/rateLimits/read`: its
/// `rateLimits` has the `primary` and `secondary` windows, the `credits`
/// and the `planType`. Every field may be `null`.
fn limits_from(result: &Value) -> Result<Limits, SourceError> {
    let Some(rate_limits) = result.get("rateLimits").filter(|v| v.is_object())
    else {
        return Err(SourceError::UnexpectedAnswer(String::from(
            "the answer has no rateLimits",
        )));
    };

    let windows = ["primary", "secondary"]
        .iter()
        .filter_map(|field| window(rate_limits.get(*field)?))
        .collect();
    let credits = rate_limits.get("credits").and_then(|credits| {
        codex_credits(
            credits.get("unlimited"),
            credits.get("hasCredits"),
            credits.get("balance"),
        )
    });
    let plan_type = rate_limits.get("planType").and_then(Value::as_str);

    codex_limits(windows, credits, plan_type)
}

/// One window: `usedPercent` from 0 to 100, `windowDurationMins` its length
/// in minutes and `resetsAt` a time in seconds since 1970.
fn window(window: &Value) -> Option<WindowNumbers> {
    let percent_used = window.get("usedPercent")?.as_f64()?;
    let seconds = window
        .get("windowDurationMins")
        .and_then(Value::as_u64)
        .and_then(|minutes| minutes.checked_mul(60));
    let resets_at = window
        .get("resetsAt")
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
    use std::collections::VecDeque;
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    use serde_json::{Value, json};

    use super::*;
    use crate::billing::{Billing, Credits, LimitWindow, UsageWindows};
    use crate::command::AgentCommand;
    use crate::program::{MockRunningProgram, MockStartProgram, ProgramOutput};
    use crate::testing::{at, codex_profile};

    /// 2026-11-16T10:00:00Z.
    const NOW: u64 = 1_794_823_200;
    /// The only methods OTTY may ever send to `codex app-server`.
    const ALLOWED_METHODS: [&str; 3] =
        ["initialize", "initialized", "account/rateLimits/read"];

    const INITIALIZE_ANSWER: &str = r#"{"id":1,"result":{"userAgent":"codex_cli_rs/0.163.0 (Linux; x86_64) otty/0.1.0"}}"#;
    const LIMITS_ANSWER: &str = r#"{"id":2,"result":{
        "rateLimits":{
            "limitId":"codex","limitName":null,
            "primary":{"usedPercent":42,"windowDurationMins":300,"resetsAt":1794826800},
            "secondary":{"usedPercent":17,"windowDurationMins":10080,"resetsAt":1794909600},
            "credits":{"hasCredits":true,"unlimited":false,"balance":"12.5"},
            "planType":"plus"
        },
        "rateLimitsByLimitId":{"codex":{"limitId":"codex","primary":null}}
    }}"#;

    /// What one ask of the source did: the command it started, the
    /// messages it sent and what it returned.
    struct Asked {
        command: Option<AgentCommand>,
        sent: Vec<Value>,
        result: Result<Limits, SourceError>,
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

    /// Asks the source of `profile` with a program that writes `outputs`
    /// one after the other, then nothing more until the time limit. The
    /// program must be stopped exactly once.
    fn ask(profile: &Profile, outputs: Vec<ProgramOutput>) -> Asked {
        let sent = Arc::new(Mutex::new(Vec::new()));
        let started = Arc::new(Mutex::new(None));
        let mut outputs = VecDeque::from(outputs);

        let mut program = MockRunningProgram::new();
        let sent_lines = Arc::clone(&sent);
        program.expect_send_line().returning(move |line| {
            sent_lines
                .lock()
                .expect("the sent lines")
                .push(line.to_owned());
            Ok(())
        });
        program.expect_next_line().returning(move |_| {
            outputs.pop_front().unwrap_or(ProgramOutput::TimedOut)
        });
        program.expect_stop().times(1).return_const(());
        let mut programs = MockStartProgram::new();
        let commands = Arc::clone(&started);
        programs
            .expect_start()
            .times(1)
            .return_once(move |command| {
                *commands.lock().expect("the command") = Some(command.clone());
                Ok(Box::new(program))
            });

        let result = AppServer::new(Arc::new(programs)).limits(profile);

        let sent = sent
            .lock()
            .expect("the sent lines")
            .iter()
            .map(|line| serde_json::from_str(line).expect("one JSON a line"))
            .collect();
        let command = started.lock().expect("the command").take();
        let asked = Asked {
            command,
            sent,
            result,
        };
        for method in asked.methods() {
            assert!(ALLOWED_METHODS.contains(&method), "sent {method}");
        }
        asked
    }

    fn profile() -> Profile {
        codex_profile(Path::new("/home/user/.codex-work"))
    }

    fn lines(lines: &[&str]) -> Vec<ProgramOutput> {
        lines
            .iter()
            .map(|line| ProgramOutput::Line(line.replace('\n', "")))
            .collect()
    }

    fn not_available(text: &str) -> Result<Limits, SourceError> {
        Err(SourceError::NotAvailable(String::from(text)))
    }

    #[test]
    fn the_answer_gives_both_windows_and_the_plan() {
        let asked = ask(&profile(), lines(&[INITIALIZE_ANSWER, LIMITS_ANSWER]));

        assert_eq!(
            asked.result,
            Ok(Limits::new(
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
            ))
        );
    }

    #[test]
    fn it_starts_codex_app_server_read_only_with_the_config_folder() {
        let asked = ask(&profile(), lines(&[INITIALIZE_ANSWER, LIMITS_ANSWER]));

        assert_eq!(
            asked.command,
            Some(
                AgentCommand::new("codex")
                    .with_arg("-s")
                    .with_arg("read-only")
                    .with_arg("-a")
                    .with_arg("never")
                    .with_arg("app-server")
                    .with_env("CODEX_HOME", "/home/user/.codex-work")
            )
        );
    }

    #[test]
    fn the_program_path_of_the_profile_is_started() {
        let profile =
            profile().with_program(PathBuf::from("/opt/codex/bin/codex"));

        let asked = ask(&profile, lines(&[INITIALIZE_ANSWER, LIMITS_ANSWER]));

        let command = asked.command.expect("a program is started");
        assert_eq!(command.program(), "/opt/codex/bin/codex");
    }

    #[test]
    fn only_the_handshake_and_the_limits_request_are_sent() {
        let asked = ask(&profile(), lines(&[INITIALIZE_ANSWER, LIMITS_ANSWER]));

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

    #[test]
    fn notices_requests_and_other_lines_between_the_answers_are_skipped() {
        let asked = ask(
            &profile(),
            lines(&[
                "Welcome to the user's shell",
                r#"{"method":"account/rateLimits/updated","params":{"rateLimits":{}}}"#,
                INITIALIZE_ANSWER,
                r#"{"id":2,"method":"item/tool/requestUserInput","params":{}}"#,
                r#"{"id":7,"result":{"rateLimits":{}}}"#,
                "{\"id\":2,",
                "[2]",
                LIMITS_ANSWER,
            ]),
        );

        let limits = asked.result.expect("the limits are read");
        assert_eq!(limits.plan(), Some("Plus"));
    }

    #[test]
    fn an_answer_with_credits_and_no_windows_is_credits() {
        let only_credits = r#"{"id":2,"result":{"rateLimits":{
            "primary":null,"secondary":null,
            "credits":{"hasCredits":true,"unlimited":false,"balance":3760},
            "planType":"enterprise"
        }}}"#;

        let asked = ask(&profile(), lines(&[INITIALIZE_ANSWER, only_credits]));

        assert_eq!(
            asked.result,
            Ok(Limits::new(
                Billing::Credits(Credits::new(Some(3760.0), None, None, None)),
                Some(String::from("Enterprise")),
            ))
        );
    }

    #[test]
    fn the_three_shapes_of_credits_with_no_windows() {
        let available = r#"{"id":2,"result":{"rateLimits":{
            "primary":null,"secondary":null,
            "credits":{"hasCredits":true,"unlimited":false,"balance":null},
            "planType":"enterprise_cbp_usage_based"
        }}}"#;
        let unlimited = r#"{"id":2,"result":{"rateLimits":{
            "primary":null,"secondary":null,
            "credits":{"hasCredits":true,"unlimited":true},
            "planType":"enterprise"
        }}}"#;
        let no_credits = r#"{"id":2,"result":{"rateLimits":{
            "primary":null,"secondary":null,
            "credits":{"hasCredits":false,"balance":"0"},
            "planType":"prolite"
        }}}"#;

        let available = ask(&profile(), lines(&[INITIALIZE_ANSWER, available]));
        let unlimited = ask(&profile(), lines(&[INITIALIZE_ANSWER, unlimited]));
        let no_credits =
            ask(&profile(), lines(&[INITIALIZE_ANSWER, no_credits]));

        assert_eq!(
            available.result,
            Ok(Limits::new(
                Billing::Credits(Credits::available()),
                Some(String::from("Enterprise, usage-based")),
            ))
        );
        assert_eq!(
            unlimited.result,
            Ok(Limits::new(
                Billing::Credits(Credits::unlimited()),
                Some(String::from("Enterprise")),
            ))
        );
        assert_eq!(
            no_credits.result,
            Err(SourceError::UnexpectedAnswer(String::from(
                "no usage window and no credits in the answer"
            )))
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

        let asked = ask(&profile(), lines(&[INITIALIZE_ANSWER, &prolite]));

        let limits = asked.result.expect("the limits are read");
        assert!(matches!(limits.billing(), Billing::UsageWindows(_)));
        assert_eq!(limits.plan(), Some("Pro Lite"));
    }

    #[test]
    fn an_answer_with_no_limits_is_unexpected() {
        let no_numbers = r#"{"id":2,"result":{"rateLimits":{"primary":null,"secondary":null,"credits":null,"planType":null}}}"#;
        let no_rate_limits = r#"{"id":2,"result":{"rateLimitsByLimitId":{}}}"#;
        let no_result = r#"{"id":2}"#;

        let no_numbers =
            ask(&profile(), lines(&[INITIALIZE_ANSWER, no_numbers]));
        let no_rate_limits =
            ask(&profile(), lines(&[INITIALIZE_ANSWER, no_rate_limits]));
        let no_result = ask(&profile(), lines(&[INITIALIZE_ANSWER, no_result]));

        assert_eq!(
            no_numbers.result,
            Err(SourceError::UnexpectedAnswer(String::from(
                "no usage window and no credits in the answer"
            )))
        );
        let no_rate_limits_error = Err(SourceError::UnexpectedAnswer(
            String::from("the answer has no rateLimits"),
        ));
        assert_eq!(no_rate_limits.result, no_rate_limits_error);
        assert_eq!(no_result.result, no_rate_limits_error);
    }

    #[test]
    fn an_error_answer_fails_with_its_message() {
        let refused = r#"{"id":2,"error":{"code":-32600,"message":"codex account authentication required to read rate limits"}}"#;

        let asked = ask(&profile(), lines(&[INITIALIZE_ANSWER, refused]));

        assert_eq!(
            asked.result,
            not_available(
                "codex app-server refused: codex account authentication required to read rate limits"
            )
        );
    }

    #[test]
    fn an_error_answer_to_initialize_sends_nothing_more() {
        let refused = r#"{"id":1,"error":{"code":-32600,"message":"Already initialized"}}"#;

        let asked = ask(&profile(), lines(&[refused]));

        assert_eq!(
            asked.result,
            not_available("codex app-server refused: Already initialized")
        );
        assert_eq!(asked.methods(), ["initialize"]);
    }

    #[test]
    fn no_answer_in_time_stops_the_program() {
        let asked = ask(&profile(), Vec::new());

        assert_eq!(
            asked.result,
            not_available("codex app-server did not answer in 10 seconds")
        );
        assert_eq!(asked.methods(), ["initialize"]);
    }

    #[test]
    fn the_time_limit_covers_the_whole_talk() {
        let mut program = MockRunningProgram::new();
        program.expect_send_line().returning(|_| Ok(()));
        let mut limits = Vec::new();
        program.expect_next_line().times(2).returning(move |until| {
            limits.push(until);
            if limits.len() == 1 {
                return ProgramOutput::Line(String::from(INITIALIZE_ANSWER));
            }
            assert_eq!(limits[0], limits[1], "one limit for the whole talk");
            ProgramOutput::TimedOut
        });
        program.expect_stop().times(1).return_const(());
        let mut programs = MockStartProgram::new();
        programs
            .expect_start()
            .return_once(move |_| Ok(Box::new(program)));
        let started = Instant::now();

        let result = AppServer::new(Arc::new(programs)).limits(&profile());

        assert!(result.is_err());
        assert!(started.elapsed() < Duration::from_secs(1), "no real wait");
    }

    #[test]
    fn a_program_that_ends_before_it_answers_is_not_available() {
        let asked = ask(
            &profile(),
            vec![
                ProgramOutput::Line(String::from(INITIALIZE_ANSWER)),
                ProgramOutput::Ended,
            ],
        );

        assert_eq!(
            asked.result,
            not_available("codex app-server ended before it answered")
        );
    }

    #[test]
    fn a_program_that_takes_no_input_is_stopped() {
        let mut program = MockRunningProgram::new();
        program
            .expect_send_line()
            .times(1)
            .returning(|_| Err(String::from("Broken pipe")));
        program.expect_next_line().never();
        program.expect_stop().times(1).return_const(());
        let mut programs = MockStartProgram::new();
        programs
            .expect_start()
            .return_once(move |_| Ok(Box::new(program)));

        let result = AppServer::new(Arc::new(programs)).limits(&profile());

        assert_eq!(
            result,
            not_available("can not write to codex app-server: Broken pipe")
        );
    }

    #[test]
    fn a_program_that_is_not_found_is_not_available() {
        let mut programs = MockStartProgram::new();
        programs
            .expect_start()
            .times(1)
            .return_once(|_| Err(String::from("codex is not found")));

        let result = AppServer::new(Arc::new(programs)).limits(&profile());

        assert_eq!(
            result,
            not_available("can not start codex: codex is not found")
        );
    }
}
