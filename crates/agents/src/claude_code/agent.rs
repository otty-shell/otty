//! Claude Code as an [`Agent`]: its name, its default config folder and the
//! commands that start it.

use std::path::PathBuf;
use std::sync::Arc;

use super::cost_report::CostReport;
use super::probe_memory::ProbeMemory;
use super::sessions::SessionFiles;
use super::spend::SessionSpend;
use super::stats::SessionFilesStats;
use super::usage::UsageRequest;
use super::usage_probe::UsageProbe;
use super::{accounts, detect, version};
use crate::account::Account;
use crate::account_key::AccountKey;
use crate::agent::{Agent, AgentEntry, AgentId, ConfigFolder};
use crate::bill::BillSource;
use crate::built_in;
use crate::command::{AgentCommand, profile_command};
use crate::config_dir::default_config_dir;
use crate::env::AgentEnv;
use crate::file_memory::FileMemory;
use crate::hidden_terminal::StartTerminal;
use crate::https::UreqClient;
use crate::limits::LimitSource;
use crate::profile::{Profile, ProfileStatus};
use crate::program::StartProgram;
use crate::registry::Reader;
use crate::session::{SessionId, SessionSource};
use crate::source_id::SourceId;
use crate::spend::SpendCounter;
use crate::spend_files::FileSpend;
use crate::stats::StatsSource;

/// The id Claude Code is saved under.
pub(super) const ID: &str = "claude-code";
/// The name shown to the user.
pub(super) const NAME: &str = "Claude Code";
/// The program, looked up on `PATH`.
pub(super) const PROGRAM: &str = "claude";
/// `~/.claude`, which `CLAUDE_CONFIG_DIR` moves.
const CONFIG_FOLDER: ConfigFolder =
    ConfigFolder::new(".claude", "CLAUDE_CONFIG_DIR");

/// The ids of the session sources, in the order they are given.
const SESSION_SOURCES: [SourceId; 1] = [SourceId::SessionFiles];
/// The ids of the limit sources, in the order they are given.
const LIMIT_SOURCES: [SourceId; 2] =
    [SourceId::AnthropicUsageApi, SourceId::ClaudeUsageCommand];

/// Claude Code, Anthropic's coding agent.
///
/// It remembers what it counted in the session files, so that the next
/// count reads only the files that changed, and the last answer of its
/// `/usage` panel, so that slow reading runs at most once in five minutes.
/// The counter and the sources it gives share that memory with it, so the
/// app keeps one `ClaudeCode` for the whole run.
///
/// ```
/// use otty_agents::{Agent, ClaudeCode};
///
/// assert_eq!(ClaudeCode::default().entry().name(), "Claude Code");
/// ```
#[derive(Debug)]
pub struct ClaudeCode {
    entry: AgentEntry,
    spend_memory: Arc<FileMemory<FileSpend>>,
    usage_probes: Arc<ProbeMemory>,
}

impl Default for ClaudeCode {
    fn default() -> Self {
        let entry = AgentEntry::new(
            AgentId::new(ID),
            NAME,
            PROGRAM,
            "Anthropic",
            Reader::BuiltIn,
        )
        .with_icon(built_in::CLAUDE_ICON);

        Self {
            entry,
            spend_memory: Arc::default(),
            usage_probes: Arc::default(),
        }
    }
}

impl Agent for ClaudeCode {
    fn entry(&self) -> &AgentEntry {
        &self.entry
    }

    fn config_folder(&self) -> ConfigFolder {
        CONFIG_FOLDER
    }

    /// No: the session files hold the tokens, not what they cost.
    fn writes_cost(&self) -> bool {
        false
    }

    fn default_config_dir(&self, env: &AgentEnv) -> PathBuf {
        default_config_dir(CONFIG_FOLDER, env)
    }

    fn detect(&self, profile: &Profile) -> ProfileStatus {
        detect::detect(profile)
    }

    fn accounts(&self, profile: &Profile) -> Vec<Account> {
        accounts::accounts(profile)
    }

    /// The key of the settings, for a key at a provider that tells its
    /// money, such as DeepSeek.
    fn account_key(
        &self,
        profile: &Profile,
        account: &Account,
    ) -> Option<AccountKey> {
        accounts::settings_key_of(profile, account)
    }

    /// Whether a project folder in `projects/` holds a session file.
    fn has_sessions(&self, profile: &Profile) -> bool {
        version::has_sessions(profile.config_dir())
    }

    /// The version the records of the newest session file name.
    fn version(&self, profile: &Profile) -> Option<String> {
        version::version(profile.config_dir())
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

    fn session_source_ids(&self) -> &'static [SourceId] {
        &SESSION_SOURCES
    }

