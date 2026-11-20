//! An agent's program started for a source that talks to it, such as
//! `codex app-server`: through the user's login shell, the way the agent
//! tab starts it, with its input and output open to the source.
//!
//! The program is never left running: [`RunningProgram::stop`], called by
//! the source and again when the program is dropped, closes its input,
//! asks it to stop, and kills it when it does not.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

#[cfg(unix)]
use nix::sys::signal::{Signal, kill};
#[cfg(unix)]
use nix::unistd::Pid;
use otty_agents::{
    AgentCommand, ProgramError, ProgramOutput, RunningProgram, StartProgram,
};
use otty_app_shell::{exec_script, login_shell_args};

use crate::login_shell::wait_with_limit;

/// How long a program may take to end after its input is closed, and
/// again after it is asked to stop, before it is killed.
const STOP_GRACE: Duration = Duration::from_secs(1);

/// Starts agent programs through the user's login shell.
pub(crate) struct LoginShellPrograms {
    shell: String,
    grace: Duration,
}

impl LoginShellPrograms {
    /// The user's shell the programs are started through.
    pub(crate) fn shell(&self) -> &str {
        &self.shell
    }

    /// Programs started through the user's shell at `shell`, such as
    /// `/bin/zsh`.
    pub(crate) fn new(shell: &str) -> Self {
        Self::with_grace(shell, STOP_GRACE)
    }

    fn with_grace(shell: &str, grace: Duration) -> Self {
        Self {
            shell: shell.to_string(),
            grace,
        }
    }

    /// Starts `command` through the user's login shell, which replaces
    /// itself with the program. A program the shell does not find ends at
    /// once.
    pub(crate) fn start_program(
        &self,
        command: &AgentCommand,
    ) -> Result<ShellProgram, ProgramError> {
        let script = exec_script(command.program(), command.args());
        let mut child = Command::new(&self.shell)
            .args(login_shell_args(&script))
            .envs(command.env().iter().map(|(name, value)| (name, value)))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|err| ProgramError::ShellNotRun {
                shell: self.shell.clone(),
                detail: err.to_string(),
            })?;
        let input = child.stdin.take();
        let output = child.stdout.take();

        let (sender, lines) = mpsc::channel();
        let reader = output.map(|output| {
            std::thread::Builder::new()
                .name(String::from("agent-program-output"))
                .spawn(move || read_lines(output, &sender))
        });
        let program = ShellProgram {
            child,
            input,
            lines,
            grace: self.grace,
            is_stopped: false,
        };

        match reader {
            Some(Ok(_)) => Ok(program),
            Some(Err(err)) => Err(ProgramError::OutputNotRead(err.to_string())),
            None => Err(ProgramError::NoOutput),
        }
    }
}

impl StartProgram for LoginShellPrograms {
    fn start(
        &self,
        command: &AgentCommand,
    ) -> Result<Box<dyn RunningProgram>, ProgramError> {
        let program = self.start_program(command)?;

        Ok(Box::new(program))
    }
}

/// A program started through the user's login shell.
///
/// A thread reads its output line by line, so a line can be waited for
/// with a time limit.
pub(crate) struct ShellProgram {
    child: Child,
    input: Option<ChildStdin>,
    lines: Receiver<String>,
    grace: Duration,
    is_stopped: bool,
}

impl ShellProgram {
    /// Whether the program ended within `time`. An ended program is
    /// reaped, so it leaves nothing behind.
    fn ends_within(&mut self, time: Duration) -> bool {
        wait_with_limit(&mut self.child, time).is_some()
    }

    /// Asks the program to stop with `SIGTERM`.
    #[cfg(unix)]
    fn ask_to_stop(&self) {
        let pid = self.child.id();
        let Ok(raw_pid) = i32::try_from(pid) else {
            log::warn!("the agent program {pid} has no process id to signal");
            return;
        };

        if let Err(err) = kill(Pid::from_raw(raw_pid), Signal::SIGTERM) {
            log::warn!("cannot ask the agent program {pid} to stop: {err}");
        }
    }

