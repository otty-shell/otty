//! The official OpenCode plugin of OTTY. Its one agent part, `opencode`,
//! reads OpenCode: its data folder (the profile), its sessions, and the
//! tokens and the cost of its answers. It is an agent only, with no
//! provider part and no login code (spec
//! 2026-11-23-accounts-apart-from-profiles, Decision 11): OpenCode works
//! with no login, with its free models, so the part never says a login is
//! missing.
//!
//! OpenCode keeps everything in one data folder, `~/.local/share/opencode`
//! (`XDG_DATA_HOME` moves it), and its SQLite database, `opencode.db`
//! (`OPENCODE_DB` moves it). The database holds the sessions and every
//! answer of a model with its provider, model, tokens and the cost
//! OpenCode worked out, which the part gives as it is, so the money is
//! exact. An answer with no cost written is priced at the API price OTTY
//! gives for its model.
//!
//! OTTY opens the database read-only and never migrates it, while
//! OpenCode may be writing to it at the same time. The logins OpenCode
//! keeps, in `auth.json` and in the `credential` table, are never read;
//! the manifest still declares them as keys, so the host never gives
//! them as text.

// A `ReadError` may hold a need, a record the contract can not box.
#![allow(clippy::result_large_err)]

mod answers;
mod database;
mod glob;
mod location;
mod message;
mod opencode;
mod sessions;
#[cfg(test)]
mod testing;

/// The plugin: one type serves the agent part its manifest lists.
pub struct OpenCode;

otty_plugin_sdk::export!(OpenCode with agents::agent);
