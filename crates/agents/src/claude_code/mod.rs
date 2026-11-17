//! Claude Code: how OTTY finds its login, lists its sessions and starts it.
//!
//! Claude Code keeps everything in its config folder, `CLAUDE_CONFIG_DIR`
//! or `~/.claude`: the login in `.credentials.json` (or only the account in
//! the global config file `.claude.json`), and one file for each session in
//! `projects/<folder name>/<session id>.jsonl`. Its limits come from a
//! request to Anthropic's server with the token of `.credentials.json`, or,
//! when that fails, from the `/usage` panel of `claude` in a hidden
//! terminal. The tokens it spent are counted from the session files. With an admin
//! key, the bill of the organization comes from the Admin API cost report.

mod agent;
mod cost_report;
mod detect;
mod plan;
mod probe_memory;
mod probe_sessions;
mod session_file;
mod sessions;
mod spend;
mod spend_file;
mod usage;
mod usage_answer;
mod usage_probe;
mod usage_screen;
mod window_name;

pub use self::agent::ClaudeCode;
