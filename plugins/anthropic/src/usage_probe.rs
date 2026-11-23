//! The limits Claude Code itself shows: `claude` in a hidden terminal,
//! asked for its `/usage` panel.
//!
//! The run follows CodexBar's Claude CLI probe (MIT licence),
//! <https://github.com/steipete/CodexBar/blob/main/Sources/CodexBarCore/Providers/Claude/ClaudeCLISession.swift>
//! and the "CLI PTY" part of its `docs/claude.md`: `claude` starts with no
//! tools, no MCP servers, no hooks and no remote control, in OTTY's
//! working folder of hidden terminals and a screen of 160 by 200, so the
//! panel is never cut. Once its screen is still, `/usage` and Enter are
//! typed; the panel is read from the screen and closed with Escape, and
//! OTTY stops the program when the terminal is dropped.
//!
//! Claude Code asks whether to trust a folder the first time it runs
//! there. As CodexBar does, the probe answers "Yes" for its own folder, and
//! only for it (`trust_question.rs`), then reads the panel as usual. Claude
//! Code keeps that answer in the profile's `.claude.json`, as the trust of
//! the probe's folder. Claude Code may also renew its own login token
//! while it runs; the plugin never sees the token.
//!
//! Any other question, and a login, ends the reading with no key typed; a
//! login waits for the user to run Claude Code, which renews it.
//! The session files the run left are removed in every case. OTTY runs
//! the reading at most once in five minutes per profile and gives the last
//! answer in between.
//!
//! The provider part `anthropic` reads the panel (issue 29): in a Claude
//! Code profile folder the account was found in, with Claude Code pointed
//! at that folder as the agent part `claude-code` starts it there, under
//! the provider part's own permissions.

use otty_plugin_sdk::agents::{LimitWindow, ReadError};
use otty_plugin_sdk::terminal::{self, HiddenTerminal};
use otty_plugin_sdk::{Command, HostError, clock};

use crate::needs;
use crate::probe_sessions::remove_probe_sessions;
use crate::trust_question::{TrustAnswer, answer_trust};
use crate::usage_screen::{UsageScreen, read_usage_screen};

/// The width of the hidden screen, in characters.
const COLUMNS: u16 = 160;
/// The height of the hidden screen, in lines: the panel can be a hundred
/// lines long.
const ROWS: u16 = 200;
/// The keys that ask for the panel: the command and Enter.
const USAGE_KEYS: &str = "/usage\r";
/// The Enter key.
const ENTER: &str = "\r";
/// The Escape key, which closes the panel.
const ESCAPE: &str = "\u{1b}";
/// The arrow key up, which moves the marker of a list of options.
const ARROW_UP: &str = "\u{1b}[A";
/// The arrow key down.
const ARROW_DOWN: &str = "\u{1b}[B";
/// How many times Enter may be typed again.
const ENTER_AGAIN_TIMES: usize = 3;
/// The settings that turn off remote control and every hook for this run
/// only, as CodexBar does.
const PROBE_SETTINGS: &str =
    r#"{"remoteControlAtStartup":false,"disableAllHooks":true}"#;
/// The variable that keeps Claude Code from updating itself in the run.
const NO_UPDATES: (&str, &str) = ("DISABLE_AUTOUPDATER", "1");
/// The times of a real reading.
const TIMING: Timing = Timing {
    // The user's shell reads its start files first, Claude Code takes a
    // few seconds to start, and the panel asks Anthropic's server.
    limit_ms: 30_000,
    // As CodexBar: the screen must show something for a while before
    // `/usage` is typed, so Claude Code is ready for keys.
    ready_after_ms: 2_000,
    // The first Enter may only pick the command in Claude Code's list of
    // commands.
    enter_again_after_ms: 3_000,
    look_ms: 250,
};

