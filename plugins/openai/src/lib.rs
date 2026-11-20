//! The official OpenAI plugin of OTTY. Its agent part `codex` reads
//! Codex: its config folders (profiles), the login of each, its sessions,
//! the tokens of its answers, its statistics and the limits Codex tells
//! (`codex app-server` and the records of its session files). Codex keeps
//! everything in its config folder, `CODEX_HOME` or `~/.codex`: the login
//! in `auth.json`, one file for each session in
//! `sessions/YYYY/MM/DD/rollout-<time>-<id>.jsonl` (older ones compressed
//! to `.jsonl.zst`, archived ones in `archived_sessions/`), and the names
//! of the sessions in `session_index.jsonl`.
//!
//! The plugin asks `codex app-server` only for the limits, never for the
//! list of sessions: that request repairs Codex's own index of sessions
//! and can delete from it (spec Decision 13).
//!
//! Its provider part `openai` reads the numbers of OpenAI accounts with
//! the handles OTTY gives it: the limits and credits of a ChatGPT login
//! from `chatgpt.com`, whichever agent keeps the login, and the costs of
//! an organization with an admin key from `api.openai.com`.

mod accounts;
mod app_server;
mod auth_file;
mod base_url;
mod bill_days;
mod chatgpt_usage;
mod codex;
mod detect;
mod glob;
mod individual_limit;
mod openai;
mod organization_costs;
mod profiles;
mod rate_limits;
mod requests;
mod session_file;
mod session_index;
mod session_limits;
mod sessions;
mod spend_file;
mod stats;
#[cfg(test)]
mod testing;
mod timestamp;
mod token_expiry;
mod version;
mod workspace_credits;

/// The plugin: one type serves every part its manifest lists.
pub struct OpenAi;

otty_plugin_sdk::export!(OpenAi with agent, provider);
