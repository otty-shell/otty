//! Claude Code as an [`Agent`]: its name, its default config folder and the
//! commands that start it.

use std::path::{Path, PathBuf};

use super::detect;
use super::sessions::SessionFiles;
use super::usage::UsageRequest;
use crate::agent::{Agent, AgentDescriptor, AgentId};
use crate::command::{AgentCommand, profile_command};
use crate::env::AgentEnv;
use crate::https::UreqClient;
use crate::limits::LimitSource;
use crate::profile::{Profile, ProfileStatus};
use crate::session::{SessionId, SessionSource};
use crate::spend::SpendCounter;

const DESCRIPTOR: AgentDescriptor = AgentDescriptor::new(
    AgentId::new("claude-code"),
    "Claude Code",
    "claude-code",
    ".claude",
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
            None => env.home().join(DESCRIPTOR.config_dir_name()),
        }
    }

    /// A folder with a `projects/` folder and a `.claude.json` file inside.
    fn looks_like_config_dir(&self, dir: &Path) -> bool {
        dir.join("projects").is_dir() && dir.join(".claude.json").is_file()
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

    /// The usage request with the profile's own login token.
    fn limit_sources(&self) -> Vec<Box<dyn LimitSource>> {
        let request = UsageRequest::new(Box::new(UreqClient::new()));

        vec![Box::new(request)]
    }

    fn spend_counter(&self) -> Option<Box<dyn SpendCounter>> {
        None
    }
}

/// `claude` of `profile`, with `CLAUDE_CONFIG_DIR` when its folder is not
/// the default one.
fn start_command(profile: &Profile) -> AgentCommand {
    profile_command(profile, PROGRAM, CONFIG_DIR_VARIABLE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{TempDir, claude_profile, write_file};

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
    fn a_default_profile_moved_to_another_folder_starts_claude_with_it() {
        let profile = Profile::default_for(
            DESCRIPTOR.id(),
            "Claude Code",
            PathBuf::from("/data/claude"),
        )
        .with_agent_default_dir(Path::new("/home/user/.claude"));

        let command = ClaudeCode.launch(&profile, None);

        assert_eq!(
            command,
            AgentCommand::new("claude")
                .with_env("CLAUDE_CONFIG_DIR", "/data/claude")
        );
    }

    #[test]
    fn a_profile_in_the_default_folder_starts_claude_with_no_variables() {
        let profile = claude_profile(Path::new("/home/user/.claude"))
            .with_agent_default_dir(Path::new("/home/user/.claude"));

        let command = ClaudeCode.launch(&profile, None);

        assert_eq!(command, AgentCommand::new("claude"));
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
    fn a_folder_with_projects_and_a_global_config_file_looks_like_claude() {
        let temp = TempDir::new();
        let dir = temp.path().join(".claude-work");
        std::fs::create_dir_all(dir.join("projects")).expect("create folder");
        write_file(&dir.join(".claude.json"), "{}");

        assert!(ClaudeCode.looks_like_config_dir(&dir));
    }

    #[test]
    fn a_folder_with_only_one_of_the_two_does_not_look_like_claude() {
        let temp = TempDir::new();
        let only_projects = temp.path().join(".claude-a");
        let only_config = temp.path().join(".claude-b");
        let config_is_a_folder = temp.path().join(".claude-c");
        std::fs::create_dir_all(only_projects.join("projects"))
            .expect("create folder");
        write_file(&only_config.join(".claude.json"), "{}");
        std::fs::create_dir_all(config_is_a_folder.join("projects"))
            .expect("create folder");
        std::fs::create_dir_all(config_is_a_folder.join(".claude.json"))
            .expect("create folder");

        assert!(!ClaudeCode.looks_like_config_dir(&only_projects));
        assert!(!ClaudeCode.looks_like_config_dir(&only_config));
        assert!(!ClaudeCode.looks_like_config_dir(&config_is_a_folder));
        assert!(!ClaudeCode.looks_like_config_dir(&temp.path().join("none")));
    }

    #[test]
    fn sessions_come_from_the_files_limits_from_the_usage_request_and_no_spend_yet()
     {
        let session_sources: Vec<_> = ClaudeCode
            .session_sources()
            .iter()
            .map(|source| source.name())
            .collect();

        assert_eq!(session_sources, ["session files"]);
        let limit_sources: Vec<_> = ClaudeCode
            .limit_sources()
            .iter()
            .map(|source| source.name())
            .collect();
        assert_eq!(limit_sources, ["usage request"]);
        assert!(ClaudeCode.spend_counter().is_none());
    }
}
