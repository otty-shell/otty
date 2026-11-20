//! The second source of Claude Code limits: `claude` in a hidden terminal,
//! asked for its `/usage` panel.
//!
//! The run follows CodexBar's Claude CLI probe (MIT licence),
//! <https://github.com/steipete/CodexBar/blob/main/Sources/CodexBarCore/Providers/Claude/ClaudeCLISession.swift>
//! and the "CLI PTY" part of its `docs/claude.md`: `claude` starts with no
//! tools, no MCP servers, no hooks and no remote control, in a working
//! folder of its own and a screen of 160 by 200, so the panel is never cut.
//! Once its screen is still, `/usage` and Enter are typed; the panel is
//! read from the screen, closed with Escape and the program is stopped.
//!
//! Claude Code asks whether to trust a folder the first time it runs there.
//! As CodexBar does, the probe answers "Yes" for its own folder, and only
//! for it (`trust_question.rs`), then reads the panel as usual. Claude Code
//! keeps that answer in the profile's `.claude.json`, as the trust of the
//! probe's folder: the one change to the agent's data OTTY makes, accepted
//! by the owner (spec Decision 12). Claude Code may also renew its own
//! login token while it runs; OTTY never reads or writes the token here.
//!
//! Any other question, and a login, ends the reading with no key typed.
//! The program is stopped in every case, and the session files it left are
//! removed.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::agent::{PROGRAM, start_command};
use super::probe_memory::ProbeMemory;
use super::probe_sessions::remove_probe_sessions;
use super::trust_question::{TrustAnswer, answer_trust};
use super::usage_screen::{UsageScreen, read_usage_screen};
use crate::command::AgentCommand;
use crate::error::{ClaudeQuestion, SourceError, Unavailable, Unexpected};
use crate::hidden_terminal::{RunningTerminal, ScreenOutput, StartTerminal};
use crate::limits::{LimitSource, Limits};
use crate::profile::Profile;
use crate::source_id::SourceId;

/// The width of the hidden screen, in characters.
const COLUMNS: u16 = 160;
/// The height of the hidden screen, in lines: the panel can be a hundred
/// lines long.
const ROWS: u16 = 200;

/// How long one reading may take, from the start of the program to the
/// panel. The user's shell reads its start files first, Claude Code takes
/// a few seconds to start, and the panel asks Anthropic's server.
const TIME_LIMIT: Duration = Duration::from_secs(30);
/// How long the screen must show something before `/usage` is typed, as in
/// CodexBar, so Claude Code is ready for keys.
const READY_AFTER: Duration = Duration::from_secs(2);
/// How long after `/usage` with no panel Enter is typed again: the first
/// Enter may only pick the command in Claude Code's list of commands.
const ENTER_AGAIN_AFTER: Duration = Duration::from_secs(3);
/// How many times Enter may be typed again.
const ENTER_AGAIN_TIMES: usize = 3;
/// How long one look at the screen waits for a change.
const LOOK: Duration = Duration::from_millis(250);
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
/// The settings that turn off remote control and every hook for this run
/// only, as CodexBar does.
const PROBE_SETTINGS: &str =
    r#"{"remoteControlAtStartup":false,"disableAllHooks":true}"#;

/// The times the reading waits for, kept apart so the tests wait less.
#[derive(Debug, Clone, Copy)]
struct Timing {
    limit: Duration,
    ready_after: Duration,
    enter_again_after: Duration,
    look: Duration,
}

/// Reads the limits from the `/usage` panel of `claude`, started in a
/// hidden terminal with `terminals`.
pub(super) struct UsageProbe {
    terminals: Arc<dyn StartTerminal>,
    memory: Arc<ProbeMemory>,
    timing: Timing,
}

impl UsageProbe {
    /// The source that starts `claude` with `terminals` and remembers its
    /// answers in `memory`.
    pub(super) fn new(
        terminals: Arc<dyn StartTerminal>,
        memory: Arc<ProbeMemory>,
    ) -> Self {
        let timing = Timing {
            limit: TIME_LIMIT,
            ready_after: READY_AFTER,
            enter_again_after: ENTER_AGAIN_AFTER,
            look: LOOK,
        };

        Self {
            terminals,
            memory,
            timing,
        }
    }