    /// The usage request with the profile's own login token, then the
    /// `/usage` panel of `claude` started in a hidden terminal with
    /// `terminals`. No source talks to `claude` line by line, so
    /// `programs` is not used.
    fn limit_sources(
        &self,
        _programs: Arc<dyn StartProgram>,
        terminals: Arc<dyn StartTerminal>,
    ) -> Vec<Box<dyn LimitSource>> {
        let request = UsageRequest::new(Box::new(UreqClient::new()));
        let probe = UsageProbe::new(terminals, Arc::clone(&self.usage_probes));

        vec![Box::new(request), Box::new(probe)]
    }

    fn limit_source_ids(&self) -> &'static [SourceId] {
        &LIMIT_SOURCES
    }

    /// The tokens counted from the session files.
    fn spend_counter(&self) -> Option<Box<dyn SpendCounter>> {
        let counter = SessionSpend::new(Arc::clone(&self.spend_memory));

        Some(Box::new(counter))
    }

    /// The cost report of the Anthropic Admin API.
    fn bill_source(&self) -> Option<Box<dyn BillSource>> {
        let report = CostReport::new(Box::new(UreqClient::new()));

        Some(Box::new(report))
    }

    /// The statistics read from the session files.
    fn stats_source(&self) -> Option<Box<dyn StatsSource>> {
        Some(Box::new(SessionFilesStats))
    }
}

/// `claude` of `profile`, with `CLAUDE_CONFIG_DIR` when its folder is not
/// the default one.
pub(super) fn start_command(profile: &Profile) -> AgentCommand {
    profile_command(profile, PROGRAM, CONFIG_FOLDER.variable())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::testing::{claude_profile, no_programs, no_terminals};

    fn default_profile() -> Profile {
        Profile::default_for(
            AgentId::new(ID),
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
        let env = AgentEnv::new(PathBuf::from("/home/user"))
            .with_config_dir("CLAUDE_CONFIG_DIR", PathBuf::from("/data/claude"))
            .with_config_dir("CODEX_HOME", PathBuf::from("/data/codex"));

        let folder = ClaudeCode::default().default_config_dir(&env);

        assert_eq!(folder, PathBuf::from("/data/claude"));
    }

    #[test]
    fn the_default_config_folder_is_dot_claude_in_the_home_folder() {
        let env = AgentEnv::new(PathBuf::from("/home/user"))
            .with_config_dir("CODEX_HOME", PathBuf::from("/data/codex"));

        let folder = ClaudeCode::default().default_config_dir(&env);

        assert_eq!(folder, PathBuf::from("/home/user/.claude"));
    }

    #[test]
    fn the_default_profile_starts_claude_with_no_variables() {
        let profile = default_profile();

        let new_session = ClaudeCode::default().launch(&profile, None);
        let resumed =
            ClaudeCode::default().launch(&profile, Some(&session_id()));

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

        let new_session = ClaudeCode::default().launch(&profile, None);
        let resumed =
            ClaudeCode::default().launch(&profile, Some(&session_id()));

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
            AgentId::new(ID),
            "Claude Code",
            PathBuf::from("/data/claude"),
        )
        .with_agent_default_dir(Path::new("/home/user/.claude"));

        let command = ClaudeCode::default().launch(&profile, None);

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

        let command = ClaudeCode::default().launch(&profile, None);

        assert_eq!(command, AgentCommand::new("claude"));
    }

    #[test]
    fn login_starts_claude_in_the_config_folder_of_the_profile() {
        assert_eq!(
            ClaudeCode::default().login(&default_profile()),
            AgentCommand::new("claude")
        );
        assert_eq!(
            ClaudeCode::default().login(&added_profile()),
            AgentCommand::new("claude")
                .with_env("CLAUDE_CONFIG_DIR", "/home/user/.claude-work")
        );
    }

    #[test]
    fn sessions_limits_and_spend_come_from_their_sources() {
        let claude = ClaudeCode::default();

        let session_sources: Vec<_> = claude
            .session_sources()
            .iter()
            .map(|source| source.id())
            .collect();
        let limit_sources: Vec<_> = claude
            .limit_sources(no_programs(), no_terminals())
            .iter()
            .map(|source| source.id())
            .collect();

        assert_eq!(session_sources, [SourceId::SessionFiles]);
        assert_eq!(session_sources, claude.session_source_ids());
        assert_eq!(
            limit_sources,
            [SourceId::AnthropicUsageApi, SourceId::ClaudeUsageCommand]
        );
        assert_eq!(limit_sources, claude.limit_source_ids());
        assert!(claude.spend_counter().is_some());
    }
}
