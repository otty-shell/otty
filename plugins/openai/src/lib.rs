//! The official OpenAI plugin of OTTY. Its agent part `codex` reads
//! Codex: its config folders (profiles), whether each is logged in, its
//! sessions and the tokens of its answers. Codex keeps
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
//! Its provider part `openai` finds its accounts itself (spec
//! 2026-11-23-accounts-apart-from-profiles, Decisions 5 and 8), in the
//! Codex profile folders OTTY gives it, and reads only there: the ChatGPT
//! login of `auth.json` is a subscription known by its account id
//! (`chatgpt:<account id>`), an API key is a key known by its fingerprint
//! (`key:<fingerprint>`), and one account in two folders is one account
//! with two places (`accounts.rs`). A login behind a gateway is no OpenAI
//! account. Add account → "Log in with Codex" runs `codex login` with
//! `CODEX_HOME` set to OTTY's own folder, and the part reads that folder
//! as a Codex profile folder. It reads the numbers any way it can (issue 29), only in the
//! account's folders: the limits and credits of a ChatGPT login from
//! `chatgpt.com` with its token, else the limits Codex tells there
//! (`codex app-server` and the records of its session files), and the
//! costs of an organization with an admin key from `api.openai.com`. With
//! no way that works it reads nothing. Whether a login expired is its
//! call: an expired or refused ChatGPT login needs `codex login` in its
//! folder.
//!
//! What only the user can do, the parts give as needs (`needs.rs`): log
//! in to Codex, log in again when a login expired, and give or replace
//! the admin key of an organization's bill. The texts come in every
//! language of OTTY, so the plugin exports the language receiver.

// A `ReadError` may hold a need, a record the contract can not box.
#![allow(clippy::result_large_err)]

mod accounts;
mod app_server;
mod auth_file;
mod base_url;
mod bill_days;
mod chatgpt_usage;
mod codex;
mod codex_limits;
mod detect;
mod glob;
mod individual_limit;
mod key_spend;
mod limits;
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
