//! The I/O of the modes: reading and writing the pinned modes file.
//!
//! The modes feature says when the pinned modes are saved; this crate does
//! it. The read at start gives the pinned modes right away, before the first
//! frame; the save is a [`Task`](iced::Task) that ends in a
//! [`ModesIntent`](otty_app_modes::ModesIntent). The binary only calls them
//! and wraps the intent into its message.

mod pinned_file;

pub use pinned_file::{load_at_start, save};
