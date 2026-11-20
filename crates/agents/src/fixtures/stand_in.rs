//! A stand-in agent for the tests of the app: Claude Code as the app
//! knows it, now that a plugin reads it (spec 2026-11-19-agent-plugins,
//! issue 13). It reads nothing: the tests tell the app what was read.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use crate::account::Account;
use crate::account_key::AccountKey;
use crate::admin_key::AdminKey;
use crate::agent::{Agent, AgentEntry, AgentId, ConfigFolder};
use crate::bill::{Bill, BillSource};
use crate::built_in::CLAUDE_ICON;
use crate::command::{AgentCommand, profile_command};
use crate::config_dir::default_config_dir;
use crate::env::AgentEnv;
use crate::error::{SourceError, Unavailable};
use crate::hidden_terminal::StartTerminal;
use crate::limits::LimitSource;
use crate::local_time::LocalNow;
use crate::profile::{Profile, ProfileStatus};
use crate::program::StartProgram;
use crate::registry::Reader;
use crate::session::{SessionId, SessionSource};
use crate::source_id::SourceId;
use crate::spend::SpendCounter;
use crate::stats::{ProjectStats, SessionStats, StatsSource};

/// `~/.claude`, which `CLAUDE_CONFIG_DIR` moves.
const CONFIG_FOLDER: ConfigFolder =
    ConfigFolder::new(".claude", "CLAUDE_CONFIG_DIR");
/// The ids of its session sources.
const SESSION_SOURCES: [SourceId; 1] = [SourceId::SessionFiles];
/// The ids of its limit sources, as the reader built into OTTY had them.
const LIMIT_SOURCES: [SourceId; 2] =
    [SourceId::AnthropicUsageApi, SourceId::ClaudeUsageCommand];

/// The stand-in: the entry, the config folder and the commands of Claude
/// Code, a bill and statistics that are never there, and no source.
struct ClaudeCodeStandIn {
    entry: AgentEntry,
}

impl Agent for ClaudeCodeStandIn {
    fn entry(&self) -> &AgentEntry {
        &self.entry
    }

    fn config_folder(&self) -> ConfigFolder {
        CONFIG_FOLDER
    }

    fn writes_cost(&self) -> bool {
        false
    }

    fn default_config_dir(&self, env: &AgentEnv) -> PathBuf {
        default_config_dir(&CONFIG_FOLDER, env)
    }

    fn detect(&self, _profile: &Profile) -> ProfileStatus {
        ProfileStatus::NotLoggedIn
    }

    fn accounts(&self, _profile: &Profile) -> Vec<Account> {
        Vec::new()
    }

    fn account_key(
        &self,
        _profile: &Profile,
        _account: &Account,
    ) -> Option<AccountKey> {
        None
    }

    fn has_sessions(&self, _profile: &Profile) -> bool {
        false
    }

    fn version(&self, _profile: &Profile) -> Option<String> {
        None
    }

    /// `claude`, `claude --resume <id>` to continue a session.
    fn launch(
        &self,
        profile: &Profile,
        session: Option<&SessionId>,
    ) -> AgentCommand {
        let command = self.login(profile);

        match session {
            Some(id) => command.with_arg("--resume").with_arg(id.as_str()),
            None => command,
        }
    }

    fn login(&self, profile: &Profile) -> AgentCommand {
        profile_command(profile, self.entry.program(), CONFIG_FOLDER.variable())
    }

    fn session_sources(&self) -> Vec<Box<dyn SessionSource>> {
        Vec::new()
    }

    fn session_source_ids(&self) -> &'static [SourceId] {
        &SESSION_SOURCES
    }

    fn limit_sources(
        &self,
        _programs: Arc<dyn StartProgram>,
        _terminals: Arc<dyn StartTerminal>,
    ) -> Vec<Box<dyn LimitSource>> {
        Vec::new()
    }

    fn limit_source_ids(&self) -> &'static [SourceId] {
        &LIMIT_SOURCES
    }

    fn spend_counter(&self) -> Option<Box<dyn SpendCounter>> {
        None
    }

    fn bill_source(&self) -> Option<Box<dyn BillSource>> {
        Some(Box::new(NotThere))
    }

    fn stats_source(&self) -> Option<Box<dyn StatsSource>> {
        Some(Box::new(NotThere))
    }
}

/// A bill and statistics that are never there.
struct NotThere;

impl BillSource for NotThere {
    fn bill(
        &self,
        _key: &AdminKey,
        _now: SystemTime,
    ) -> Result<Bill, SourceError> {
        Err(SourceError::NotAvailable(Unavailable::NoBill))
    }
}

impl StatsSource for NotThere {
    fn session_stats(
        &self,
        _profile: &Profile,
        _session: &SessionId,
        _now: &LocalNow,
    ) -> Result<SessionStats, SourceError> {
        Err(SourceError::NotAvailable(Unavailable::NoSessionFiles))
    }

    fn project_stats(
        &self,
        _profile: &Profile,
        _folder: &Path,
    ) -> Result<ProjectStats, SourceError> {
        Err(SourceError::NotAvailable(Unavailable::NoSessionFiles))
    }
}

/// Claude Code as the tests of the app know it: id `claude-code`, name
/// "Claude Code", program `claude`, config folder `~/.claude` moved by
/// `CLAUDE_CONFIG_DIR`, a built-in entry with the Claude icon, a bill and
/// statistics, and the limit sources the reader built into OTTY had. It
/// reads nothing; its bill and statistics are never there.
///
/// ```
/// use otty_agents::fixtures::claude_code_stand_in;
///
/// let agent = claude_code_stand_in();
///
/// assert_eq!(agent.entry().name(), "Claude Code");
/// assert!(agent.has_bill());
/// ```
pub fn claude_code_stand_in() -> Box<dyn Agent> {
    let entry = AgentEntry::new(
        AgentId::new("claude-code"),
        "Claude Code",
        "claude",
        "Anthropic",
        Reader::BuiltIn,
    )
    .with_icon(CLAUDE_ICON);

    Box::new(ClaudeCodeStandIn { entry })
}
