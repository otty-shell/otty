//! The programs a plugin starts in a terminal nobody sees, to read what
//! they show on their screen, as the app starts them for the host.
//!
//! The host does not start terminals itself: the app gives a
//! [`StartTerminal`] that starts the program in a pseudo-terminal, in a
//! working folder of OTTY's own, and keeps its screen with OTTY's own
//! terminal engine. The host checks the permissions of the called part
//! first and keeps the time limit; the tests give a mock, so no test
//! starts a real agent.

use std::path::PathBuf;
use std::time::Instant;

use crate::programs::{ProgramCommand, ProgramError};

/// What the screen of a running program shows next, or why it shows
/// nothing new.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScreenOutput {
    /// The visible text of the screen, one line of text for each row,
    /// after it changed.
    Screen(String),
    /// The screen did not change before the time limit.
    Unchanged,
    /// The program ended.
    Ended,
}

/// Starts the programs plugins read the screen of.
///
/// It is a trait because it crosses into the programs of the computer and
/// into OTTY's terminal engine, which the host must not depend on: the app
/// starts them through the user's login shell in a pseudo-terminal, and
/// the tests answer with mocks.
#[cfg_attr(test, mockall::automock)]
pub trait StartTerminal: Send + Sync {
    /// The working folder of OTTY's own that every program starts in.
    fn working_folder(&self) -> PathBuf;

    /// Starts `command` in a terminal of `cols` by `rows` cells that
    /// nobody sees, in a working folder of OTTY's own, so a program that
    /// keeps a record of where it ran never records a real project.
    fn start(
        &self,
        command: &ProgramCommand,
        cols: u16,
        rows: u16,
    ) -> Result<Box<dyn RunningTerminal>, ProgramError>;
}

/// A program started by [`StartTerminal`]: keys go to it, its screen
/// comes back as text.
///
/// The host calls [`stop`](RunningTerminal::stop) when the call that
/// started it ends, whatever the program showed.
#[cfg_attr(test, mockall::automock)]
pub trait RunningTerminal: Send {
    /// Types `keys` into the program, such as `"/usage\r"`.
    fn send_keys(&mut self, keys: &str) -> Result<(), ProgramError>;

    /// The screen once it changed and then stayed still for a moment,
    /// waiting for it until `until` at the latest.
    fn next_screen(&mut self, until: Instant) -> ScreenOutput;

    /// Stops the program: asks it to stop, and kills it when it does not.
    /// The program never keeps running after this.
    fn stop(&mut self);
}
