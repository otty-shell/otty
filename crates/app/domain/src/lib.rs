//! The types the OTTY app crates share: the modes, the saved launches, the
//! app commands, the shortcut names with their labels, and what one feature
//! hands to another, such as the machine of a tab or the parts of plugins
//! that are off; and how a folder is
//! written: short, as a terminal prompt writes it, by its name, and with `~`
//! for the home folder; and the order of text in a table, A to Z whatever
//! the case.
//!
//! This crate has no iced, no I/O and no tokio, so the binary, every feature
//! and every helper crate may depend on it.

pub mod commands;
pub mod exit_behavior;
pub mod folder_path;
pub mod keys;
pub mod mode;
pub mod plugins;
pub mod quick_launch;
pub mod shell_session;
pub mod shortcuts;
pub mod tab;
pub mod text_order;
