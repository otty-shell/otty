//! The official OpenAI plugin of OTTY. Its agent part `codex` reads
//! Codex: its config folders (profiles), the login of each, its sessions,
//! the tokens of its answers and the limits Codex tells
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
//! an organization with an admin key from `api.openai.com`. A ChatGPT
//! login it sees refused or expired it only reports as refused: OTTY asks
//! the agent part that keeps it, Codex's or another plugin's, what the
//! user must do.
//!
//! What only the user can do, the parts give as needs (`needs.rs`): log
//! in to Codex, run Codex so it renews an expired login, and give or
//! replace the admin key of an organization's bill. The texts come in
//! every language of OTTY, so the plugin exports the language receiver.

// A `ReadError` may hold a need, a record the contract can not box.
#![allow(clippy::result_large_err)]

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
mod needs;
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
#[cfg(test)]
mod testing;
mod timestamp;
mod token_expiry;
mod version;
mod workspace_credits;

use otty_plugin_sdk::Language;

/// The plugin: one type serves every part its manifest lists.
pub struct OpenAi;

impl Language for OpenAi {
    /// The texts of the needs come in every language of OTTY.
    fn language_changed(_tag: &str) -> bool {
        true
    }
}

otty_plugin_sdk::export!(OpenAi with agents::agent, agents::provider, language);
