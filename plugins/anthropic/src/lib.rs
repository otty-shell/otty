//! The official Anthropic plugin of OTTY. Its agent part `claude-code`
//! reads Claude Code: its config folders (profiles), whether each is
//! logged in, its sessions and the tokens of its answers. Claude Code
//! keeps everything in its config folder, `CLAUDE_CONFIG_DIR` or
//! `~/.claude`: the login in `.credentials.json` (the account in the
//! global config file `.claude.json`), keys in `settings.json`, and one
//! file for each session in `projects/<folder name>/<session id>.jsonl`.
//!
//! Its provider part `anthropic` owns the accounts (spec
//! 2026-11-23-accounts-apart-from-profiles, Decisions 5 and 8). OTTY
//! gives it the folders of the Claude Code profiles, and it reads the
//! logins there itself (`logins.rs`): a Claude login is a subscription
//! known by its account id (`claude:<uuid>`), a Console organization
//! paid per token is that organization (`console:<uuid>`), and an
//! Anthropic key is known by its fingerprint (`key:<fingerprint>`). A
//! key the settings send to another server, such as DeepSeek's, is no
//! account. It reads the numbers of an account any way it can (issue
//! 29), only in the account's folders: the limits of a Claude login from
//! `api.anthropic.com` with its token, else from the `/usage` panel of
//! `claude` pointed at the folder in a hidden terminal, such as a profile
//! whose `.credentials.json` holds no token while Claude Code still
//! works; and the bill of an organization with an admin key. With no way
//! that works it reads nothing. Add account → "Log in with Claude Code"
//! runs `claude auth login --claudeai` with `CLAUDE_CONFIG_DIR` set to
//! OTTY's own folder, and the part reads that folder as a profile folder.
//!
//! What only the user can do, the parts give as needs (`needs.rs`): log
//! in to Claude Code, run Claude Code in a folder so it renews a login
//! that expired or Anthropic refused, and give or replace the admin key
//! of an organization's bill. The texts come in every language of OTTY,
//! so the plugin exports the language receiver.

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
mod session_spend;
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
