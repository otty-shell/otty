//! An agent's program started in a terminal no one sees, for a source that
//! reads its screen, such as Claude Code's `/usage` panel: through the
//! user's login shell, the way the agent tab starts it, in a
//! pseudo-terminal of `otty-pty`, with its output fed to OTTY's own
//! terminal engine so the screen can be read as text.
//!
//! The program is never left running: [`RunningTerminal::stop`], called by
//! the source and again when the terminal is dropped, sends it `SIGHUP`, as
//! when a terminal window closes, and kills it when it does not end within
//! a second.

use std::path::{Path, PathBuf};
#[cfg(unix)]
use std::time::{Duration, Instant};

#[cfg(unix)]
use otty_agents::ScreenOutput;
use otty_agents::{AgentCommand, RunningTerminal, StartTerminal};
#[cfg(unix)]
use otty_app_shell::{exec_script, login_shell_args};
#[cfg(unix)]
use otty_libterm::surface::{Flags, SnapshotOwned, Surface, SurfaceConfig};
#[cfg(unix)]
use otty_libterm::{
    DefaultParser, TerminalEngine, TerminalEvent, TerminalEvents,
    TerminalHandle, TerminalOptions, TerminalRequest, TerminalSize, pty,
};

/// How long the screen must stay still after a change before it is given,
/// as in CodexBar, so a screen drawn in several writes is read whole.
#[cfg(unix)]
const SETTLE: Duration = Duration::from_millis(200);
/// How often the terminal looks for new output while it waits.
#[cfg(unix)]
const POLL: Duration = Duration::from_millis(20);
/// The variables that tell the program what terminal it runs in, the same
/// as in a terminal tab.
#[cfg(unix)]
const TERMINAL_VARIABLES: [(&str, &str); 2] =
    [("TERM", "xterm-256color"), ("COLORTERM", "truecolor")];

/// Starts agent programs in hidden terminals through the user's login
/// shell, in a working folder of OTTY's own.
pub(crate) struct LoginShellTerminals {
    shell: String,
    working_dir: PathBuf,
}

impl LoginShellTerminals {
    /// Terminals that run the user's shell at `shell`, such as `/bin/zsh`,
    /// in `working_dir`, which is created when it is missing.
    pub(crate) fn new(shell: &str, working_dir: &Path) -> Self {
        Self {
            shell: shell.to_string(),
            working_dir: working_dir.to_path_buf(),
        }
    }

    /// Checks what can be checked before anything starts: a shell that is
    /// not there, and the working folder, created when it is missing.
    fn check_start(&self) -> Result<(), String> {
        let shell = &self.shell;
        if !is_executable_file(Path::new(shell)) {
            return Err(format!("can not run the shell {shell}"));
        }

        std::fs::create_dir_all(&self.working_dir).map_err(|err| {
            let folder = self.working_dir.display();
            format!("can not create the folder {folder}: {err}")
        })
    }

    /// Starts `command` through the user's login shell, which replaces
    /// itself with the program, in a pseudo-terminal of `columns` by
    /// `rows`.
    #[cfg(unix)]
    fn start_terminal(
        &self,
        command: &AgentCommand,
        columns: u16,
        rows: u16,
    ) -> Result<PtyTerminal, String> {
        self.check_start()?;

        let script = exec_script(command.program(), command.args());
        let size = TerminalSize {
            cell_width: 1,
            cell_height: 1,
            cols: columns,
            rows,
        };
        let mut session = pty::local(&self.shell)
            .with_args(&login_shell_args(&script))
            .with_cwd(&self.working_dir)
            .with_size(size.into())
            .set_controling_tty_enable();
        for (name, value) in TERMINAL_VARIABLES {
            session = session.with_env(name, value);
        }
        for (name, value) in command.env() {
            session = session.with_env(name, value);
        }

        let session = session.spawn().map_err(|err| {
            let shell = &self.shell;
            format!("can not run the shell {shell}: {err}")
        })?;
        let surface = Surface::new(SurfaceConfig::default(), &size);
        let (mut engine, handle, events) = TerminalEngine::new(
            session,
            DefaultParser::default(),
            surface,
            TerminalOptions::default(),
        )
        .map_err(|err| format!("can not start the terminal: {err}"))?;
        let mut terminal = PtyTerminal {
            engine: None,
            handle,
            events,
            screen: String::new(),
            changed_at: None,
            has_ended: false,
        };
        let resized = engine.process_request(TerminalRequest::Resize(size));
        terminal.engine = Some(engine);

        match resized {
            Ok(()) => Ok(terminal),
            Err(err) => Err(format!("can not size the terminal: {err}")),
        }
    }
}

