//! Codex: how OTTY finds its login, lists its sessions and starts it.
//!
//! Codex keeps everything in its config folder, `CODEX_HOME` or `~/.codex`:
//! the login in `auth.json`, one file for each session in
//! `sessions/YYYY/MM/DD/rollout-<time>-<id>.jsonl` (older ones compressed
//! to `.jsonl.zst`, archived ones in `archived_sessions/`), and the names
//! of the sessions in `session_index.jsonl`. Its limits come from a request
//! to the ChatGPT server with the token of `auth.json`, else from
//! `codex app-server`, else from the last record of the limits in the
//! newest session file. The tokens it spent and the statistics of a session
//! or a project are read from the session files.
//!
//! OTTY asks `codex app-server` only for the limits, never for the list of
//! sessions: that request repairs Codex's own index of sessions and can
//! delete from it (spec Decision 5).

mod agent;
mod app_server;
mod auth_file;
mod base_url;
mod detect;
mod individual_limit;
mod rate_limits;
mod session_file;
mod session_index;
mod session_limits;
mod sessions;
mod spend;
mod spend_file;
mod stats;
mod token_expiry;
mod usage;
mod workspace_credits;

pub use self::agent::Codex;
