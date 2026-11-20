//! The programs and hidden terminals of plugins (spec
//! 2026-11-19-agent-plugins, issue 06): `otty-plugin-host` starts none
//! itself, so the app gives it the same login shell programs and hidden
//! terminals the in-core sources use. The host checks the permissions
//! and gives the time and output limits; a program run once is run here
//! and killed when it runs past them.

use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use otty_agents::{AgentCommand, ProgramOutput};
use otty_app_shell::{exec_script, login_shell_args};
use otty_plugin_host::{
    LineOutput, ProgramCommand, RunOutput, RunningProgram, RunningTerminal,
    StartProgram, StartTerminal,
};

use crate::agent_program::{LoginShellPrograms, ShellProgram};
use crate::hidden_terminal::LoginShellTerminals;
#[cfg(unix)]
use crate::hidden_terminal::PtyTerminal;

/// How often a program run once is asked whether it ended.
const POLL: Duration = Duration::from_millis(20);

impl StartProgram for LoginShellPrograms {
    fn run(
        &self,
        command: &ProgramCommand,
        until: Instant,
        max_output: usize,
    ) -> Result<RunOutput, otty_plugin_host::ProgramError> {
        run_once(self.shell(), command, until, max_output)
    }

    fn start(
        &self,
        command: &ProgramCommand,
    ) -> Result<Box<dyn RunningProgram>, otty_plugin_host::ProgramError> {
        let program = self
            .start_program(&agent_command(command))
            .map_err(|error| host_error(&error))?;

        Ok(Box::new(program))
    }
}

impl RunningProgram for ShellProgram {
    fn send_line(
        &mut self,
        line: &str,
    ) -> Result<(), otty_plugin_host::ProgramError> {
        otty_agents::RunningProgram::send_line(self, line)
            .map_err(|error| host_error(&error))
    }

    fn next_line(&mut self, until: Instant) -> LineOutput {
        match otty_agents::RunningProgram::next_line(self, until) {
            ProgramOutput::Line(line) => LineOutput::Line(line),
            ProgramOutput::TimedOut => LineOutput::TimedOut,
            ProgramOutput::Ended => LineOutput::Ended,
        }
    }

    fn stop(&mut self) {
        otty_agents::RunningProgram::stop(self);
    }
}

impl StartTerminal for LoginShellTerminals {
    #[cfg(unix)]
    fn start(
        &self,
        command: &ProgramCommand,
        cols: u16,
        rows: u16,
    ) -> Result<Box<dyn RunningTerminal>, otty_plugin_host::ProgramError> {
        let terminal = self
            .start_terminal(&agent_command(command), cols, rows)
            .map_err(|error| host_error(&error))?;

        Ok(Box::new(terminal))
    }

    /// Without a pseudo-terminal there is no hidden terminal.
    #[cfg(not(unix))]
    fn start(
        &self,
        _command: &ProgramCommand,
        _cols: u16,
        _rows: u16,
    ) -> Result<Box<dyn RunningTerminal>, otty_plugin_host::ProgramError> {
        Err(host_error(&otty_agents::ProgramError::NeedsUnix))
    }
}

#[cfg(unix)]
impl RunningTerminal for PtyTerminal {
    fn send_keys(
        &mut self,
        keys: &str,
    ) -> Result<(), otty_plugin_host::ProgramError> {
        otty_agents::RunningTerminal::send(self, keys)
            .map_err(|error| host_error(&error))
    }

    fn next_screen(
        &mut self,
        until: Instant,
    ) -> otty_plugin_host::ScreenOutput {
        match otty_agents::RunningTerminal::next_screen(self, until) {
            otty_agents::ScreenOutput::Screen(screen) => {
                otty_plugin_host::ScreenOutput::Screen(screen)
            },
            otty_agents::ScreenOutput::Unchanged => {
                otty_plugin_host::ScreenOutput::Unchanged
            },
            otty_agents::ScreenOutput::Ended => {
                otty_plugin_host::ScreenOutput::Ended
            },
        }
    }

