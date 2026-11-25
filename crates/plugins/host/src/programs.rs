//! The programs a plugin runs once or talks to line by line, as the app
//! starts them for the host.
//!
//! The host does not start programs itself: the app gives a
//! [`StartProgram`] that starts them the way the agent tab does, through
//! the user's login shell, so a program is found as in the user's
//! terminal. The host checks the permissions of the called part first and
//! keeps the time and output limits; the tests give a mock, so no test
//! starts a real agent.

use std::time::Instant;

/// A program a plugin asked for, once the permissions of the called part
/// allowed it: the program, its arguments and the variables the plugin
/// sets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramCommand {
    program: String,
    args: Vec<String>,
    env: Vec<(String, String)>,
}

impl ProgramCommand {
    /// The program: a name to look up as the user's shell finds it, or the
    /// full path OTTY found it at among the program places of its part.
    pub fn program(&self) -> &str {
        &self.program
    }

    /// The arguments of the program.
    pub fn args(&self) -> &[String] {
        &self.args
    }

    /// The variables the plugin sets, as names and values. The program
    /// gets them on top of the environment the user's shell gives it.
    pub fn env(&self) -> &[(String, String)] {
        &self.env
    }

    /// The command that runs `program` with `args` and the variables
    /// `env`.
    ///
    /// ```
    /// use otty_plugin_host::ProgramCommand;
    ///
    /// let command = ProgramCommand::new(
    ///     "codex",
    ///     &["--version".to_string()],
    ///     &[(
    ///         "CODEX_HOME".to_string(),
    ///         "/home/user/.codex-work".to_string(),
    ///     )],
    /// );
    ///
    /// assert_eq!(command.program(), "codex");
    /// assert_eq!(command.args(), ["--version"]);
    /// ```
    pub fn new(
        program: &str,
        args: &[String],
        env: &[(String, String)],
    ) -> Self {
        Self {
            program: program.to_string(),
            args: args.to_vec(),
            env: env.to_vec(),
        }
    }
}

/// What a program run once wrote and how it ended.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RunOutput {
    status: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

impl RunOutput {
    /// The exit status; `None` when a signal stopped the program.
    pub fn status(&self) -> Option<i32> {
        self.status
    }

    /// What the program wrote to its output.
    pub fn stdout(&self) -> &[u8] {
        &self.stdout
    }

    /// What the program wrote to its error output.
    pub fn stderr(&self) -> &[u8] {
        &self.stderr
    }

    /// The output of a program that ended with `status` after writing
    /// `stdout` and `stderr`.
    pub fn new(status: Option<i32>, stdout: Vec<u8>, stderr: Vec<u8>) -> Self {
        Self {
            status,
            stdout,
            stderr,
        }
    }

    /// How many bytes the program wrote to both outputs.
    pub fn written_bytes(&self) -> usize {
        self.stdout.len() + self.stderr.len()
    }

    /// The parts of the output: status, output and error output.
    pub fn into_parts(self) -> (Option<i32>, Vec<u8>, Vec<u8>) {
        (self.status, self.stdout, self.stderr)
    }
}

/// What a program spoken to line by line wrote next, or why it wrote
/// nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineOutput {
    /// One line it wrote, without the line end.
    Line(String),
    /// It wrote nothing more before the time limit.
    TimedOut,
    /// It closed its output: it ended, or could not start.
    Ended,
}

/// Why a program could not be started, run or written to. The text is
/// the system's own reason; it never holds what the program wrote.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProgramError {
    /// The program, or the shell that starts it, could not start.
    #[error("can not start it: {0}")]
    NotStarted(String),
    /// It ran past its time limit and was stopped.
    #[error("it ran past its time limit and was stopped")]
    TimedOut,
    /// It wrote more than the output limit and was stopped.
    #[error("it wrote more than {0} bytes and was stopped")]
    TooMuchOutput(usize),
    /// It has ended, or its input is closed.
    #[error("it has ended")]
    Ended,
    /// What was sent could not be written.
    #[error("can not write to it: {0}")]
    NotWritten(String),
}

/// Starts the programs plugins run.
///
/// It is a trait because it crosses into the programs of the computer:
/// the app starts them through the user's login shell, the way the agent
/// tab does, and the tests answer with mocks.
#[cfg_attr(test, mockall::automock)]
pub trait StartProgram: Send + Sync {
    /// Runs `command` once with no input and gives what it wrote. A
    /// program still running at `until` is killed and the run is
    /// [`ProgramError::TimedOut`]; one that writes more than `max_output`
    /// bytes in all is killed and the run is
    /// [`ProgramError::TooMuchOutput`].
    fn run(
        &self,
        command: &ProgramCommand,
        until: Instant,
        max_output: usize,
    ) -> Result<RunOutput, ProgramError>;

    /// Starts `command` with its input and its output open, to be spoken
    /// to line by line.
    fn start(
        &self,
        command: &ProgramCommand,
    ) -> Result<Box<dyn RunningProgram>, ProgramError>;
}

/// A program started by [`StartProgram::start`]: lines go to its input
/// and come from its output.
///
/// The host calls [`stop`](RunningProgram::stop) when the call that
/// started it ends, whatever the program answered.
#[cfg_attr(test, mockall::automock)]
pub trait RunningProgram: Send {
    /// Writes `line` and a line end to the program's input.
    fn send_line(&mut self, line: &str) -> Result<(), ProgramError>;

    /// The next line the program writes, waiting for it until `until` at
    /// the latest.
    fn next_line(&mut self, until: Instant) -> LineOutput;

    /// Stops the program: closes its input, asks it to stop, and kills it
    /// when it does not. The program never keeps running after this.
    fn stop(&mut self);
}
