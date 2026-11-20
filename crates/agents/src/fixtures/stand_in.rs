//! Stand-in agents for the tests of the app: made-up agent parts of the
//! official plugins, with the ids, names, programs and folders of the
//! agents those plugins read (spec 2026-11-19-agent-plugins, Decision 10).
//! They read nothing: the tests tell the app what was read. No code of
//! OTTY knows these names; they are test data only.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::account::Account;
use crate::agent::{Agent, AgentEntry, AgentId, ConfigFolder};
use crate::command::AgentCommand;
use crate::env::AgentEnv;
use crate::error::{SourceError, Unavailable};
use crate::hidden_terminal::StartTerminal;
use crate::limits::LimitSource;
use crate::local_time::LocalNow;
use crate::profile::{Profile, ProfileStatus};
use crate::program::StartProgram;
use crate::registry::PluginPart;
use crate::session::{SessionId, SessionSource};
use crate::source_id::{SourceId, SourceName};
use crate::spend::SpendCounter;
use crate::stats::{ProjectStats, SessionStats, StatsSource};

/// A made-up icon, a circle, standing in for the icon of a plugin part.
const CIRCLE_ICON: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><circle cx="8" cy="8" r="6"/></svg>"#;
/// Another made-up icon, a square.
const SQUARE_ICON: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><rect x="2" y="2" width="12" height="12"/></svg>"#;

/// The ids of the session sources of both: the plugin part.
const SESSION_SOURCES: [SourceId; 1] = [SourceId::Plugin];
/// The names the Claude Code stand-in gives its sources of limits.
const CLAUDE_LIMIT_SOURCES: [&str; 2] =
    ["Anthropic usage API", "Claude Code /usage"];
/// The names the Codex stand-in gives its sources of limits.
const CODEX_LIMIT_SOURCES: [&str; 3] = [
    "ChatGPT usage API",
    "codex app-server",
    "Latest Codex session",
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
    limit_sources: &'static [&'static str],
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
        self.limit_sources
            .iter()
            .map(|name| SourceId::Named(SourceName::new(name)))
            .collect()
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
/// `CLAUDE_CONFIG_DIR`, read by the made-up part `claude-code` of the
/// plugin `anthropic`, with a made-up icon, statistics and two named
/// sources of limits. It reads nothing; its statistics are never there.
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
        official_part("anthropic", "Anthropic", "claude-code"),
    )
    .with_icon(CIRCLE_ICON);

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
/// `~/.codex` moved by `CODEX_HOME`, read by the made-up part `codex` of
/// the plugin `openai`, with a made-up icon, statistics and three named
/// sources of limits. It reads nothing; its statistics are never there.
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
        official_part("openai", "OpenAI", "codex"),
    )
    .with_icon(SQUARE_ICON);

    Box::new(StandIn {
        entry,
        folder: ConfigFolder::new(".codex", "CODEX_HOME"),
        resume: "resume",
        login: Some("login"),
        limit_sources: &CODEX_LIMIT_SOURCES,
    })
}

/// The program of `profile` with its config folder and no arguments.
///
/// The program is `program`, a name the user's login shell finds the way
/// it finds it on `PATH`. A profile whose folder is not the agent's
/// default folder sets `config_dir_variable` to its folder; a profile in
/// the default folder sets nothing.
pub(crate) fn profile_command(
    profile: &Profile,
    program: &str,
    config_dir_variable: &str,
) -> AgentCommand {
    let command = AgentCommand::new(program);

    if profile.is_in_default_config_dir() {
        return command;
    }

    command
        .with_env(config_dir_variable, profile.config_dir().to_string_lossy())
}

/// The default config folder of `agent`: the folder the agent's own
/// variable names in `env` when it is set, else the agent's config folder
/// name in the home folder.
pub(crate) fn default_config_dir(
    agent: &ConfigFolder,
    env: &AgentEnv,
) -> PathBuf {
    match env.config_dir_of(agent.variable()) {
        Some(folder) => folder.to_path_buf(),
        None => env.home().join(agent.name()),
    }
}

/// The made-up part `part` of the official plugin `plugin`, named `name`,
/// version 0.1.0.
fn official_part(plugin: &str, name: &str, part: &str) -> PluginPart {
    PluginPart::new(
        plugin,
        name,
        "0.1.0",
        format!("https://github.com/otty-shell/otty-plugin-{plugin}"),
        part,
    )
}