impl StartTerminal for LoginShellTerminals {
    fn working_dir(&self) -> PathBuf {
        self.working_dir.clone()
    }

    #[cfg(unix)]
    fn start(
        &self,
        command: &AgentCommand,
        columns: u16,
        rows: u16,
    ) -> Result<Box<dyn RunningTerminal>, String> {
        let terminal = self.start_terminal(command, columns, rows)?;

        Ok(Box::new(terminal))
    }

    /// Without a pseudo-terminal there is no hidden terminal.
    #[cfg(not(unix))]
    fn start(
        &self,
        _command: &AgentCommand,
        _columns: u16,
        _rows: u16,
    ) -> Result<Box<dyn RunningTerminal>, String> {
        self.check_start()?;

        Err(String::from("a hidden terminal needs a Unix system"))
    }
}

/// A program running in a pseudo-terminal, with its screen kept by OTTY's
/// terminal engine.
///
/// The engine is dropped when the program is stopped, so nothing of it is
/// left once the source is done.
#[cfg(unix)]
pub(crate) struct PtyTerminal {
    engine: Option<TerminalEngine<pty::LocalSession, DefaultParser, Surface>>,
    handle: TerminalHandle,
    events: TerminalEvents,
    screen: String,
    changed_at: Option<Instant>,
    has_ended: bool,
}

#[cfg(unix)]
impl PtyTerminal {
    /// Reads what the program wrote, answers the questions it asked the
    /// terminal (such as where the cursor is), notes whether it ended, and
    /// keeps the newest screen.
    fn read_output(&mut self) {
        let Some(engine) = self.engine.as_mut() else {
            self.has_ended = true;
            return;
        };

        if engine.on_readable().is_err() {
            self.has_ended = true;
        }
        let _ = engine.on_writable();

        while let Ok(event) = self.events.try_recv() {
            if matches!(event, TerminalEvent::ChildExit { .. }) {
                self.has_ended = true;
            }
        }

        if let Some(snapshot) = self.handle.take_snapshot() {
            let text = screen_text(&snapshot);
            if text != self.screen {
                self.screen = text;
                self.changed_at = Some(Instant::now());
            }
        }
    }
}

#[cfg(unix)]
impl RunningTerminal for PtyTerminal {
    fn send(&mut self, keys: &str) -> Result<(), String> {
        let Some(engine) = self.engine.as_mut().filter(|_| !self.has_ended)
        else {
            return Err(String::from("the program has ended"));
        };

        let bytes = keys.as_bytes().to_vec();
        engine
            .process_request(TerminalRequest::WriteBytes(bytes))
            .map_err(|err| err.to_string())
    }

    /// A screen that keeps changing is given when `until` comes, so the
    /// time limit is kept.
    fn next_screen(&mut self, until: Instant) -> ScreenOutput {
        loop {
            self.read_output();
            let now = Instant::now();

            if let Some(changed_at) = self.changed_at {
                let is_settled = now >= changed_at + SETTLE;
                if is_settled || self.has_ended || now >= until {
                    self.changed_at = None;
                    return ScreenOutput::Screen(self.screen.clone());
                }
            }
            if self.has_ended {
                return ScreenOutput::Ended;
            }
            if now >= until {
                return ScreenOutput::Unchanged;
            }

            std::thread::sleep(POLL);
        }
    }

    fn stop(&mut self) {
        let Some(mut engine) = self.engine.take() else {
            return;
        };

        // Shutting down sends SIGHUP, waits a second, then kills.
        if let Err(err) = engine.process_request(TerminalRequest::Shutdown) {
            log::warn!("the hidden terminal did not shut down: {err}");
        }
    }
}

