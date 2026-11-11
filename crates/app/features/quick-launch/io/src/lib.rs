//! The I/O of quick launch: the launches file and the check before a launch
//! runs.
//!
//! The quick launch feature says when the launches are saved and which
//! launch to check; this crate does it. The read at start gives the saved
//! launches right away, before the first frame; the save and the check are
//! [`Task`](iced::Task)s that end in a
//! [`QuickLaunchIntent`](otty_app_quick_launch::QuickLaunchIntent). A launch
//! of a plain zsh or bash gets the OTTY shell integration from
//! `otty-app-shell`. The binary only calls them and wraps the intents into
//! its message.

mod launch_check;
mod launches_file;

pub use launch_check::prepare;
pub use launches_file::{load_at_start, save};
