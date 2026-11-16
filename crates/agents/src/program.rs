//! The agent's own program, started for a source that talks to it one line
//! at a time, such as `codex app-server`.
//!
//! The crate does not start programs itself: the app gives a
//! [`StartProgram`] that starts them the way the agent tab does, and the
//! tests give one with canned lines, so no test starts a real agent.

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
    ) -> Result<Box<dyn RunningProgram>, String>;
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
    fn send_line(&mut self, line: &str) -> Result<(), String>;

    /// The next line the program writes, waiting for it until `until` at
    /// the latest.
    fn next_line(&mut self, until: Instant) -> ProgramOutput;

    /// Stops the program: closes its input, asks it to stop, and kills it
    /// when it does not. The program never keeps running after this.
    fn stop(&mut self);
}