    fn stop(&mut self) {
        otty_agents::RunningTerminal::stop(self);
    }
}

/// What a program run once writes to its two outputs, each read to its
/// end, or to one byte past the limit, on a thread of its own.
struct Outputs {
    stdout: Reading,
    stderr: Reading,
}

impl Outputs {
    fn new(stdout: Receiver<Vec<u8>>, stderr: Receiver<Vec<u8>>) -> Self {
        Self {
            stdout: Reading::Waiting(stdout),
            stderr: Reading::Waiting(stderr),
        }
    }

    /// Takes the outputs that were read to their end or their limit.
    fn take_ready(&mut self) {
        self.stdout.take_ready();
        self.stderr.take_ready();
    }

    /// How many bytes were read from both outputs so far.
    fn written_bytes(&self) -> usize {
        self.stdout.len() + self.stderr.len()
    }

    /// Both outputs once read, waiting for them until `until`; `None`
    /// when an output stays open past it, such as one a program left to
    /// a child of its own.
    fn finish(self, until: Instant) -> Option<(Vec<u8>, Vec<u8>)> {
        Some((self.stdout.finish(until)?, self.stderr.finish(until)?))
    }
}

/// One output of a program run once: still read on its thread, or read.
enum Reading {
    Waiting(Receiver<Vec<u8>>),
    Read(Vec<u8>),
}

impl Reading {
    fn len(&self) -> usize {
        match self {
            Reading::Read(bytes) => bytes.len(),
            Reading::Waiting(_) => 0,
        }
    }

    fn take_ready(&mut self) {
        if let Reading::Waiting(output) = self
            && let Ok(bytes) = output.try_recv()
        {
            *self = Reading::Read(bytes);
        }
    }

    fn finish(self, until: Instant) -> Option<Vec<u8>> {
        match self {
            Reading::Read(bytes) => Some(bytes),
            Reading::Waiting(output) => {
                let left = until.saturating_duration_since(Instant::now());
                match output.recv_timeout(left.max(POLL)) {
                    Ok(bytes) => Some(bytes),
                    Err(RecvTimeoutError::Timeout) => None,
                    Err(RecvTimeoutError::Disconnected) => Some(Vec::new()),
                }
            },
        }
    }
}

/// Runs `command` once through the user's login shell at `shell`, which
/// replaces itself with the program, with no input. A program still
/// running at `until`, or one that writes more than `max_output` bytes in
/// all, is killed. Only the program is ever logged, never what it wrote.
fn run_once(
    shell: &str,
    command: &ProgramCommand,
    until: Instant,
    max_output: usize,
) -> Result<RunOutput, otty_plugin_host::ProgramError> {
    let script = exec_script(command.program(), command.args());
    let mut child = Command::new(shell)
        .args(login_shell_args(&script))
        .envs(command.env().iter().map(|(name, value)| (name, value)))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            otty_plugin_host::ProgramError::NotStarted(format!(
                "can not run the shell {shell}: {error}"
            ))
        })?;
    let stdout = child
        .stdout
        .take()
        .map(|output| read_capped(output, max_output));
    let stderr = child
        .stderr
        .take()
        .map(|output| read_capped(output, max_output));
    let (Some(Ok(stdout)), Some(Ok(stderr))) = (stdout, stderr) else {
        stop(&mut child);
        return Err(otty_plugin_host::ProgramError::NotStarted(
            "can not read its output".to_string(),
        ));
    };

    let mut outputs = Outputs::new(stdout, stderr);
    let status = loop {
        outputs.take_ready();
        if outputs.written_bytes() > max_output {
            stop(&mut child);
            return Err(otty_plugin_host::ProgramError::TooMuchOutput(
                max_output,
            ));
        }

        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < until => std::thread::sleep(POLL),
            _ => {
                log::warn!(
                    "the program {} of a plugin ran past its time; killing it",
                    command.program()
                );
                stop(&mut child);
                return Err(otty_plugin_host::ProgramError::TimedOut);
            },
        }
    };

    let (stdout, stderr) = outputs
        .finish(until)
        .ok_or(otty_plugin_host::ProgramError::TimedOut)?;
    if stdout.len() + stderr.len() > max_output {
        return Err(otty_plugin_host::ProgramError::TooMuchOutput(max_output));
    }

    Ok(RunOutput::new(status.code(), stdout, stderr))
}

