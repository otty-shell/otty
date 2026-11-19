//! OpenCode and Kilo, a fork of OpenCode with the same storage: how OTTY
//! finds their logins, lists their sessions and counts their tokens.
//!
//! Both keep their data in one folder of the XDG data folder,
//! `~/.local/share/opencode` or `~/.local/share/kilo` (`XDG_DATA_HOME`
//! moves it): the logins in `auth.json`, Kilo's also in `account.json`,
//! and in the `credential` table of their SQLite database, `opencode.db`
//! or `kilo.db` (`OPENCODE_DB` and `KILO_DB` move it). The database holds
//! the sessions and every answer of a model with its provider, model,
//! tokens and the cost the agent worked out, so their money is exact.
//!
//! OTTY opens the database read-only, never migrates it and never writes
//! to it, while the agent may be writing to it at the same time.

mod accounts;
mod agent;
mod database;
mod fork;
mod location;
mod message;
mod provider_ids;
mod sessions;
mod spend;
mod stats;
#[cfg(test)]
mod testing;

pub use self::agent::OpenCodeAgent;