/// What the probe does with the hidden terminal OTTY started. It is a
/// trait so the tests play a made-up screen with `mockall`; the plugin
/// gives the SDK's [`HiddenTerminal`].
#[cfg_attr(test, mockall::automock)]
pub(crate) trait ProbeTerminal {
    /// Types `keys`.
    fn send_keys(&self, keys: &str) -> Result<(), HostError>;

    /// Whether the screen changed within `timeout_ms`; an error once the
    /// program ended.
    fn wait_for_change(&self, timeout_ms: u32) -> Result<bool, HostError>;

    /// The visible text of the screen, the last one after the program
    /// ended.
    fn screen(&self) -> Result<String, HostError>;
}

impl ProbeTerminal for HiddenTerminal {
    fn send_keys(&self, keys: &str) -> Result<(), HostError> {
        HiddenTerminal::send_keys(self, keys)
    }

    fn wait_for_change(&self, timeout_ms: u32) -> Result<bool, HostError> {
        HiddenTerminal::wait_for_change(self, timeout_ms)
    }

    fn screen(&self) -> Result<String, HostError> {
        HiddenTerminal::screen(self)
    }
}

/// The times the reading waits for, in milliseconds, kept apart so the
/// tests wait less.
#[derive(Debug, Clone, Copy)]
struct Timing {
    limit_ms: i64,
    ready_after_ms: i64,
    enter_again_after_ms: i64,
    look_ms: u32,
}

/// The windows of the `/usage` panel of `claude`, the command that starts
/// Claude Code in the profile folder `folder` an account was found in.
/// The panel tells no plan: the part takes it from the login.
pub(crate) fn read_windows(
    folder: &str,
    claude: &Command,
) -> Result<Vec<LimitWindow>, ReadError> {
    let folders = terminal::working_folders().map_err(ReadError::Host)?;
    let command = probe_command(claude);

    // The terminal is dropped at the end of the block, which stops
    // `claude` before its files are removed.
    let answer = {
        let terminal = HiddenTerminal::start(
            &command.program,
            &command.args,
            &command.env,
            COLUMNS,
            ROWS,
        )
        .map_err(ReadError::Host)?;
        read_panel(&terminal, &folders, claude, TIMING)
    };

    remove_probe_sessions(folder, &folders);

    answer
}

/// `launch`, the command that starts Claude Code in a profile, as a
/// `claude` that can run nothing: no tools, no MCP servers of the user, no
/// hooks, no remote control and no update of itself.
fn probe_command(launch: &Command) -> Command {
    let args = [
        "--allowed-tools",
        "",
        "--strict-mcp-config",
        "--settings",
        PROBE_SETTINGS,
    ]
    .map(String::from)
    .to_vec();

    let mut env = launch.env.clone();
    let (name, value) = NO_UPDATES;
    env.push((name.to_string(), value.to_string()));

    Command {
        program: launch.program.clone(),
        args,
        env,
    }
}

/// The windows of the panel `terminal` shows, read in OTTY's working
/// folder `folders`; a read panel is closed with Escape. When Claude Code
/// asks to log in, the read waits for the user to run `renew`, Claude
/// Code in the profile's config folder.
fn read_panel(
    terminal: &impl ProbeTerminal,
    folders: &[String],
    renew: &Command,
    timing: Timing,
) -> Result<Vec<LimitWindow>, ReadError> {
    let windows = wait_for_panel(terminal, folders, renew, timing)?;

    // The panel is read; closing it is only polite, so a key that can not
    // be typed changes nothing.
    let _ = terminal.send_keys(ESCAPE);

    Ok(windows)
}

