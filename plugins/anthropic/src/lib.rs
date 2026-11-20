//! The official Anthropic plugin of OTTY. Its agent part `claude-code`
//! reads Claude Code: its config folders (profiles), its sessions, the
//! tokens of its answers, its statistics, its logins and the limits its
//! `/usage` panel shows, read in a hidden terminal. Claude Code
//! keeps everything in its config folder, `CLAUDE_CONFIG_DIR` or
//! `~/.claude`: the login in `.credentials.json` (the account in the
//! global config file `.claude.json`), keys in `settings.json`, and one
//! file for each session in `projects/<folder name>/<session id>.jsonl`.
//!
//! Its provider part `anthropic` reads the numbers of Anthropic accounts
//! with the handles OTTY gives it: the limits of a Claude login, whichever
//! agent keeps the login, and the bill of an organization with an admin
//! key, both from `api.anthropic.com`.

mod anthropic;
mod bill_days;
mod claude_code;
mod config_files;
mod cost_report;
mod detect;
mod file_ends;
mod glob;
mod logins;
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
mod stats;
#[cfg(test)]
mod testing;
mod timestamp;
mod trust_question;
mod usage_answer;
mod usage_probe;
mod usage_screen;
mod version;

/// The plugin: one type serves every part its manifest lists.
pub struct Anthropic;

otty_plugin_sdk::export!(Anthropic with agent, provider);
