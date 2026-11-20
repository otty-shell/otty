//! The official OpenAI plugin of OTTY. Its agent part `codex` reads
//! Codex: its config folders (profiles), its sessions, the tokens of its
//! answers and its statistics. Codex keeps everything in its config
//! folder, `CODEX_HOME` or `~/.codex`: the login in `auth.json`, one file
//! for each session in `sessions/YYYY/MM/DD/rollout-<time>-<id>.jsonl`
//! (older ones compressed to `.jsonl.zst`, archived ones in
//! `archived_sessions/`), and the names of the sessions in
//! `session_index.jsonl`.
//!
//! The plugin never asks `codex app-server` for the list of sessions:
//! that request repairs Codex's own index of sessions and can delete from
//! it (spec Decision 13).

mod auth_file;
mod codex;
mod detect;
mod glob;
mod profiles;
mod session_file;
mod session_index;
mod sessions;
mod spend_file;
mod stats;
#[cfg(test)]
mod testing;
mod timestamp;
mod version;

/// The plugin: one type serves every part its manifest lists.
pub struct OpenAi;

otty_plugin_sdk::export!(OpenAi with agent);
