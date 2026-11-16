//! Claude Code: how OTTY finds its login, lists its sessions and starts it.
//!
//! Claude Code keeps everything in its config folder, `CLAUDE_CONFIG_DIR`
//! or `~/.claude`: the login in `.credentials.json` (or only the account in
//! the global config file `.claude.json`), and one file for each session in
//! `projects/<folder name>/<session id>.jsonl`. Its limits come from a
//! request to Anthropic's server with the token of `.credentials.json`.

mod agent;
mod detect;
mod plan;
mod session_file;
mod sessions;
mod usage;

pub use self::agent::ClaudeCode;