/// Reads `output` to its end, or to one byte past `max_output`, on a
/// thread of its own; the bytes arrive on the returned channel.
fn read_capped(
    output: impl Read + Send + 'static,
    max_output: usize,
) -> std::io::Result<Receiver<Vec<u8>>> {
    let (sender, bytes) = mpsc::channel();
    let limit = u64::try_from(max_output)
        .unwrap_or(u64::MAX)
        .saturating_add(1);

    std::thread::Builder::new()
        .name(String::from("plugin-program-output"))
        .spawn(move || {
            let mut read = Vec::new();
            let _ = output.take(limit).read_to_end(&mut read);
            let _ = sender.send(read);
        })?;

    Ok(bytes)
}

/// Kills `child` and reaps it, so nothing of it is left.
fn stop(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

/// `command` as the in-core sources start it.
fn agent_command(command: &ProgramCommand) -> AgentCommand {
    let with_args = command
        .args()
        .iter()
        .fold(AgentCommand::new(command.program()), |agent, arg| {
            agent.with_arg(arg)
        });

    command
        .env()
        .iter()
        .fold(with_args, |agent, (name, value)| {
            agent.with_env(name, value)
        })
}

/// `error` of the in-core starters in the words of the host.
fn host_error(
    error: &otty_agents::ProgramError,
) -> otty_plugin_host::ProgramError {
    match error {
        otty_agents::ProgramError::Ended => {
            otty_plugin_host::ProgramError::Ended
        },
        otty_agents::ProgramError::NotWritten(reason) => {
            otty_plugin_host::ProgramError::NotWritten(reason.clone())
        },
        _ => otty_plugin_host::ProgramError::NotStarted(error.to_string()),
    }
}

#[cfg(all(test, unix))]
mod tests {
    use otty_app_testing::temp_dir::TempDir;
    use otty_plugin_host::{ProgramError, ScreenOutput};

    use super::*;
    use crate::testing::{executable_script, fake_shell, is_running};

    const MIB: usize = 1024 * 1024;

    fn soon() -> Instant {
        Instant::now() + Duration::from_secs(5)
    }

    /// The command of a made-up program in `folder` that runs `body`.
    fn stand_in(folder: &TempDir, body: &str) -> ProgramCommand {
        let path = executable_script(folder.path(), "stand-in", body);

        ProgramCommand::new(&path.display().to_string(), &[], &[])
    }

    fn with(
        command: &ProgramCommand,
        args: &[&str],
        env: &[(&str, &str)],
    ) -> ProgramCommand {
        let args: Vec<String> =
            args.iter().map(|arg| arg.to_string()).collect();
        let env: Vec<(String, String)> = env
            .iter()
            .map(|(name, value)| (name.to_string(), value.to_string()))
            .collect();

        ProgramCommand::new(command.program(), &args, &env)
    }

    #[test]
    fn a_run_gives_both_outputs_the_status_and_gets_its_variables() {
        let folder = TempDir::new("plugin-run");
        let programs = LoginShellPrograms::new(&fake_shell(&folder));
        let command = with(
            &stand_in(
                &folder,
                "echo \"args: $*\"\n\
                 echo \"home: $STAND_IN_HOME\"\n\
                 echo oops >&2\n\
                 exit 3",
            ),
            &["--version", "two words"],
            &[("STAND_IN_HOME", "/made/up/.stand-in")],
        );

        let output = programs.run(&command, soon(), MIB).expect("a run");

        assert_eq!(output.status(), Some(3));
        assert_eq!(
            output.stdout(),
            b"args: --version two words\nhome: /made/up/.stand-in\n"
        );
        assert_eq!(output.stderr(), b"oops\n");
    }

    #[test]
    fn a_program_that_never_ends_is_killed_at_its_time() {
        let folder = TempDir::new("plugin-run-forever");
        let pid_file = folder.path().join("pid");
        let programs = LoginShellPrograms::new(&fake_shell(&folder));
        let command = stand_in(
            &folder,
            &format!("echo $$ > '{}'\nexec sleep 30", pid_file.display()),
        );
        let started = Instant::now();

        let output = programs.run(
            &command,
            Instant::now() + Duration::from_millis(500),
            MIB,
        );

        let pid = std::fs::read_to_string(&pid_file).unwrap_or_default();
        assert_eq!(output, Err(ProgramError::TimedOut));
        assert!(started.elapsed() < Duration::from_secs(5));
        assert!(!is_running(pid.trim()), "the program {pid} was killed");
    }

    #[test]
    fn a_program_that_writes_too_much_is_killed() {
        let folder = TempDir::new("plugin-run-loud");
        let programs = LoginShellPrograms::new(&fake_shell(&folder));
        let command =
            stand_in(&folder, "while :; do echo 0123456789abcdef; done");
        let started = Instant::now();

        let output = programs.run(&command, soon(), 1024);

        assert_eq!(output, Err(ProgramError::TooMuchOutput(1024)));
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn a_shell_that_is_not_there_runs_nothing() {
        let programs = LoginShellPrograms::new("/no/such/shell");
        let command = ProgramCommand::new("stand-in", &[], &[]);

        let output = programs.run(&command, soon(), MIB);

        assert!(
            matches!(&output, Err(ProgramError::NotStarted(reason)) if reason.contains("/no/such/shell")),
            "{output:?}"
        );
    }

    #[test]
    fn a_talk_goes_line_by_line() {
        let folder = TempDir::new("plugin-talk");
        let programs = LoginShellPrograms::new(&fake_shell(&folder));
        let command = stand_in(&folder, "read line\necho \"got: $line\"");

        let mut talk =
            StartProgram::start(&programs, &command).expect("a talk");
        let sent = talk.send_line("ping");
        let answers = [talk.next_line(soon()), talk.next_line(soon())];
        RunningProgram::stop(talk.as_mut());

        assert_eq!(sent, Ok(()));
        assert_eq!(
            answers,
            [LineOutput::Line("got: ping".to_string()), LineOutput::Ended]
        );
    }

    #[test]
    fn a_hidden_terminal_shows_its_screen_and_takes_keys() {
        let folder = TempDir::new("plugin-terminal");
        let terminals = LoginShellTerminals::new(
            &fake_shell(&folder),
            &folder.path().join("probe"),
        );
        let command = stand_in(
            &folder,
            "echo ready\nread line\necho \"got: $line\"\nexec sleep 30",
        );

        let mut terminal = StartTerminal::start(&terminals, &command, 80, 24)
            .expect("a terminal");
        let ready = screen_showing(terminal.as_mut(), "ready");
        let sent = terminal.send_keys("/usage\r");
        let answered = screen_showing(terminal.as_mut(), "got: /usage");
        RunningTerminal::stop(terminal.as_mut());

        assert!(ready.contains("ready"), "{ready}");
        assert_eq!(sent, Ok(()));
        assert!(answered.contains("got: /usage"), "{answered}");
    }

    /// The screen once it shows `text`, or the last screen after five
    /// seconds.
    fn screen_showing(
        terminal: &mut dyn RunningTerminal,
        text: &str,
    ) -> String {
        let deadline = soon();
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
}
