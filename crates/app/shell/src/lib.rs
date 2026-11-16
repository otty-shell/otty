//! The shell new terminals start and the OTTY shell integration.
//!
//! [`ShellSession`] is the session new terminal tabs start: the user's
//! shell, in the home folder or in a folder the tab asks for. The integration writes the bundled zsh and
//! bash scripts to a folder and adds them to a shell's launch options, so
//! the shell reports its commands and current folder to the terminal. The
//! binary writes the files with [`integrate`], in the background, at start
//! and when the shell changes in Settings; quick launch adds them to a
//! launch of a plain zsh or bash. A program OTTY starts by name, such as a
//! coding agent, runs through the user's login shell
//! ([`ShellSession::command_session`]), so a program installed by a version
//! manager is found.

mod integration;
mod login;
mod session;

pub use integration::{
    integrate, integrated_shell_session, with_shell_integration,
};
pub use login::{login_shell_args, quote_word};
pub use session::{ShellSession, shell_session_options};
