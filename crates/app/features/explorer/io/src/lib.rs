//! The I/O of the explorer, the Files section of the right panel.
//!
//! The explorer feature decides which folder to read and which folders to
//! watch; this crate does it. Each folder read is a [`Task`](iced::Task) and
//! the file watch is a [`Subscription`](iced::Subscription), and both end in
//! an [`ExplorerIntent`](otty_app_explorer::ExplorerIntent). The binary only
//! calls them and wraps the intents into its message.

mod changed_folders;
mod folders;
mod watch;

pub use folders::{load_pressed_folder, load_root, reload_folder};
pub use watch::watch_folders;
