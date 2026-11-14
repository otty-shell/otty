//! The I/O of the explorer, the Files section of the right panel.
//!
//! The explorer feature decides which folder to read, which folders to
//! watch and which files to create, move or delete; this crate does it. Each
//! folder read and each disk operation is a [`Task`](iced::Task) and the
//! file watch is a [`Subscription`](iced::Subscription), and all of them end
//! in an [`ExplorerIntent`](otty_app_explorer::ExplorerIntent). The binary
//! only calls them and wraps the intents into its message.

mod changed_folders;
mod disk_operation;
mod folders;
mod watch;

pub use disk_operation::run_disk_operation;
pub use folders::{load_pressed_folder, load_root, reload_folder};
pub use watch::watch_folders;