    /// Reads the panel of `claude` started in `terminal` for `profile` and
    /// stops it, then removes the session files it left.
    fn read_panel(
        &self,
        profile: &Profile,
        mut terminal: Box<dyn RunningTerminal>,
    ) -> Result<Limits, SourceError> {
        let working_dir = self.terminals.working_dir();
        let probe_dirs = probe_dirs(&working_dir);

        let answer =
            wait_for_panel(terminal.as_mut(), &probe_dirs, self.timing);
        if answer.is_ok() {
            // The panel is read; closing it is only polite, so a key that
            // can not be typed changes nothing.
            let _ = terminal.send(ESCAPE);
        }
        terminal.stop();

        remove_probe_sessions(profile.config_dir(), &working_dir);

        answer
    }
}

#[cfg(test)]
impl UsageProbe {
    fn with_timing(
        terminals: Arc<dyn StartTerminal>,
        memory: Arc<ProbeMemory>,
        timing: Timing,
    ) -> Self {
        Self {
            terminals,
            memory,
            timing,
        }
    }
}

impl LimitSource for UsageProbe {
    fn id(&self) -> SourceId {
        SourceId::ClaudeUsageCommand
    }

    /// The question whether to trust the probe's own folder is answered.
    /// A login or another question on the screen, a program that can not
    /// start or does not show the panel in time is "not available" or "not
    /// logged in"; a program that ends with no reason on the screen
    /// "stopped unexpectedly"; a panel that can not be read is "unexpected
    /// answer". A reading that started a program less than five minutes
    /// ago is not started again: its answer is given again. A program that
    /// could not start is not remembered, so the next read tries again.
    fn limits(&self, profile: &Profile) -> Result<Limits, SourceError> {
        let config_dir = profile.config_dir();
        let started_at = Instant::now();

        if let Some(answer) = self.memory.recent(config_dir, started_at) {
            return answer;
        }

        let command = probe_command(profile);
        let terminal = self.terminals.start(&command, COLUMNS, ROWS).map_err(
            |reason| {
                SourceError::NotAvailable(Unavailable::NotStarted {
                    program: PROGRAM,
                    reason,
                })
            },
        )?;

        let answer = self.read_panel(profile, terminal);
        self.memory.remember(config_dir, started_at, answer.clone());

        answer
    }
}

/// `claude` of `profile` that can run nothing: no tools, no MCP servers of
/// the user, no hooks, no remote control and no update of itself.
fn probe_command(profile: &Profile) -> AgentCommand {
    start_command(profile)
        .with_arg("--allowed-tools")
        .with_arg("")
        .with_arg("--strict-mcp-config")
        .with_arg("--settings")
        .with_arg(PROBE_SETTINGS)
        .with_env("DISABLE_AUTOUPDATER", "1")
}

/// The probe's folder `working_dir` as OTTY names it and, when it differs,
/// as the system resolves its links, which is how Claude Code may show it.
fn probe_dirs(working_dir: &Path) -> Vec<PathBuf> {
    let mut dirs = vec![working_dir.to_path_buf()];

    if let Ok(real) = std::fs::canonicalize(working_dir)
        && real != working_dir
    {
        dirs.push(real);
    }

    dirs
}

