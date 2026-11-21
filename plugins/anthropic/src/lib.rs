//! The official Anthropic plugin of OTTY. Its agent part `claude-code`
//! reads Claude Code: its config folders (profiles), its sessions, the
//! tokens of its answers, its logins and the limits its
//! `/usage` panel shows, read in a hidden terminal. Claude Code
//! keeps everything in its config folder, `CLAUDE_CONFIG_DIR` or
//! `~/.claude`: the login in `.credentials.json` (the account in the
//! global config file `.claude.json`), keys in `settings.json`, and one
//! file for each session in `projects/<folder name>/<session id>.jsonl`.
//!
//! Its provider part `anthropic` reads the numbers of Anthropic accounts
//! with the handles OTTY gives it: the limits of a Claude login, whichever
//! agent keeps the login, and the bill of an organization with an admin
//! key, both from `api.anthropic.com`. A Claude login it sees refused it
//! only reports as refused: OTTY asks the agent part that keeps it what
//! the user must do.
//!
//! What only the user can do, the parts give as needs (`needs.rs`): log
//! in to Claude Code, run Claude Code so it renews an expired login, and
//! give or replace the admin key of an organization's bill. The texts
//! come in every language of OTTY, so the plugin exports the language
//! receiver.

// A `ReadError` may hold a need, a record the contract can not box.
#![allow(clippy::result_large_err)]

mod anthropic;
mod bill_days;
mod claude_code;
mod config_files;
mod cost_report;
mod detect;
mod file_ends;
mod glob;
mod logins;
mod needs;
mod oauth_usage;
mod plan;
mod probe_sessions;
mod profiles;
mod project_folder;
mod provider;
mod requests;
mod session_file;
mod sessions;
mod spend_file;
#[cfg(test)]
mod testing;
mod timestamp;
mod trust_question;
mod usage_answer;
mod usage_probe;
mod usage_screen;
mod version;

use otty_plugin_sdk::Language;

/// The plugin: one type serves every part its manifest lists.
pub struct Anthropic;

impl Language for Anthropic {
    /// The texts of the needs come in every language of OTTY.
    fn language_changed(_tag: &str) -> bool {
        true
    }
}

otty_plugin_sdk::export!(Anthropic with agents::agent, agents::provider, language);
