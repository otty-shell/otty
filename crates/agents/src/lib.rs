//! What OTTY knows about coding agents such as Claude Code and Codex.
//!
//! Each agent implements [`Agent`]: it says how to find its default
//! [`Profile`], whether a profile is connected, and how to start or continue
//! a session. The data an agent can read comes from small sources:
//! [`SessionSource`], [`LimitSource`] and [`SpendCounter`]. An agent gives
//! its sources best first, and [`read_sessions`] and [`read_limits`] try them
//! in order until one answers. [`projects`] groups the sessions of all
//! profiles by their folder.
//!
//! [`profiles`] gives the list of profiles to show, from the default
//! profile of each agent and the [`SavedProfiles`] of `agents.json`. The
//! dialog that adds a profile gets its folder from
//! [`candidate_config_dirs`] or [`proposed_config_dir`] and
//! [`create_config_dir`], and checks what the user typed with
//! [`check_profile_name`] and [`check_profile_config_dir`].
//!
//! The crate knows nothing about the window and reads no environment
//! itself: the app passes in an [`AgentEnv`] and runs the blocking calls on
//! a background thread.

mod agent;
mod claude_code;
mod codex;
mod command;
mod config_dir;
mod env;
mod error;
mod json_file;
mod limits;
mod profile;
mod profile_check;
mod profile_list;
mod project;
mod saved_profiles;
mod session;
mod spend;
#[cfg(test)]
mod testing;
mod timestamp;
mod title;
mod walk;

pub use crate::agent::{Agent, AgentDescriptor, AgentId};
pub use crate::claude_code::ClaudeCode;
pub use crate::codex::Codex;
pub use crate::command::AgentCommand;
pub use crate::config_dir::{
    CreateConfigDirError, candidate_config_dirs, create_config_dir,
    proposed_config_dir,
};
pub use crate::env::AgentEnv;
pub use crate::error::SourceError;
pub use crate::limits::{Credits, LimitSource, LimitWindow, Limits};
pub use crate::profile::{Profile, ProfileStatus};
pub use crate::profile_check::{
    ProfileInputError, check_profile_config_dir, check_profile_name,
};
pub use crate::profile_list::profiles;
pub use crate::project::{Project, ProjectSession, projects};
pub use crate::saved_profiles::{
    DefaultProfileChanges, SavedProfiles, SavedProfilesError,
};
pub use crate::session::{AgentSession, SessionId, SessionSource};
pub use crate::spend::{Spend, SpendCounter, TokenCount};
pub use crate::walk::{Answer, SourceFailure, read_limits, read_sessions};

/// The agents OTTY knows, in the order they are shown.
///
/// Adding an agent means implementing [`Agent`] and adding it here.
///
/// An agent may remember what it read between two refreshes, so the app
/// calls this once and keeps the list for the whole run.
///
/// ```
/// for agent in otty_agents::known_agents() {
///     println!("{}", agent.descriptor().name());
/// }
/// ```
pub fn known_agents() -> Vec<Box<dyn Agent>> {
    vec![Box::new(ClaudeCode), Box::new(Codex::default())]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claude_code_and_codex_are_the_known_agents() {
        let ids: Vec<_> = known_agents()
            .iter()
            .map(|agent| agent.descriptor().id().as_str())
            .collect();

        assert_eq!(ids, ["claude-code", "codex"]);
    }
}
