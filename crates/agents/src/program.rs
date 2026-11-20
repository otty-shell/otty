//! The agent's own program, started for a source that talks to it one line
//! at a time, such as `codex app-server`.
//!
//! The crate does not start programs itself: the app gives a
//! [`StartProgram`] that starts them the way the agent tab does, and the
//! tests give one with canned lines, so no test starts a real agent.

use std::path::PathBuf;
use std::time::Instant;

use crate::command::AgentCommand;

/// What a running program wrote next, or why it wrote nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProgramOutput {
    /// One line it wrote, without the line end.
    Line(String),
    /// It wrote nothing more before the time limit.
    TimedOut,
    /// It closed its output: it ended, or could not start the agent.
    Ended,
}

/// Why an agent's program could not be started, or could not be written
/// to. It is shared by the programs a source talks to one line at a time
/// and the ones it starts in a hidden terminal.
///
/// The reasons are data the app writes in the user's language; the text
/// of `Display` is the English one, for the log. A `detail` is the
/// system's own reason, kept as it came.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProgramError {
    /// The read may start no program now: OTTY starts one only while
    /// Agents mode is on screen.
    #[error("OTTY starts it only while Agents mode is on screen")]
    NotNow,
    /// The user's shell, named here, is not a program that can run.
    #[error("can not run the shell {0}")]
    ShellNotFound(String),
    /// The user's shell could not be run.
    #[error("can not run the shell {shell}: {detail}")]
    ShellNotRun {
        /// The shell.
        shell: String,
        /// The system's own reason.
        detail: String,
    },
    /// The folder the program runs in could not be created.
    #[error("can not create the folder {}: {detail}", .folder.display())]
    FolderNotCreated {
        /// The folder.
        folder: PathBuf,
        /// The system's own reason.
        detail: String,
    },
    /// The hidden terminal could not be started; the text is the system's
    /// own reason.
    #[error("can not start the terminal: {0}")]
    TerminalNotStarted(String),
    /// The output of the program could not be read; the text is the
    /// system's own reason.
    #[error("can not read the program: {0}")]
    OutputNotRead(String),
    /// The program has no output to read.
    #[error("the program has no output")]
    NoOutput,
    /// A hidden terminal needs a Unix system.
    #[error("a hidden terminal needs a Unix system")]
    NeedsUnix,
    /// The program has ended, or its input is closed.
    #[error("the program has ended")]
    Ended,
    /// What was sent could not be written; the text is the system's own
    /// reason.
    #[error("{0}")]
    NotWritten(String),
}

/// Starts an agent's program for a source that talks to it.
///
/// It is a trait because it crosses into the programs of the computer: the
/// app starts them through the user's login shell, the way the agent tab
/// does, and the tests answer with canned lines.
#[cfg_attr(test, mockall::automock)]
pub trait StartProgram: Send + Sync {
    /// Starts `command` with its input and its output open to the caller.
    /// The error says why it could not start, such as a program that is
    /// not found.
    fn start(
        &self,
        command: &AgentCommand,
    ) -> Result<Box<dyn RunningProgram>, ProgramError>;

    /// Whether programs start now. A starter that refuses every start, as
    /// the app's while Agents mode is away, says no, so the plugin parts,
    /// which the plugin engine starts programs for, start none either.
    fn starts_now(&self) -> bool {
        true
    }
}

/// A program started by [`StartProgram`]: lines go to its input and come
/// from its output.
///
/// The caller calls [`stop`](RunningProgram::stop) when it is done, whatever
/// the program answered.
#[cfg_attr(test, mockall::automock)]
pub trait RunningProgram {
    /// Writes `line` and a line end to the program's input. The error says
    /// why it could not be written, such as a program that has ended.
    fn send_line(&mut self, line: &str) -> Result<(), ProgramError>;

    /// The next line the program writes, waiting for it until `until` at
    /// the latest.
    fn next_line(&mut self, until: Instant) -> ProgramOutput;

    /// Stops the program: closes its input, asks it to stop, and kills it
    /// when it does not. The program never keeps running after this.
    fn stop(&mut self);
}
