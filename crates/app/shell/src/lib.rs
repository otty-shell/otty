//! The shell new terminals start and the OTTY shell integration.
//!
//! [`ShellSession`] is the session new terminal tabs start: the user's
//! shell, in the home folder. The integration writes the bundled zsh and
//! bash scripts to a folder and adds them to a shell's launch options, so
//! the shell reports its commands and current folder to the terminal. The
//! binary writes the files at start and when the shell changes in Settings;
//! quick launch adds them to a launch of a plain zsh or bash.

mod integration;
mod session;

pub use integration::{integrated_shell_session, with_shell_integration};
pub use session::{ShellSession, shell_session_options};