    /// Asks the program to stop; without `SIGTERM` there is no way to ask,
    /// so it is killed after the grace time.
    #[cfg(not(unix))]
    fn ask_to_stop(&self) {}
}

#[cfg(test)]
impl ShellProgram {
    /// Whether the program has ended.
    fn has_ended(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(Some(_)))
    }
}

impl RunningProgram for ShellProgram {
    fn send_line(&mut self, line: &str) -> Result<(), ProgramError> {
        let Some(input) = self.input.as_mut() else {
            return Err(ProgramError::Ended);
        };

        input
            .write_all(line.as_bytes())
            .and_then(|()| input.write_all(b"\n"))
            .and_then(|()| input.flush())
            .map_err(|err| ProgramError::NotWritten(err.to_string()))
    }

    /// The time limit is kept even while the program writes without a
    /// pause.
    fn next_line(&mut self, until: Instant) -> ProgramOutput {
        let left = until.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return ProgramOutput::TimedOut;
        }

        match self.lines.recv_timeout(left) {
            Ok(line) => ProgramOutput::Line(line),
            Err(RecvTimeoutError::Timeout) => ProgramOutput::TimedOut,
            Err(RecvTimeoutError::Disconnected) => ProgramOutput::Ended,
        }
    }

    fn stop(&mut self) {
        if self.is_stopped {
            return;
        }
        self.is_stopped = true;

        // A program that serves its input, such as `codex app-server`,
        // ends when the input closes.
        self.input = None;
        if self.ends_within(self.grace) {
            return;
        }

        self.ask_to_stop();
        if self.ends_within(self.grace) {
            return;
        }

        let pid = self.child.id();
        log::warn!("the agent program {pid} did not stop; killing it");
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for ShellProgram {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Sends each line of `output` to `lines` until the output closes or no
/// one waits for the lines any more. A line that is not UTF-8 is sent
/// with its bad bytes replaced.
fn read_lines(output: ChildStdout, lines: &Sender<String>) {
    let mut output = BufReader::new(output);
    let mut line = Vec::new();

    while matches!(output.read_until(b'\n', &mut line), Ok(read) if read > 0) {
        let text = String::from_utf8_lossy(&line);
        let text = text.trim_end_matches(['\n', '\r']).to_string();
        if lines.send(text).is_err() {
            return;
        }
        line.clear();
    }
}

#[cfg(all(test, unix))]
mod tests {
    use otty_app_testing::temp_dir::TempDir;

    use super::*;
    use crate::testing::{agent, fake_shell, is_running};

    /// A short grace, so a program that does not stop is killed soon.
    const SHORT_GRACE: Duration = Duration::from_millis(100);

    fn start(
        folder: &TempDir,
        command: &AgentCommand,
        grace: Duration,
    ) -> ShellProgram {
        let shell = fake_shell(folder);

        LoginShellPrograms::with_grace(&shell, grace)
            .start_program(command)
            .unwrap_or_else(|reason| panic!("the program starts: {reason}"))
    }

    fn soon() -> Instant {
        Instant::now() + Duration::from_secs(5)
    }

    fn line(text: &str) -> ProgramOutput {
        ProgramOutput::Line(String::from(text))
    }

    #[test]
    fn the_program_gets_its_arguments_variables_and_lines() {
        let folder = TempDir::new("program-talk");
        let command = agent(
            &folder,
            "read request\n\
             echo \"args: $*\"\n\
             echo \"home: $CODEX_HOME\"\n\
             echo \"got: $request\"",
        )
        .with_arg("-s")
        .with_arg("read only")
        .with_env("CODEX_HOME", "/made/up/.codex-work");
        let mut program = start(&folder, &command, STOP_GRACE);

        let sent = program.send_line("{\"id\":1}");
        let answers = [
            program.next_line(soon()),
            program.next_line(soon()),
            program.next_line(soon()),
            program.next_line(soon()),
        ];
        program.stop();

        assert_eq!(sent, Ok(()));
        assert_eq!(
            answers,
            [
                line("args: -s read only"),
                line("home: /made/up/.codex-work"),
                line("got: {\"id\":1}"),
                ProgramOutput::Ended,
            ]
        );
        assert!(program.has_ended());
    }

    #[test]
    fn a_program_that_ends_when_its_input_closes_is_not_killed() {
        let folder = TempDir::new("program-input");
        let marker = folder.path().join("ended");
        let command = agent(
            &folder,
            &format!(
                "trap 'echo asked >> \"{}\"' TERM\n\
                 while read line; do :; done\n\
                 echo input-closed >> \"{}\"",
                marker.display(),
                marker.display()
            ),
        );
        let mut program = start(&folder, &command, Duration::from_secs(5));

        program.stop();

        assert!(program.has_ended());
        let ended = std::fs::read_to_string(&marker).unwrap_or_default();
        assert_eq!(ended.trim(), "input-closed");
    }

    #[test]
    fn a_program_that_ignores_its_input_is_asked_to_stop() {
        let folder = TempDir::new("program-term");
        let marker = folder.path().join("asked");
        let command = agent(
            &folder,
            &format!(
                "trap 'echo asked > \"{}\"; exit 0' TERM\n\
                 while :; do sleep 0.05; done",
                marker.display()
            ),
        );
        let mut program = start(&folder, &command, Duration::from_secs(2));

        program.stop();

        assert!(program.has_ended());
        let asked = std::fs::read_to_string(&marker).unwrap_or_default();
        assert_eq!(asked.trim(), "asked");
    }

    #[test]
    fn a_program_that_will_not_stop_is_killed() {
        let folder = TempDir::new("program-kill");
        let command = agent(&folder, "trap '' TERM\nexec sleep 30");
        let mut program = start(&folder, &command, SHORT_GRACE);
        let started = Instant::now();

        program.stop();

        assert!(program.has_ended());
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn a_program_that_writes_nothing_times_out() {
        let folder = TempDir::new("program-silent");
        let command = agent(&folder, "exec sleep 30");
        let mut program = start(&folder, &command, SHORT_GRACE);
        let started = Instant::now();

        let output =
            program.next_line(Instant::now() + Duration::from_millis(100));
        program.stop();

        assert_eq!(output, ProgramOutput::TimedOut);
        assert!(started.elapsed() < Duration::from_secs(5));
        assert!(program.has_ended());
    }

    #[test]
    fn a_dropped_program_is_stopped() {
        let folder = TempDir::new("program-drop");
        let command = agent(&folder, "echo $$\nexec sleep 30");
        let mut program = start(&folder, &command, SHORT_GRACE);

        let ProgramOutput::Line(pid) = program.next_line(soon()) else {
            panic!("the program tells its process id");
        };
        assert!(is_running(&pid));
        drop(program);

        assert!(!is_running(&pid));
    }

    #[test]
    fn a_program_the_shell_does_not_find_ends_at_once() {
        let folder = TempDir::new("program-missing");
        let command = AgentCommand::new("otty-no-such-agent-program");
        let mut program = start(&folder, &command, SHORT_GRACE);

        let output = program.next_line(soon());
        program.stop();

        assert_eq!(output, ProgramOutput::Ended);
        assert!(program.send_line("{}").is_err());
    }

    #[test]
    fn a_shell_that_can_not_start_starts_nothing() {
        let programs = LoginShellPrograms::new("/no/such/shell");

        let result = programs.start(&AgentCommand::new("codex"));

        let reason = result.err().map(|error| error.to_string());
        let reason = reason.unwrap_or_default();
        assert!(
            reason.starts_with("can not run the shell /no/such/shell"),
            "{reason}"
        );
    }
}
