//! The I/O of the Settings dialog: reading and writing the settings file.
//!
//! The settings feature says when the file is read or written; this crate
//! does it. The read at start gives the saved settings right away, before
//! the first frame; the reads and writes of the open dialog are
//! [`Task`](iced::Task)s that end in a
//! [`SettingsIntent`](otty_app_settings::SettingsIntent). The binary only
//! calls them and wraps the intents into its message.

mod file;

pub use file::{read, read_at_start, write};
