//! Claude Code as an [`Agent`]: its name, its default config folder and the
//! commands that start it.

use std::path::PathBuf;

use super::detect;
use super::sessions::SessionFiles;
use crate::agent::{Agent, AgentDescriptor, AgentId};
use crate::command::AgentCommand;
use crate::env::AgentEnv;
use crate::limits::LimitSource;
use crate::profile::{Profile, ProfileStatus};
use crate::session::{SessionId, SessionSource};
use crate::spend::SpendCounter;

const DESCRIPTOR: AgentDescriptor = AgentDescriptor::new(
    AgentId::new("claude-code"),
    "Claude Code",
    "claude-code",
);
/// The name of the program, looked up on `PATH`.
const PROGRAM: &str = "claude";
/// The variable that points Claude Code at a config folder.
const CONFIG_DIR_VARIABLE: &str = "CLAUDE_CONFIG_DIR";

/// Claude Code, Anthropic's coding agent.
///
/// ```
/// use otty_agents::{Agent, ClaudeCode};
///
/// assert_eq!(ClaudeCode.descriptor().name(), "Claude Code");
/// ```
#[derive(Debug, Default, Clone, Copy)]
pub struct ClaudeCode;

impl Agent for ClaudeCode {
    fn descriptor(&self) -> &AgentDescriptor {
        &DESCRIPTOR
    }

    fn default_config_dir(&self, env: &AgentEnv) -> PathBuf {
        match env.claude_config_dir() {
            Some(config_dir) => config_dir.to_path_buf(),
            None => env.home().join(".claude"),
        }
    }

    fn detect(&self, profile: &Profile) -> ProfileStatus {
        detect::detect(profile)
    }

    /// `claude` for a new session, `claude --resume <id>` to continue one.
    fn launch(
        &self,
        profile: &Profile,
        session: Option<&SessionId>,
    ) -> AgentCommand {
        let command = start_command(profile);

        match session {
            Some(id) => command.with_arg("--resume").with_arg(id.as_str()),
            None => command,
        }
    }

    /// `claude`, which asks to log in on its first start in a config folder.
    fn login(&self, profile: &Profile) -> AgentCommand {
        start_command(profile)
    }

    fn session_sources(&self) -> Vec<Box<dyn SessionSource>> {
        vec![Box::new(SessionFiles)]
    }

    fn limit_sources(&self) -> Vec<Box<dyn LimitSource>> {
        Vec::new()
    }

    fn spend_counter(&self) -> Option<Box<dyn SpendCounter>> {
        None
    }
}

/// The program of `profile` with its config folder and no arguments.
///
/// An added profile sets `CLAUDE_CONFIG_DIR`. The default profile sets
/// nothing, so what the user has in their own environment keeps working.
fn start_command(profile: &Profile) -> AgentCommand {
    let program = match profile.program() {
        Some(path) => path.to_string_lossy().into_owned(),
        None => String::from(PROGRAM),
    };
    let command = AgentCommand::new(program);

    if profile.is_default() {
        return command;
    }

    command
        .with_env(CONFIG_DIR_VARIABLE, profile.config_dir().to_string_lossy())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::testing::claude_profile;

    fn default_profile() -> Profile {
        Profile::default_for(
            DESCRIPTOR.id(),
            "Claude Code",
            PathBuf::from("/home/user/.claude"),
        )
    }

    fn added_profile() -> Profile {
        claude_profile(Path::new("/home/user/.claude-work"))
    }

    fn session_id() -> SessionId {
        SessionId::new("0b6c4e8a-1f2d-4c3b-9a7e-5d1f2e3c4b5a")
    }

    #[test]
    fn the_default_config_folder_is_claude_config_dir_when_it_is_set() {
        let env = AgentEnv::new(
            PathBuf::from("/home/user"),
            Some(PathBuf::from("/data/claude")),
            None,
        );

        let folder = ClaudeCode.default_config_dir(&env);

        assert_eq!(folder, PathBuf::from("/data/claude"));
    }

    #[test]
    fn the_default_config_folder_is_dot_claude_in_the_home_folder() {
        let env = AgentEnv::new(PathBuf::from("/home/user"), None, None);

        let folder = ClaudeCode.default_config_dir(&env);

        assert_eq!(folder, PathBuf::from("/home/user/.claude"));
    }

    #[test]
    fn the_default_profile_starts_claude_with_no_variables() {
        let profile = default_profile();

        let new_session = ClaudeCode.launch(&profile, None);
        let resumed = ClaudeCode.launch(&profile, Some(&session_id()));

        assert_eq!(new_session, AgentCommand::new("claude"));
        assert_eq!(
            resumed,
            AgentCommand::new("claude")
                .with_arg("--resume")
                .with_arg("0b6c4e8a-1f2d-4c3b-9a7e-5d1f2e3c4b5a")
        );
    }

    #[test]
    fn an_added_profile_starts_claude_with_its_config_folder() {
        let profile = added_profile();

        let new_session = ClaudeCode.launch(&profile, None);
        let resumed = ClaudeCode.launch(&profile, Some(&session_id()));

        assert_eq!(
            new_session,
            AgentCommand::new("claude")
                .with_env("CLAUDE_CONFIG_DIR", "/home/user/.claude-work")
        );
        assert_eq!(
            resumed,
            AgentCommand::new("claude")
                .with_arg("--resume")
                .with_arg("0b6c4e8a-1f2d-4c3b-9a7e-5d1f2e3c4b5a")
                .with_env("CLAUDE_CONFIG_DIR", "/home/user/.claude-work")
        );
    }

    #[test]
    fn login_starts_claude_in_the_config_folder_of_the_profile() {
        assert_eq!(
            ClaudeCode.login(&default_profile()),
            AgentCommand::new("claude")
        );
        assert_eq!(
            ClaudeCode.login(&added_profile()),
            AgentCommand::new("claude")
                .with_env("CLAUDE_CONFIG_DIR", "/home/user/.claude-work")
        );
    }

    #[test]
    fn the_program_path_of_the_profile_is_used_instead_of_claude() {
        let profile = default_profile()
            .with_program(PathBuf::from("/opt/claude/bin/claude"));

        let command = ClaudeCode.launch(&profile, None);

        assert_eq!(command, AgentCommand::new("/opt/claude/bin/claude"));
    }

    #[test]
    fn sessions_come_from_the_files_and_there_are_no_limits_or_spend_yet() {
        let session_sources: Vec<_> = ClaudeCode
            .session_sources()
            .iter()
            .map(|source| source.name())
            .collect();

        assert_eq!(session_sources, ["session files"]);
        assert!(ClaudeCode.limit_sources().is_empty());
        assert!(ClaudeCode.spend_counter().is_none());
    }
}