/// Waits for the screen of `terminal` to show the panel and reads it.
///
/// The question whether to trust the folder is answered for the probe's
/// own folder, one key for each new screen of the question. `/usage` is
/// typed once the screen has shown something else for a while and stays
/// still. While no panel shows, Enter is typed again now and then. A login
/// or another question ends the wait at once, before any key is typed.
fn wait_for_panel(
    terminal: &mut dyn RunningTerminal,
    probe_dirs: &[PathBuf],
    timing: Timing,
) -> Result<Limits, SourceError> {
    let started = Instant::now();
    let deadline = started + timing.limit;
    let mut screen = String::new();
    let mut first_shown: Option<Instant> = None;
    let mut asked_at: Option<Instant> = None;
    let mut enters_left = ENTER_AGAIN_TIMES;

    loop {
        let look_until = deadline.min(Instant::now() + timing.look);
        let output = terminal.next_screen(look_until);
        let is_still = output == ScreenOutput::Unchanged;
        if let ScreenOutput::Screen(text) = &output {
            screen.clone_from(text);
        }

        let reading = read_usage_screen(&screen);
        match reading {
            UsageScreen::Usage(limits) => return Ok(limits),
            UsageScreen::Stopped(error) => return Err(error),
            _ => {},
        }
        if output == ScreenOutput::Ended {
            return Err(ended(&screen));
        }

        let now = Instant::now();
        if now >= deadline {
            return Err(timed_out(reading, timing.limit));
        }

        if matches!(reading, UsageScreen::TrustQuestion) {
            if matches!(output, ScreenOutput::Screen(_)) {
                answer_trust_question(terminal, &screen, probe_dirs)?;
            }
            // Claude Code is ready for `/usage` only a while after the
            // question is gone.
            first_shown = None;
            continue;
        }

        if first_shown.is_none() && !screen.trim().is_empty() {
            first_shown = Some(now);
        }
        let is_ready =
            first_shown.is_some_and(|shown| now >= shown + timing.ready_after);
        let panel_shows = matches!(reading, UsageScreen::Unreadable);

        match asked_at {
            None if is_ready && is_still => {
                type_keys(terminal, USAGE_KEYS)?;
                asked_at = Some(now);
            },
            Some(at)
                if !panel_shows
                    && enters_left > 0
                    && now >= at + timing.enter_again_after =>
            {
                type_keys(terminal, ENTER)?;
                asked_at = Some(now);
                enters_left -= 1;
            },
            _ => {},
        }
    }
}

/// Answers the question on `screen` whether to trust the folder: one key
/// toward "Yes" for the probe's own folder in `probe_dirs`, or none. A
/// question about another folder ends the reading.
fn answer_trust_question(
    terminal: &mut dyn RunningTerminal,
    screen: &str,
    probe_dirs: &[PathBuf],
) -> Result<(), SourceError> {
    match answer_trust(screen, probe_dirs) {
        TrustAnswer::Confirm => type_keys(terminal, ENTER),
        TrustAnswer::MoveUp => type_keys(terminal, ARROW_UP),
        TrustAnswer::MoveDown => type_keys(terminal, ARROW_DOWN),
        TrustAnswer::Refuse(folder) => Err(SourceError::NotAvailable(
            Unavailable::ClaudeAsks(ClaudeQuestion::TrustOtherFolder(folder)),
        )),
        _ => Ok(()),
    }
}

/// Types `keys` into `terminal`.
fn type_keys(
    terminal: &mut dyn RunningTerminal,
    keys: &str,
) -> Result<(), SourceError> {
    terminal.send(keys).map_err(|reason| {
        SourceError::NotAvailable(Unavailable::NotWritten {
            program: PROGRAM,
            reason,
        })
    })
}

/// Why the program ended before the panel, from its last `screen`: a
/// program that is not found is "not available", any other end "stopped
/// unexpectedly".
fn ended(screen: &str) -> SourceError {
    if screen.to_lowercase().contains("not found") {
        return SourceError::NotAvailable(Unavailable::ProgramNotFound(
            PROGRAM,
        ));
    }

    SourceError::ProgramStopped(PROGRAM)
}

