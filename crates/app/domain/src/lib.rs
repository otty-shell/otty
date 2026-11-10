//! The types the OTTY app crates share: the modes, the saved launches, the
//! app commands, the shortcut names with their labels, and what one feature
//! hands to another, such as the machine of a tab.
//!
//! This crate has no iced, no I/O and no tokio, so the binary, every feature
//! and every helper crate may depend on it.

pub mod commands;
pub mod exit_behavior;
pub mod keys;
pub mod mode;
pub mod quick_launch;
pub mod shell_session;
pub mod shortcuts;
pub mod tab;
