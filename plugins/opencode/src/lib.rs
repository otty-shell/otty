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

mod answers;
mod database;
mod glob;
mod location;
mod logins;
mod message;
mod opencode;
mod plan;
mod provider_ids;
mod sessions;
mod stats;
#[cfg(test)]
mod testing;

/// The plugin: one type serves the agent part its manifest lists.
pub struct OpenCode;

otty_plugin_sdk::export!(OpenCode with agent);