/// Why the time ran out, from the last `reading` of the screen: a panel
/// that showed but could not be read, a trust question that stayed after
/// its answer, or no panel at all.
fn timed_out(reading: UsageScreen, limit: Duration) -> SourceError {
    match reading {
        UsageScreen::Unreadable => {
            SourceError::UnexpectedAnswer(Unexpected::UsagePanelUnreadable)
        },
        UsageScreen::TrustQuestion => SourceError::NotAvailable(
            Unavailable::ClaudeAsks(ClaudeQuestion::TrustOwnFolderAgain),
        ),
        _ => SourceError::NotAvailable(Unavailable::NoAnswerInTime {
            program: PROGRAM,
            seconds: limit.as_secs(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;
    use std::time::Instant;

    use super::*;
    use crate::hidden_terminal::{
        MockRunningTerminal, MockStartTerminal, ScreenOutput,
    };
    use crate::limits::{Billing, LimitWindow, UsageWindows, WindowId};
    use crate::program::ProgramError;
    use crate::testing::{TempDir, claude_profile, write_file};

    const WORKING_DIR: &str = "/home/user/.cache/otty/agent-probe";
    const USAGE: &str = USAGE_KEYS;

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

 /home/user/.cache/otty/agent-probe

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

    /// The arrow key up.
    const UP: &str = "\u{1b}[A";

    /// What the fake terminal shows: the screens before any key, and the
    /// screens after each key typed, by the keys.
    struct Script {
        at_start: Vec<ScreenOutput>,
        after_keys: Vec<(&'static str, Vec<ScreenOutput>)>,
    }

    /// The command and the size a terminal was started with.
    struct Start {
        command: AgentCommand,
        size: (u16, u16),
    }

    /// What one ask of the source did.
    struct Asked {
        start: Option<Start>,
        sent: Vec<String>,
        result: Result<Limits, SourceError>,
    }

    fn screen(text: &str) -> ScreenOutput {
        ScreenOutput::Screen(String::from(text))
    }

    fn fast() -> Timing {
        Timing {
            limit: Duration::from_millis(400),
            ready_after: Duration::ZERO,
            enter_again_after: Duration::from_millis(50),
            look: Duration::from_millis(20),
        }
    }

    fn profile() -> Profile {
        claude_profile(Path::new("/home/user/.claude-work"))
    }

    fn panel_limits() -> Limits {
        let windows = vec![
            LimitWindow::new(WindowId::hours(5), 29.0, None)
                .with_reset_text("Resets 5pm (Europe/Berlin)"),
            LimitWindow::new(WindowId::days(7), 17.0, None)
                .with_reset_text("Resets Nov 20, 10am (Europe/Berlin)"),
        ];

        Limits::new(
            Billing::UsageWindows(UsageWindows::new(windows, None)),
            None,
        )
    }

    /// A fake terminal that plays `script`: the screens are given one by
    /// one, then nothing changes until the time asked. It must be stopped
    /// exactly once.
    fn fake_terminal(
        script: Script,
        sent: Arc<Mutex<Vec<String>>>,
    ) -> MockRunningTerminal {
        let screens = Arc::new(Mutex::new(VecDeque::from(script.at_start)));
        let mut after_keys = script.after_keys;
        let mut terminal = MockRunningTerminal::new();

        let typed = Arc::clone(&sent);
        let queued = Arc::clone(&screens);
        terminal.expect_send().returning(move |keys| {
            typed.lock().expect("the keys").push(keys.to_owned());
            if let Some(index) =
                after_keys.iter().position(|(after, _)| *after == keys)
            {
                let (_, next) = after_keys.remove(index);
                queued.lock().expect("the screens").extend(next);
            }
            Ok(())
        });
        terminal.expect_next_screen().returning(move |until| {
            let next = screens.lock().expect("the screens").pop_front();
            next.unwrap_or_else(|| {
                std::thread::sleep(
                    until.saturating_duration_since(Instant::now()),
                );
                ScreenOutput::Unchanged
            })
        });
        terminal.expect_stop().times(1).return_const(());

        terminal
    }

    /// A starter that starts `terminal` once in `working_dir` and keeps
    /// how it was started in `starts`.
    fn starter(
        working_dir: &Path,
        terminal: MockRunningTerminal,
        starts: &Arc<Mutex<Option<Start>>>,
    ) -> MockStartTerminal {
        let mut terminals = MockStartTerminal::new();
        terminals
            .expect_working_dir()
            .return_const(working_dir.to_path_buf());
        let starts = Arc::clone(starts);
        terminals.expect_start().times(1).return_once(
            move |command, columns, rows| {
                *starts.lock().expect("the start") = Some(Start {
                    command: command.clone(),
                    size: (columns, rows),
                });
                Ok(Box::new(terminal))
            },
        );

        terminals
    }

    fn ask_with(
        probe: &UsageProbe,
        profile: &Profile,
        sent: &Arc<Mutex<Vec<String>>>,
        starts: &Arc<Mutex<Option<Start>>>,
    ) -> Asked {
        let result = probe.limits(profile);

        Asked {
            start: starts.lock().expect("the start").take(),
            sent: sent.lock().expect("the keys").clone(),
            result,
        }
    }

    /// Asks the source of `profile` once, with a terminal that plays
    /// `script` in `working_dir`.
    fn ask_in(profile: &Profile, working_dir: &Path, script: Script) -> Asked {
        let sent = Arc::new(Mutex::new(Vec::new()));
        let starts = Arc::new(Mutex::new(None));
        let terminal = fake_terminal(script, Arc::clone(&sent));
        let terminals = starter(working_dir, terminal, &starts);

        let probe = UsageProbe::with_timing(
            Arc::new(terminals),
            Arc::new(ProbeMemory::default()),
            fast(),
        );

        ask_with(&probe, profile, &sent, &starts)
    }

    fn ask(script: Script) -> Asked {
        ask_in(&profile(), Path::new(WORKING_DIR), script)
    }

    #[test]
    fn the_panel_is_read_closed_and_claude_is_stopped() {
        let asked = ask(Script {
            at_start: vec![screen(PROMPT)],
            after_keys: vec![(USAGE, vec![screen(LOADING), screen(PANEL)])],
        });

        assert_eq!(asked.result, Ok(panel_limits()));
        assert_eq!(asked.sent, [USAGE, ESCAPE]);
    }

    #[test]
    fn claude_starts_with_the_profile_folder_and_can_run_nothing() {
        let asked = ask(Script {
            at_start: vec![screen(PROMPT)],
            after_keys: vec![(USAGE, vec![screen(PANEL)])],
        });

        let settings =
            r#"{"remoteControlAtStartup":false,"disableAllHooks":true}"#;
        let start = asked.start.expect("claude started");
        assert_eq!(
            start.command,
            AgentCommand::new("claude")
                .with_arg("--allowed-tools")
                .with_arg("")
                .with_arg("--strict-mcp-config")
                .with_arg("--settings")
                .with_arg(settings)
                .with_env("CLAUDE_CONFIG_DIR", "/home/user/.claude-work")
                .with_env("DISABLE_AUTOUPDATER", "1")
        );
        assert_eq!(start.size, (160, 200));
    }

    #[test]
    fn the_default_profile_in_its_default_folder_sets_no_config_folder() {
        let profile = claude_profile(Path::new("/home/user/.claude"))
            .with_agent_default_dir(Path::new("/home/user/.claude"));

        let asked = ask_in(
            &profile,
            Path::new(WORKING_DIR),
            Script {
                at_start: vec![screen(PROMPT)],
                after_keys: vec![(USAGE, vec![screen(PANEL)])],
            },
        );

        let command = asked.start.expect("claude started").command;
        assert_eq!(
            command.env(),
            [(String::from("DISABLE_AUTOUPDATER"), String::from("1"))]
        );
    }

    #[test]
    fn enter_is_typed_again_when_it_only_picked_the_command() {
        let asked = ask(Script {
            at_start: vec![screen(PROMPT)],
            after_keys: vec![
                (USAGE, vec![screen(COMMAND_LIST)]),
                (ENTER, vec![screen(PANEL)]),
            ],
        });

        assert_eq!(asked.result, Ok(panel_limits()));
        assert_eq!(asked.sent, [USAGE, ENTER, ESCAPE]);
    }

    #[test]
    fn a_panel_that_never_appears_stops_claude_in_time() {
        let started = Instant::now();

        let asked = ask(Script {
            at_start: vec![screen(PROMPT)],
            after_keys: Vec::new(),
        });

        assert_eq!(
            asked.result,
            Err(SourceError::NotAvailable(Unavailable::NoAnswerInTime {
                program: "claude",
                seconds: 0,
            }))
        );
        assert!(started.elapsed() < Duration::from_secs(5));
        assert_eq!(asked.sent.first().map(String::as_str), Some(USAGE));
    }

    #[test]
    fn a_panel_that_can_not_be_read_is_an_unexpected_answer() {
        let unreadable = PANEL.replace("29% used", "a third");
        let unreadable = unreadable.replace("17% used", "a little");

        let asked = ask(Script {
            at_start: vec![screen(PROMPT)],
            after_keys: vec![(USAGE, vec![ScreenOutput::Screen(unreadable)])],
        });

        assert_eq!(
            asked.result,
            Err(SourceError::UnexpectedAnswer(
                Unexpected::UsagePanelUnreadable
            ))
        );
    }

    #[test]
    fn a_login_screen_is_never_answered() {
        let asked = ask(Script {
            at_start: vec![screen(LOGIN)],
            after_keys: Vec::new(),
        });

        assert_eq!(asked.result, Err(SourceError::NotLoggedIn));
        assert!(asked.sent.is_empty(), "typed {:?}", asked.sent);
    }

    #[test]
    fn another_question_is_never_answered() {
        let asked = ask(Script {
            at_start: vec![screen(API_KEY_QUESTION)],
            after_keys: Vec::new(),
        });

        assert_eq!(
            asked.result,
            Err(SourceError::NotAvailable(Unavailable::ClaudeAsks(
                ClaudeQuestion::UseApiKey,
            )))
        );
        assert!(asked.sent.is_empty(), "typed {:?}", asked.sent);
    }

    /// Owner, issue 29 (spec Decision 12): the probe trusts its own
    /// folder, then reads the panel as usual.
    #[test]
    fn the_trust_question_for_its_own_folder_is_answered_and_the_panel_read() {
        let asked = ask(Script {
            at_start: vec![screen(TRUST)],
            after_keys: vec![
                (ENTER, vec![screen(PROMPT)]),
                (USAGE, vec![screen(PANEL)]),
            ],
        });

        assert_eq!(asked.result, Ok(panel_limits()));
        assert_eq!(asked.sent, [ENTER, USAGE, ESCAPE]);
    }

    #[test]
    fn the_marker_is_moved_to_yes_before_the_trust_is_confirmed() {
        let marker_on_no = TRUST
            .replace("❯ 1. Yes", "  1. Yes")
            .replace("  2. No", "❯ 2. No");

        let asked = ask(Script {
            at_start: vec![ScreenOutput::Screen(marker_on_no)],
            after_keys: vec![
                (UP, vec![screen(TRUST)]),
                (ENTER, vec![screen(PROMPT)]),
                (USAGE, vec![screen(PANEL)]),
            ],
        });

        assert_eq!(asked.result, Ok(panel_limits()));
        assert_eq!(asked.sent, [UP, ENTER, USAGE, ESCAPE]);
    }

    #[test]
    fn a_trust_question_for_another_folder_is_never_answered() {
        let other = TRUST.replace(WORKING_DIR, "/home/user/work/secret");

        let asked = ask(Script {
            at_start: vec![ScreenOutput::Screen(other)],
            after_keys: Vec::new(),
        });

        assert_eq!(
            asked.result,
            Err(SourceError::NotAvailable(Unavailable::ClaudeAsks(
                ClaudeQuestion::TrustOtherFolder(String::from(
                    "/home/user/work/secret",
                )),
            )))
        );
        assert!(asked.sent.is_empty(), "typed {:?}", asked.sent);
    }

    /// Enter is pressed once for each new screen of the question, never
    /// again on a screen that did not change.
    #[test]
    fn a_trust_question_that_stays_stops_claude_in_time() {
        let asked = ask(Script {
            at_start: vec![screen(TRUST)],
            after_keys: Vec::new(),
        });

        assert_eq!(
            asked.result,
            Err(SourceError::NotAvailable(Unavailable::ClaudeAsks(
                ClaudeQuestion::TrustOwnFolderAgain
            )))
        );
        assert_eq!(asked.sent, [ENTER]);
    }

    #[test]
    fn claude_that_ends_before_the_panel_is_not_available() {
        let asked = ask(Script {
            at_start: vec![
                screen("zsh: command not found: claude"),
                ScreenOutput::Ended,
            ],
            after_keys: Vec::new(),
        });

        assert_eq!(
            asked.result,
            Err(SourceError::NotAvailable(Unavailable::ProgramNotFound(
                "claude"
            )))
        );
    }

    #[test]
    fn claude_that_ends_with_no_reason_on_the_screen_stopped_unexpectedly() {
        let asked = ask(Script {
            at_start: vec![screen(PROMPT), ScreenOutput::Ended],
            after_keys: Vec::new(),
        });

        assert_eq!(asked.result, Err(SourceError::ProgramStopped("claude")));
    }

    #[test]
    fn a_program_that_is_not_found_is_not_available() {
        let mut terminals = MockStartTerminal::new();
        terminals
            .expect_working_dir()
            .return_const(PathBuf::from(WORKING_DIR));
        terminals.expect_start().times(1).returning(|_, _, _| {
            Err(ProgramError::ShellNotFound(String::from("/no/shell")))
        });
        let probe = UsageProbe::with_timing(
            Arc::new(terminals),
            Arc::new(ProbeMemory::default()),
            fast(),
        );

        let result = probe.limits(&profile());

        assert_eq!(
            result,
            Err(SourceError::NotAvailable(Unavailable::NotStarted {
                program: "claude",
                reason: ProgramError::ShellNotFound(String::from("/no/shell")),
            }))
        );
    }

    /// A program that could not start, such as one OTTY did not let start
    /// while Agents mode is away, is not remembered: the next read starts
    /// it.
    #[test]
    fn a_program_that_could_not_start_is_started_at_the_next_read() {
        let mut terminals = MockStartTerminal::new();
        terminals
            .expect_working_dir()
            .return_const(PathBuf::from(WORKING_DIR));
        terminals
            .expect_start()
            .times(2)
            .returning(|_, _, _| Err(ProgramError::NotNow));
        let probe = UsageProbe::with_timing(
            Arc::new(terminals),
            Arc::new(ProbeMemory::default()),
            fast(),
        );

        let first = probe.limits(&profile());
        let second = probe.limits(&profile());

        let refused = Err(SourceError::NotAvailable(Unavailable::NotStarted {
            program: "claude",
            reason: ProgramError::NotNow,
        }));
        assert_eq!(first, refused);
        assert_eq!(second, refused);
    }

    #[test]
    fn the_session_files_claude_left_are_removed() {
        let temp = TempDir::new();
        let config_dir = temp.path().join(".claude-test");
        let working_dir = temp.path().join("agent-probe");
        let profile = claude_profile(&config_dir);
        let projects = config_dir.join("projects");
        let real_session = projects.join("-work-otty").join("c3.jsonl");
        write_file(&real_session, "{}");
        let name: String = working_dir
            .to_string_lossy()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect();
        let left = projects.join(name).join("a1.jsonl");
        write_file(&left, "{}");

        let asked = ask_in(
            &profile,
            &working_dir,
            Script {
                at_start: vec![screen(PROMPT)],
                after_keys: vec![(USAGE, vec![screen(PANEL)])],
            },
        );

        assert_eq!(asked.result, Ok(panel_limits()));
        assert!(!left.exists());
        assert!(real_session.is_file());
    }

    #[test]
    fn a_second_read_soon_after_gives_the_first_answer_and_starts_nothing() {
        let sent = Arc::new(Mutex::new(Vec::new()));
        let starts = Arc::new(Mutex::new(None));
        let terminal = fake_terminal(
            Script {
                at_start: vec![screen(LOGIN)],
                after_keys: Vec::new(),
            },
            Arc::clone(&sent),
        );
        let terminals = starter(Path::new(WORKING_DIR), terminal, &starts);
        let probe = UsageProbe::with_timing(
            Arc::new(terminals),
            Arc::new(ProbeMemory::default()),
            fast(),
        );

        let first = ask_with(&probe, &profile(), &sent, &starts);
        let second = ask_with(&probe, &profile(), &sent, &starts);

        assert_eq!(first.result, Err(SourceError::NotLoggedIn));
        assert_eq!(second.result, Err(SourceError::NotLoggedIn));
        assert!(second.start.is_none(), "claude is not started again");
    }
}
