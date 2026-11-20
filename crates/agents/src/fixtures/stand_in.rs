//! Stand-in agents for the tests of the app: Claude Code and Codex as the
//! app knows them, now that plugins read them (spec
//! 2026-11-19-agent-plugins, issues 11 and 13). They read nothing: the
//! tests tell the app what was read.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::account::Account;
use crate::account_key::AccountKey;
use crate::agent::{Agent, AgentEntry, AgentId, ConfigFolder};
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

/// The Claude logo from Simple Icons, in one color that the app tints.
const CLAUDE_ICON: &[u8] =
    include_bytes!("../../../../assets/svg/agent-claude-code.svg");
/// The OpenAI logo from Simple Icons, in one color that the app tints.
const OPENAI_ICON: &[u8] =
    include_bytes!("../../../../assets/svg/agent-codex.svg");

/// The ids of the session sources of both, as the readers built into OTTY
/// had them.
const SESSION_SOURCES: [SourceId; 1] = [SourceId::SessionFiles];
/// The ids of the limit sources of Claude Code, as the reader built into
/// OTTY had them.
const CLAUDE_LIMIT_SOURCES: [SourceId; 2] =
    [SourceId::AnthropicUsageApi, SourceId::ClaudeUsageCommand];
/// The ids of the limit sources of Codex, as the reader built into OTTY
/// had them.
const CODEX_LIMIT_SOURCES: [SourceId; 3] = [
    SourceId::ChatGptUsageApi,
    SourceId::CodexAppServer,
    SourceId::LatestCodexSession,
];

/// A stand-in: the entry, the config folder and the commands of an agent,
/// statistics that are never there, and no source.
struct StandIn {
    entry: AgentEntry,
    folder: ConfigFolder,
    /// The argument before a session id that continues the session.
    resume: &'static str,
    /// The argument that logs in, when the agent has one.
    login: Option<&'static str>,
    limit_sources: &'static [SourceId],
}

impl Agent for StandIn {
    fn entry(&self) -> &AgentEntry {
        &self.entry
    }

    fn config_folder(&self) -> ConfigFolder {
        self.folder.clone()
    }

    fn writes_cost(&self) -> bool {
        false
    }

    fn default_config_dir(&self, env: &AgentEnv) -> PathBuf {
        default_config_dir(&self.folder, env)
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

    /// The program, with the resume argument and the id to continue a
    /// session.
    fn launch(
        &self,
        profile: &Profile,
        session: Option<&SessionId>,
    ) -> AgentCommand {
        let command = profile_command(
            profile,
            self.entry.program(),
            self.folder.variable(),
        );

        match session {
            Some(id) => command.with_arg(self.resume).with_arg(id.as_str()),
            None => command,
        }
    }

    /// The program, with its login argument when it has one.
    fn login(&self, profile: &Profile) -> AgentCommand {
        let command = profile_command(
            profile,
            self.entry.program(),
            self.folder.variable(),
        );

        match self.login {
            Some(login) => command.with_arg(login),
            None => command,
        }
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

    fn limit_source_ids(&self) -> Vec<SourceId> {
        self.limit_sources.to_vec()
    }

    fn spend_counter(&self) -> Option<Box<dyn SpendCounter>> {
        None
    }

    fn stats_source(&self) -> Option<Box<dyn StatsSource>> {
        Some(Box::new(NotThere))
    }
}

/// Statistics that are never there.
struct NotThere;

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
/// `CLAUDE_CONFIG_DIR`, a built-in entry with the Claude icon, statistics,
/// and the limit sources the reader built into OTTY had. It reads nothing;
/// its statistics are never there.
///
/// ```
/// use otty_agents::fixtures::claude_code_stand_in;
///
/// let agent = claude_code_stand_in();
///
/// assert_eq!(agent.entry().name(), "Claude Code");
/// assert!(agent.has_stats());
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

    Box::new(StandIn {
        entry,
        folder: ConfigFolder::new(".claude", "CLAUDE_CONFIG_DIR"),
        resume: "--resume",
        login: None,
        limit_sources: &CLAUDE_LIMIT_SOURCES,
    })
}

/// Codex as the tests of the app know it: id `codex`, name "Codex",
/// program `codex` (`codex resume <id>`, `codex login`), config folder
/// `~/.codex` moved by `CODEX_HOME`, a built-in entry with the OpenAI
/// icon, statistics, and the limit sources the reader built into OTTY had.
/// It reads nothing; its statistics are never there.
///
/// ```
/// use otty_agents::fixtures::codex_stand_in;
///
/// let agent = codex_stand_in();
///
/// assert_eq!(agent.entry().name(), "Codex");
/// assert!(agent.has_stats());
/// ```
pub fn codex_stand_in() -> Box<dyn Agent> {
    let entry = AgentEntry::new(
        AgentId::new("codex"),
        "Codex",
        "codex",
        "OpenAI",
        Reader::BuiltIn,
    )
    .with_icon(OPENAI_ICON);

    Box::new(StandIn {
        entry,
        folder: ConfigFolder::new(".codex", "CODEX_HOME"),
        resume: "resume",
        login: Some("login"),
        limit_sources: &CODEX_LIMIT_SOURCES,
    })
}