#[cfg(unix)]
impl Drop for PtyTerminal {
    fn drop(&mut self) {
        self.stop();
    }
}

/// The screen of `snapshot` as text: one line for each row, with the
/// spaces at the end of each row and the empty rows at the end left out.
#[cfg(unix)]
fn screen_text(snapshot: &SnapshotOwned) -> String {
    let view = snapshot.view();
    let spacers = Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER;
    let mut rows = vec![String::new(); view.size.screen_lines];

    for indexed in view.cells {
        let row = usize::try_from(indexed.point.line.0)
            .ok()
            .and_then(|row| rows.get_mut(row));
        let Some(row) = row else {
            continue;
        };
        if indexed.cell.flags.intersects(spacers) {
            continue;
        }

        row.push(indexed.cell.c);
        if let Some(marks) = indexed.cell.zerowidth() {
            row.extend(marks);
        }
    }

    let rows: Vec<&str> = rows.iter().map(|row| row.trim_end()).collect();
    rows.join("\n").trim_end().to_string()
}

/// Whether `path` is a file the user may run.
#[cfg(unix)]
fn is_executable_file(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;

    std::fs::metadata(path).is_ok_and(|metadata| {
        metadata.is_file() && metadata.permissions().mode() & 0o111 != 0
    })
}

/// Whether `path` is a file the user may run.
#[cfg(not(unix))]
fn is_executable_file(path: &Path) -> bool {
    path.is_file()
}

#[cfg(all(test, unix))]
mod tests {
    use std::process::Command;

    use otty_app_testing::temp_dir::TempDir;

    use super::*;
    use crate::testing::{executable_script, fake_shell};

    /// A made-up agent program in `folder` that runs `body`.
    fn agent(folder: &TempDir, body: &str) -> AgentCommand {
        let path = executable_script(folder.path(), "agent", body);

        AgentCommand::new(path.display().to_string())
    }

    fn terminals(folder: &TempDir) -> LoginShellTerminals {
        let shell = fake_shell(folder);

        LoginShellTerminals::new(&shell, &folder.path().join("probe"))
    }

    fn start(folder: &TempDir, command: &AgentCommand) -> PtyTerminal {
        terminals(folder)
            .start_terminal(command, 100, 40)
            .unwrap_or_else(|reason| panic!("the program starts: {reason}"))
    }

    /// The screen once it shows `text`, or the last screen after five
    /// seconds.
    fn screen_showing(terminal: &mut PtyTerminal, text: &str) -> String {
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut screen = String::new();

        while Instant::now() < deadline && !screen.contains(text) {
            match terminal.next_screen(deadline) {
                ScreenOutput::Screen(next) => screen = next,
                ScreenOutput::Ended => break,
                ScreenOutput::Unchanged => {},
            }
        }

        screen
    }

    /// Whether a process with `pid` still exists.
    fn is_running(pid: &str) -> bool {
        Command::new("kill")
            .args(["-0", pid])
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    }

    /// The process id the program printed before "ready".
    fn printed_pid(screen: &str) -> String {
        screen
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .to_string()
    }

    #[test]
    fn the_program_runs_in_its_folder_and_screen_with_its_variables() {
        let folder = TempDir::new("terminal-start");
        let command = agent(
            &folder,
            "echo \"folder: $(pwd)\"\n\
             echo \"size: $(stty size)\"\n\
             echo \"term: $TERM\"\n\
             echo \"config: $CLAUDE_CONFIG_DIR\"\n\
             echo \"args: $*\"\n\
             echo ready\n\
             exec sleep 30",
        )
        .with_arg("--allowed-tools")
        .with_arg("")
        .with_arg("--strict-mcp-config")
        .with_env("CLAUDE_CONFIG_DIR", "/made/up/.claude-work");
        let mut terminal = start(&folder, &command);

        let screen = screen_showing(&mut terminal, "ready");
        terminal.stop();

        let probe = folder.path().join("probe");
        let probe = probe.canonicalize().unwrap_or(probe);
        assert!(screen.contains(&format!("folder: {}", probe.display())));
        assert!(screen.contains("size: 40 100"), "{screen}");
        assert!(screen.contains("term: xterm-256color"), "{screen}");
        assert!(screen.contains("config: /made/up/.claude-work"), "{screen}");
        assert!(
            screen.contains("args: --allowed-tools  --strict-mcp-config"),
            "{screen}"
        );
    }

