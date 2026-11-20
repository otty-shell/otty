//! The host operations a plugin imports, `otty:host`: one binding per
//! version, and their implementation for a call's [`CallState`].
//!
//! [`CallState`]: crate::CallState

mod http;
mod keyring;
mod keys;
mod programs;
mod remove;
mod serve;
mod terminal;
pub mod v0_1;