/// Waits for the screen of `terminal` to show the panel and reads it.
///
/// The question whether to trust the folder is answered for the probe's
/// own folder, one key for each new screen of the question. `/usage` is
/// typed once the screen has shown something else for a while and stays
/// still. While no panel shows, Enter is typed again now and then. A login
/// or another question ends the wait at once, before any key is typed; a
/// login waits for the user to run `renew`.
fn wait_for_panel(
    terminal: &impl ProbeTerminal,
    folders: &[String],
    renew: &Command,
    timing: Timing,
) -> Result<Vec<LimitWindow>, ReadError> {
    let deadline = clock::now_ms() + timing.limit_ms;
    let mut screen = String::new();
    let mut first_shown: Option<i64> = None;
    let mut asked_at: Option<i64> = None;
    let mut enters_left = ENTER_AGAIN_TIMES;

    loop {
        let look = look_ms(timing.look_ms, deadline - clock::now_ms());
        let (changed, has_ended) = match terminal.wait_for_change(look) {
            Ok(changed) => (changed, false),
            Err(HostError::Failed(_)) => (false, true),
            Err(error) => return Err(ReadError::Host(error)),
        };
        if changed || has_ended {
            screen = terminal.screen().map_err(ReadError::Host)?;
        }

        let reading = read_usage_screen(&screen);
        match reading {
            UsageScreen::Usage(windows) => return Ok(windows),
            UsageScreen::LoginAsked => {
                let need = needs::login_expired(renew.clone());
                return Err(ReadError::NeedsUser(need));
            },
            UsageScreen::Stopped(error) => return Err(error),
            _ => {},
        }
        if has_ended {
            return Err(ended(&screen));
        }

        let now = clock::now_ms();
        if now >= deadline {
            return Err(timed_out(&reading, timing.limit_ms));
        }

        if reading == UsageScreen::TrustQuestion {
            if changed {
                answer_trust_question(terminal, &screen, folders)?;
            }
            // Claude Code is ready for `/usage` only a while after the
            // question is gone.
            first_shown = None;
            continue;
        }

        if first_shown.is_none() && !screen.trim().is_empty() {
            first_shown = Some(now);
        }
        let is_ready = first_shown
            .is_some_and(|shown| now >= shown + timing.ready_after_ms);
        let panel_shows = reading == UsageScreen::Unreadable;

        match asked_at {
            None if is_ready && !changed => {
                type_keys(terminal, USAGE_KEYS)?;
                asked_at = Some(now);
            },
            Some(at)
                if !panel_shows
                    && enters_left > 0
                    && now >= at + timing.enter_again_after_ms =>
            {
                type_keys(terminal, ENTER)?;
                asked_at = Some(now);
                enters_left -= 1;
            },
            _ => {},
        }
    }
}

/// How long one look at the screen waits: `look_ms`, or the `left_ms`
/// before the deadline when that is shorter.
fn look_ms(look_ms: u32, left_ms: i64) -> u32 {
    let left = u32::try_from(left_ms.max(0)).unwrap_or(u32::MAX);

    look_ms.min(left)
}

/// Answers the question on `screen` whether to trust the folder: one key
/// toward "Yes" for the probe's own folder in `folders`, or none. A
/// question about another folder ends the reading.
fn answer_trust_question(
    terminal: &impl ProbeTerminal,
    screen: &str,
    folders: &[String],
) -> Result<(), ReadError> {
    match answer_trust(screen, folders) {
        TrustAnswer::Confirm => type_keys(terminal, ENTER),
        TrustAnswer::MoveUp => type_keys(terminal, ARROW_UP),
        TrustAnswer::MoveDown => type_keys(terminal, ARROW_DOWN),
        TrustAnswer::Refuse => Err(ReadError::Failed(
            "Claude Code asks whether to trust another folder".to_string(),
        )),
        TrustAnswer::Wait => Ok(()),
    }
}

/// Types `keys` into `terminal`.
fn type_keys(
    terminal: &impl ProbeTerminal,
    keys: &str,
) -> Result<(), ReadError> {
    terminal.send_keys(keys).map_err(ReadError::Host)
}

/// Why the program ended before the panel, from its last `screen`.
fn ended(screen: &str) -> ReadError {
    if screen.to_lowercase().contains("not found") {
        return ReadError::Failed("claude is not found".to_string());
    }

    ReadError::Failed("claude stopped before it showed its usage".to_string())
}