    #[test]
    fn keys_reach_the_program_and_its_answer_shows_on_the_screen() {
        let folder = TempDir::new("terminal-keys");
        let command = agent(
            &folder,
            "echo ready\n\
             read line\n\
             echo \"got: $line\"\n\
             exec sleep 30",
        );
        let mut terminal = start(&folder, &command);
        let _ = screen_showing(&mut terminal, "ready");

        let sent = terminal.send("/usage\r");
        let screen = screen_showing(&mut terminal, "got: /usage");
        terminal.stop();

        assert_eq!(sent, Ok(()));
        assert!(screen.contains("got: /usage"), "{screen}");
    }

    #[test]
    fn a_program_that_ends_shows_its_last_screen_and_then_ended() {
        let folder = TempDir::new("terminal-end");
        let command = agent(&folder, "echo bye");
        let mut terminal = start(&folder, &command);

        let screen = screen_showing(&mut terminal, "bye");
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut output = terminal.next_screen(deadline);
        while matches!(output, ScreenOutput::Screen(_)) {
            output = terminal.next_screen(deadline);
        }

        assert!(screen.contains("bye"), "{screen}");
        assert_eq!(output, ScreenOutput::Ended);
        assert!(terminal.send("x").is_err());
        terminal.stop();
    }

    #[test]
    fn a_still_screen_is_unchanged_until_the_time_given() {
        let folder = TempDir::new("terminal-still");
        let command = agent(&folder, "echo ready\nexec sleep 30");
        let mut terminal = start(&folder, &command);
        let _ = screen_showing(&mut terminal, "ready");
        let started = Instant::now();

        let output =
            terminal.next_screen(Instant::now() + Duration::from_millis(100));
        terminal.stop();

        assert_eq!(output, ScreenOutput::Unchanged);
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn a_program_that_ignores_the_hangup_is_killed() {
        let folder = TempDir::new("terminal-kill");
        let command =
            agent(&folder, "trap '' HUP\necho \"$$ ready\"\nexec sleep 30");
        let mut terminal = start(&folder, &command);
        let pid = printed_pid(&screen_showing(&mut terminal, "ready"));
        assert!(is_running(&pid), "the program runs as {pid}");

        terminal.stop();

        assert!(!is_running(&pid));
    }

    #[test]
    fn a_dropped_terminal_stops_its_program() {
        let folder = TempDir::new("terminal-drop");
        let command = agent(&folder, "echo \"$$ ready\"\nexec sleep 30");
        let mut terminal = start(&folder, &command);
        let pid = printed_pid(&screen_showing(&mut terminal, "ready"));
        assert!(is_running(&pid), "the program runs as {pid}");

        drop(terminal);

        assert!(!is_running(&pid));
    }

    #[test]
    fn a_shell_that_is_not_there_starts_nothing() {
        let folder = TempDir::new("terminal-shell");
        let terminals = LoginShellTerminals::new(
            "/no/such/shell",
            &folder.path().join("probe"),
        );

        let result = terminals.start(&AgentCommand::new("claude"), 80, 24);

        assert_eq!(
            result.err(),
            Some(String::from("can not run the shell /no/such/shell"))
        );
    }

    #[test]
    fn the_working_folder_is_created_and_given() {
        let folder = TempDir::new("terminal-folder");
        let terminals = terminals(&folder);
        let command = agent(&folder, "exit 0");

        let started = terminals.start(&command, 80, 24);

        assert!(started.is_ok());
        assert!(folder.path().join("probe").is_dir());
        assert_eq!(terminals.working_dir(), folder.path().join("probe"));
    }
}
