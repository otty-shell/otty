//! Codex: how OTTY finds its login, lists its sessions and starts it.
//!
//! Codex keeps everything in its config folder, `CODEX_HOME` or `~/.codex`:
//! the login in `auth.json`, one file for each session in
//! `sessions/YYYY/MM/DD/rollout-<time>-<id>.jsonl` (older ones compressed
//! to `.jsonl.zst`, archived ones in `archived_sessions/`), and the names
//! of the sessions in `session_index.jsonl`. Its limits come from a request
//! to the ChatGPT server with the token of `auth.json`.
//!
//! OTTY never asks `codex app-server` for the list of sessions: that
//! request repairs Codex's own index of sessions and can delete from it
//! (spec Decision 5).

mod agent;
mod base_url;
mod cache;
mod detect;
mod session_file;
mod session_index;
mod sessions;
mod token_expiry;
mod usage;

pub use self::agent::Codex;
