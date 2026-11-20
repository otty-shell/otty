//! The host operations a plugin imports, `otty:host`: one binding per
//! version, and their implementation for a call's [`CallState`].
//!
//! [`CallState`]: crate::CallState

mod serve;
pub mod v0_1;
