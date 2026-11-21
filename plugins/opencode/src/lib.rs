//! The official OpenCode plugin of OTTY. Its one agent part, `opencode`,
//! reads OpenCode: its data folder (the profile), its logins, its
//! sessions, the tokens and the cost of its answers, and its statistics.
//!
//! OpenCode keeps everything in one data folder, `~/.local/share/opencode`
//! (`XDG_DATA_HOME` moves it): the logins in `auth.json` and in the
//! `credential` table of its SQLite database, `opencode.db`
//! (`OPENCODE_DB` moves it). The database holds the sessions and every
//! answer of a model with its provider, model, tokens and the cost
//! OpenCode worked out, so the money is exact.
//!
//! OTTY opens the database read-only and never migrates it, while
//! OpenCode may be writing to it at the same time. The plugin never sees
//! a key: the key columns and fields come back as handles.
//!
//! What only the user can do, the part gives as needs (`needs.rs`): log
//! in, and log in again when a provider part refused a login OpenCode
//! keeps. The texts come in every language of OTTY, so the plugin exports
//! the language receiver.

// A `ReadError` may hold a need, a record the contract can not box.
#![allow(clippy::result_large_err)]

mod answers;
mod database;
mod glob;
mod location;
mod logins;
mod message;
mod needs;
mod opencode;
mod plan;
mod provider_ids;
mod sessions;
mod stats;
#[cfg(test)]
mod testing;

use otty_plugin_sdk::Language;

/// The plugin: one type serves the agent part its manifest lists.
pub struct OpenCode;

impl Language for OpenCode {
    /// The texts of the needs come in every language of OTTY.
    fn language_changed(_tag: &str) -> bool {
        true
    }
}

otty_plugin_sdk::export!(OpenCode with agent, language);