/// Why the time ran out, from the last `reading` of the screen: a panel
/// that showed but could not be read, a trust question that stayed after
/// its answer, or no panel at all.
fn timed_out(reading: &UsageScreen, limit_ms: i64) -> ReadError {
    match reading {
        UsageScreen::Unreadable => ReadError::Unexpected(
            "the /usage panel shows no percent used or left".to_string(),
        ),
        UsageScreen::TrustQuestion => ReadError::Failed(
            "Claude Code asks again whether to trust OTTY's folder".to_string(),
        ),
        _ => ReadError::Host(HostError::TimedOut(format!(
            "claude showed no usage in {} s",
            limit_ms / 1000
        ))),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use super::*;
    use crate::claude_code::claude_in_folder;
    use crate::testing::{as_provider_in, host, login_folder, path};

    const FOLDER: &str = "/home/dev/.cache/otty/agent-probe";
    const PROJECT: &str =
        ".claudeprobe/projects/-home-dev--cache-otty-agent-probe";

    const PROMPT: &str = "\
╭───────────────────────────────────────────────╮
│ ✻ Welcome to Claude Code!                     │
╰───────────────────────────────────────────────╯

> Try \"how does <filepath> work?\"
  ? for shortcuts
";

    const COMMAND_LIST: &str = "\
> /usage
  /usage    Show plan usage limits
";

    const PANEL: &str = "\
> /usage
 Settings:  Status   Config   Usage

 Current session
 ██████████████▌                                    29% used
 Resets 5pm (Europe/Berlin)

 Current week (all models)
 ████████▌                                          17% used
 Resets Nov 20, 10am (Europe/Berlin)

 Esc to exit
";

    const LOADING: &str = "\
> /usage
 Settings:  Status   Config   Usage

 Current session
 Loading usage data…
";

    const LOGIN: &str = "\
 Select login method:

 ❯ 1. Claude account with subscription
   2. Anthropic Console account
";

    const TRUST: &str = "\
 Accessing workspace:

 /home/dev/.cache/otty/agent-probe

 Quick safety check: Is this a project you created or one you trust?

 ❯ 1. Yes, I trust this folder
   2. No, exit

 Enter to confirm · Esc to cancel
";

    const API_KEY_QUESTION: &str = "\
 Do you want to use this API key?

 ❯ 1. Yes
   2. No (recommended)
";

    const ARGS: [&str; 5] = [
        "--allowed-tools",
        "",
        "--strict-mcp-config",
        "--settings",
        r#"{"remoteControlAtStartup":false,"disableAllHooks":true}"#,
    ];

    /// One step of a made-up screen: a new screen, or the program's end.
    #[derive(Debug, Clone)]
    enum Step {
        Show(String),
        End,
    }

    /// What the made-up terminal shows: the steps before any key, and the
    /// steps after each key typed, by the keys.
    struct Script {
        at_start: Vec<Step>,
        after_keys: Vec<(&'static str, Vec<Step>)>,
    }

    /// What the made-up terminal shares between its calls.
    #[derive(Default)]
    struct Played {
        steps: VecDeque<Step>,
        shown: String,
        sent: Vec<String>,
    }

    fn show(text: &str) -> Step {
        Step::Show(text.to_string())
    }

    fn fast() -> Timing {
        Timing {
            limit_ms: 400,
            ready_after_ms: 0,
            enter_again_after_ms: 50,
            look_ms: 20,
        }
    }

    fn folders() -> Vec<String> {
        vec![FOLDER.to_string()]
    }

    fn panel_windows() -> Vec<LimitWindow> {
        let window =
            |minutes: u32, percent_used: f64, reset: &str| LimitWindow {
                length_minutes: Some(minutes),
                model: None,
                percent_used,
                resets_at_ms: None,
                reset_text: Some(reset.to_string()),
            };

        vec![
            window(300, 29.0, "Resets 5pm (Europe/Berlin)"),
            window(10_080, 17.0, "Resets Nov 20, 10am (Europe/Berlin)"),
        ]
    }

    fn failed(text: &str) -> ReadError {
        ReadError::Failed(text.to_string())
    }

    /// A made-up terminal that plays `script`: the steps are given one by
    /// one, then nothing changes until the time asked. A program that
    /// ended fails every wait, as OTTY does, and keeps its last screen.
    fn terminal(script: Script) -> (MockProbeTerminal, Arc<Mutex<Played>>) {
        let played = Arc::new(Mutex::new(Played {
            steps: VecDeque::from(script.at_start),
            ..Played::default()
        }));
        let mut after_keys = script.after_keys;
        let mut terminal = MockProbeTerminal::new();

        let typed = Arc::clone(&played);
        terminal.expect_send_keys().returning(move |keys| {
            let mut played = typed.lock().expect("the play");
            played.sent.push(keys.to_string());
            if let Some(index) =
                after_keys.iter().position(|(after, _)| *after == keys)
            {
                let (_, next) = after_keys.remove(index);
                played.steps.extend(next);
            }
            Ok(())
        });
        let waited = Arc::clone(&played);
        terminal
            .expect_wait_for_change()
            .returning(move |timeout_ms| {
                let next = waited.lock().expect("the play").steps.pop_front();
                match next {
                    Some(Step::Show(text)) => {
                        waited.lock().expect("the play").shown = text;
                        Ok(true)
                    },
                    Some(Step::End) => {
                        let mut played = waited.lock().expect("the play");
                        played.steps.push_front(Step::End);
                        Err(HostError::Failed(
                            "claude: it has ended".to_string(),
                        ))
                    },
                    None => {
                        std::thread::sleep(Duration::from_millis(u64::from(
                            timeout_ms,
                        )));
                        Ok(false)
                    },
                }
            });
        let seen = Arc::clone(&played);
        terminal.expect_screen().returning(move || {
            Ok(seen.lock().expect("the play").shown.clone())
        });

        (terminal, played)
    }

    /// The command that renews the login of the probed profile.
    fn claude() -> Command {
        Command {
            program: "claude".to_string(),
            args: Vec::new(),
            env: vec![(
                "CLAUDE_CONFIG_DIR".to_string(),
                "/made-up".to_string(),
            )],
        }
    }

    /// Reads the panel of a made-up terminal that plays `script`, and the
    /// keys typed.
    fn read(
        script: Script,
    ) -> (Result<Vec<LimitWindow>, ReadError>, Vec<String>) {
        let (terminal, played) = terminal(script);

        let answer = read_panel(&terminal, &folders(), &claude(), fast());

        let sent = played.lock().expect("the play").sent.clone();
        (answer, sent)
    }

    #[test]
    fn the_panel_is_read_and_closed() {
        let (answer, sent) = read(Script {
            at_start: vec![show(PROMPT)],
            after_keys: vec![(USAGE_KEYS, vec![show(LOADING), show(PANEL)])],
        });

        assert_eq!(answer, Ok(panel_windows()));
        assert_eq!(sent, [USAGE_KEYS, ESCAPE]);
    }

    #[test]
    fn enter_is_typed_again_when_it_only_picked_the_command() {
        let (answer, sent) = read(Script {
            at_start: vec![show(PROMPT)],
            after_keys: vec![
                (USAGE_KEYS, vec![show(COMMAND_LIST)]),
                (ENTER, vec![show(PANEL)]),
            ],
        });

        assert_eq!(answer, Ok(panel_windows()));
        assert_eq!(sent, [USAGE_KEYS, ENTER, ESCAPE]);
    }

    #[test]
    fn a_panel_that_never_appears_ends_the_reading_in_time() {
        let started = std::time::Instant::now();

        let (answer, sent) = read(Script {
            at_start: vec![show(PROMPT)],
            after_keys: Vec::new(),
        });

        assert_eq!(
            answer,
            Err(ReadError::Host(HostError::TimedOut(
                "claude showed no usage in 0 s".to_string()
            )))
        );
        assert!(started.elapsed() < Duration::from_secs(5));
        assert_eq!(sent.first().map(String::as_str), Some(USAGE_KEYS));
    }

    #[test]
    fn a_panel_that_can_not_be_read_is_unexpected() {
        let unreadable = PANEL
            .replace("29% used", "a third")
            .replace("17% used", "a little");

        let (answer, _) = read(Script {
            at_start: vec![show(PROMPT)],
            after_keys: vec![(USAGE_KEYS, vec![Step::Show(unreadable)])],
        });

        assert_eq!(
            answer,
            Err(ReadError::Unexpected(
                "the /usage panel shows no percent used or left".to_string()
            ))
        );
    }

    #[test]
    fn a_login_screen_is_never_answered() {
        let (answer, sent) = read(Script {
            at_start: vec![show(LOGIN)],
            after_keys: Vec::new(),
        });

        let Err(ReadError::NeedsUser(need)) = answer else {
            panic!("a need: {answer:?}");
        };
        assert_eq!(need.id, "login-expired");
        assert!(need.text.contains("Open Claude Code"), "{}", need.text);
        assert_eq!(need.action, otty_plugin_sdk::NeedAction::Run(claude()));
        assert!(sent.is_empty(), "typed {sent:?}");
    }

    #[test]
    fn another_question_is_never_answered() {
        let (answer, sent) = read(Script {
            at_start: vec![show(API_KEY_QUESTION)],
            after_keys: Vec::new(),
        });

        assert_eq!(
            answer,
            Err(failed("Claude Code asks whether to use an API key"))
        );
        assert!(sent.is_empty(), "typed {sent:?}");
    }

    /// The probe trusts its own folder, then reads the panel as usual.
    #[test]
    fn the_trust_question_for_its_own_folder_is_answered_and_the_panel_read() {
        let (answer, sent) = read(Script {
            at_start: vec![show(TRUST)],
            after_keys: vec![
                (ENTER, vec![show(PROMPT)]),
                (USAGE_KEYS, vec![show(PANEL)]),
            ],
        });

        assert_eq!(answer, Ok(panel_windows()));
        assert_eq!(sent, [ENTER, USAGE_KEYS, ESCAPE]);
    }

    #[test]
    fn the_marker_is_moved_to_yes_before_the_trust_is_confirmed() {
        let marker_on_no = TRUST
            .replace("❯ 1. Yes", "  1. Yes")
            .replace("  2. No", "❯ 2. No");

        let (answer, sent) = read(Script {
            at_start: vec![Step::Show(marker_on_no)],
            after_keys: vec![
                (ARROW_UP, vec![show(TRUST)]),
                (ENTER, vec![show(PROMPT)]),
                (USAGE_KEYS, vec![show(PANEL)]),
            ],
        });

        assert_eq!(answer, Ok(panel_windows()));
        assert_eq!(sent, [ARROW_UP, ENTER, USAGE_KEYS, ESCAPE]);
    }

    #[test]
    fn a_trust_question_for_another_folder_is_never_answered() {
        let other = TRUST.replace(FOLDER, "/home/dev/work/secret");

        let (answer, sent) = read(Script {
            at_start: vec![Step::Show(other)],
            after_keys: Vec::new(),
        });

        assert_eq!(
            answer,
            Err(failed("Claude Code asks whether to trust another folder"))
        );
        assert!(sent.is_empty(), "typed {sent:?}");
    }

    /// Enter is pressed once for each new screen of the question, never
    /// again on a screen that did not change.
    #[test]
    fn a_trust_question_that_stays_ends_the_reading_in_time() {
        let (answer, sent) = read(Script {
            at_start: vec![show(TRUST)],
            after_keys: Vec::new(),
        });

        assert_eq!(
            answer,
            Err(failed(
                "Claude Code asks again whether to trust OTTY's folder"
            ))
        );
        assert_eq!(sent, [ENTER]);
    }

    #[test]
    fn claude_that_ends_as_not_found_is_not_found() {
        let (answer, _) = read(Script {
            at_start: vec![show("zsh: command not found: claude"), Step::End],
            after_keys: Vec::new(),
        });

        assert_eq!(answer, Err(failed("claude is not found")));
    }

    #[test]
    fn claude_that_ends_with_no_reason_on_the_screen_stopped() {
        let (answer, _) = read(Script {
            at_start: vec![show(PROMPT), Step::End],
            after_keys: Vec::new(),
        });

        assert_eq!(
            answer,
            Err(failed("claude stopped before it showed its usage"))
        );
    }

    #[test]
    fn a_key_that_can_not_be_typed_ends_the_reading() {
        let mut terminal = MockProbeTerminal::new();
        let mut looks = 0;
        // The first look shows the prompt, the next ones see it still.
        terminal.expect_wait_for_change().returning(move |_| {
            looks += 1;
            Ok(looks == 1)
        });
        terminal
            .expect_screen()
            .returning(|| Ok(PROMPT.to_string()));
        terminal
            .expect_send_keys()
            .times(1)
            .returning(|_| Err(HostError::Failed("closed".to_string())));

        let answer = read_panel(&terminal, &folders(), &claude(), fast());

        assert_eq!(
            answer,
            Err(ReadError::Host(HostError::Failed("closed".to_string())))
        );
    }

    /// Through the test host: the provider part starts `claude` with the
    /// arguments and the variables its manifest declares, pointed at the
    /// profile folder the account was found in, and the session files of
    /// the run are removed.
    #[test]
    fn claude_starts_as_declared_and_its_session_files_are_removed() {
        let host = host().with_terminal_folder(FOLDER);
        host.answer_terminal("claude", &ARGS, &[PROMPT, PANEL]);
        let folder = path(&host, ".claudeprobe");
        let folders = [login_folder(&host, ".claudeprobe")];

        let windows = as_provider_in(&host, &folders, || {
            read_windows(&folder, &claude_in_folder(&folder))
        });

        let started = host.started();
        assert_eq!(windows, Ok(panel_windows()));
        assert_eq!(started.len(), 1);
        assert_eq!(started[0].program(), "claude");
        assert_eq!(started[0].args(), ARGS);
        assert_eq!(
            started[0].env(),
            [
                ("CLAUDE_CONFIG_DIR".to_string(), folder.clone()),
                ("DISABLE_AUTOUPDATER".to_string(), "1".to_string()),
            ]
        );
        let mut removed = host.removed();
        removed.sort();
        assert_eq!(
            removed,
            [
                std::path::PathBuf::from(path(
                    &host,
                    &format!("{PROJECT}/a1.jsonl")
                )),
                std::path::PathBuf::from(path(
                    &host,
                    &format!("{PROJECT}/b2.jsonl")
                )),
            ]
        );
    }

    /// A profile in the default folder sets no config folder variable.
    #[test]
    fn the_default_profile_sets_only_the_update_variable() {
        let launch = Command {
            program: "claude".to_string(),
            args: Vec::new(),
            env: Vec::new(),
        };

        let command = probe_command(&launch);

        assert_eq!(command.program, "claude");
        assert_eq!(command.args, ARGS);
        assert_eq!(
            command.env,
            [("DISABLE_AUTOUPDATER".to_string(), "1".to_string())]
        );
    }

    #[test]
    fn a_look_never_waits_past_the_deadline() {
        assert_eq!(look_ms(250, 10_000), 250);
        assert_eq!(look_ms(250, 40), 40);
        assert_eq!(look_ms(250, -5), 0);
    }
}
