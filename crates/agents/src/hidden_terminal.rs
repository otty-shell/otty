//! The agent's own program, started in a terminal no one sees, for a source
//! that reads what the program shows on its screen, such as Claude Code's
//! `/usage` panel.
//!
//! The crate does not start programs itself: the app gives a
//! [`StartTerminal`] that starts them in a pseudo-terminal and keeps their
//! screen with OTTY's own terminal engine, and the tests give one with
//! screens typed by hand, so no test starts a real agent.

use std::path::PathBuf;
use std::time::Instant;

use crate::command::AgentCommand;

/// What the screen of a running program shows next, or why it shows
/// nothing new.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScreenOutput {
    /// The whole screen as text, one line of text for each row, after it
    /// changed.
    Screen(String),
    /// The screen did not change before the time limit.
    Unchanged,
    /// The program ended.
    Ended,
}

/// Starts an agent's program in a hidden terminal.
///
/// It is a trait because it crosses into the programs of the computer: the
/// app starts them through the user's login shell in a pseudo-terminal, and
/// the tests answer with screens typed by hand.
#[cfg_attr(test, mockall::automock)]
pub trait StartTerminal: Send + Sync {
    /// The folder every program starts in: a folder of OTTY's own, so a
    /// program that keeps a record of where it ran never records a real
    /// project.
    fn working_dir(&self) -> PathBuf;

    /// Starts `command` in a terminal of `columns` by `rows` in
    /// [`working_dir`](StartTerminal::working_dir). The error says why it
    /// could not start, such as a program that is not found.
    fn start(
        &self,
        command: &AgentCommand,
        columns: u16,
        rows: u16,
    ) -> Result<Box<dyn RunningTerminal>, String>;
}

/// A program started by [`StartTerminal`]: keys go to it, its screen comes
/// back as text.
///
/// The caller calls [`stop`](RunningTerminal::stop) when it is done,
/// whatever the program showed.
#[cfg_attr(test, mockall::automock)]
pub trait RunningTerminal {
    /// Types `keys` into the program, such as `"/usage\r"` or the Escape
    /// key `"\u{1b}"`. The error says why they could not be typed, such as
    /// a program that has ended.
    fn send(&mut self, keys: &str) -> Result<(), String>;

    /// The screen once it changed and then stayed still for a moment,
    /// waiting for it until `until` at the latest.
    fn next_screen(&mut self, until: Instant) -> ScreenOutput;

    /// Stops the program: asks it to stop, and kills it when it does not.
    /// The program never keeps running after this.
    fn stop(&mut self);
}
