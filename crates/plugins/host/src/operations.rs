//! The host operations a plugin imports, `otty:host`: one binding per
//! version, and their implementation for a call's [`CallState`].
//!
//! [`CallState`]: crate::CallState

mod http;
mod keyring;
mod keys;
mod lines;
mod listing;
mod programs;
mod real_path;
mod remove;
mod serve;
mod small_file;
mod sqlite_query;
mod terminal;
pub mod v0_1;
mod variables;

pub(crate) use crate::operations::lines::ZstdCursor;
pub(crate) use crate::operations::listing::list as list_files;
