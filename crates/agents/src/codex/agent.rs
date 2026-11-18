//! Codex as an [`Agent`]: its name, its default config folder and the
//! commands that start it.

use std::path::PathBuf;
use std::sync::Arc;

use super::app_server::AppServer;
use super::detect;
use super::session_file::SessionMeta;
use super::session_limits::SessionRecord;
use super::sessions::SessionFiles;
use super::spend::SessionSpend;
use super::stats::SessionFilesStats;
use super::usage::UsageRequest;
use crate::agent::{Agent, AgentDescriptor, AgentId};
use crate::bill::BillSource;
use crate::command::{AgentCommand, profile_command};
use crate::config_dir::default_config_dir;
use crate::env::AgentEnv;
use crate::file_memory::FileMemory;
use crate::hidden_terminal::StartTerminal;
use crate::https::UreqClient;
use crate::limits::LimitSource;
use crate::profile::{Profile, ProfileStatus};
use crate::program::StartProgram;
use crate::session::{SessionId, SessionSource};
use crate::spend::SpendCounter;
use crate::spend_files::FileSpend;
use crate::stats::StatsSource;

const DESCRIPTOR: AgentDescriptor = AgentDescriptor::new(
    AgentId::new("codex"),
    "Codex",
    "codex",
    ".codex",
    "OpenAI",
);
/// The name of the program, looked up on `PATH`.
const PROGRAM: &str = "codex";
/// The variable that points Codex at a config folder.
const CONFIG_DIR_VARIABLE: &str = "CODEX_HOME";

/// Codex, OpenAI's coding agent.
///
/// It remembers what it read and counted in the session files, so that
/// the next refresh reads only the files that changed. The sources and the
/// counter it gives share that memory with it, so the app keeps one
/// `Codex` for the whole run: a new one reads every file again.
///
/// ```
/// use otty_agents::{Agent, Codex};
///
/// assert_eq!(Codex::default().descriptor().name(), "Codex");
/// ```
#[derive(Debug, Default)]
pub struct Codex {
    read_files: Arc<FileMemory<Option<SessionMeta>>>,
    counted_files: Arc<FileMemory<FileSpend>>,
}

impl Agent for Codex {
    fn descriptor(&self) -> &AgentDescriptor {
        &DESCRIPTOR
    }

    fn default_config_dir(&self, env: &AgentEnv) -> PathBuf {
        default_config_dir(&DESCRIPTOR, env, env.codex_home())
    }

    fn detect(&self, profile: &Profile) -> ProfileStatus {
        detect::detect(profile)
    }

    /// `codex` for a new session, `codex resume <id>` to continue one.
    fn launch(
        &self,
        profile: &Profile,
        session: Option<&SessionId>,
    ) -> AgentCommand {
        let command = start_command(profile);

        match session {
            Some(id) => command.with_arg("resume").with_arg(id.as_str()),
            None => command,
        }
    }

    /// `codex login`.
    fn login(&self, profile: &Profile) -> AgentCommand {
        start_command(profile).with_arg("login")
    }

    fn session_sources(&self) -> Vec<Box<dyn SessionSource>> {
        let files = SessionFiles::new(Arc::clone(&self.read_files));

        vec![Box::new(files)]
    }

    /// The usage request with the profile's own login token, then
    /// `codex app-server` started with `programs`, then the last record of
    /// the limits in the newest session file.
    fn limit_sources(
        &self,
        programs: Arc<dyn StartProgram>,
        _terminals: Arc<dyn StartTerminal>,
    ) -> Vec<Box<dyn LimitSource>> {
        let request = UsageRequest::new(Box::new(UreqClient::new()));
        let app_server = AppServer::new(programs);

        vec![
            Box::new(request),
            Box::new(app_server),
            Box::new(SessionRecord),
        ]
    }

    /// The tokens counted from the session files.
    fn spend_counter(&self) -> Option<Box<dyn SpendCounter>> {
        let counter = SessionSpend::new(Arc::clone(&self.counted_files));

        Some(Box::new(counter))
    }

    /// None: the bill of an OpenAI organization is out of scope, so a
    /// Codex API key profile shows only the estimate.
    fn bill_source(&self) -> Option<Box<dyn BillSource>> {
        None
    }

    /// The statistics read from the session files, found by what the
    /// session list read last.
    fn stats_source(&self) -> Option<Box<dyn StatsSource>> {
        let stats = SessionFilesStats::new(Arc::clone(&self.read_files));

        Some(Box::new(stats))
    }
}

/// `codex` of `profile`, with `CODEX_HOME` when its folder is not the
/// default one.
pub(super) fn start_command(profile: &Profile) -> AgentCommand {
    profile_command(profile, PROGRAM, CONFIG_DIR_VARIABLE)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::source_id::SourceId;
    use crate::testing::{
        TempDir, at, codex_profile, no_programs, no_terminals, set_file_time,
        write_file,
    };

    const ID: &str = "019a8f3e-7c2d-7b41-9e5a-3c4d5e6f7a8b";

    fn default_profile() -> Profile {
        Profile::default_for(
            DESCRIPTOR.id(),
            "Codex",
            PathBuf::from("/home/user/.codex"),
        )
    }

    fn added_profile() -> Profile {
        codex_profile(Path::new("/home/user/.codex-work"))
    }

    fn session_id() -> SessionId {
        SessionId::new(ID)
    }

    fn first_line(folder: &str) -> String {
        format!(
            r#"{{"timestamp":"2026-11-16T10:00:00.000Z","type":"session_meta","payload":{{"id":"{ID}","cwd":"{folder}","source":"cli"}}}}"#
        )
    }

    #[test]
    fn the_default_config_folder_is_codex_home_when_it_is_set() {
        let env = AgentEnv::new(
            PathBuf::from("/home/user"),
            Some(PathBuf::from("/data/claude")),
            Some(PathBuf::from("/data/codex")),
        );

        let folder = Codex::default().default_config_dir(&env);

        assert_eq!(folder, PathBuf::from("/data/codex"));
    }

    #[test]
    fn the_default_config_folder_is_dot_codex_in_the_home_folder() {
        let env = AgentEnv::new(
            PathBuf::from("/home/user"),
            Some(PathBuf::from("/data/claude")),
            None,
        );

        let folder = Codex::default().default_config_dir(&env);

        assert_eq!(folder, PathBuf::from("/home/user/.codex"));
    }

    #[test]
    fn the_default_profile_starts_codex_with_no_variables() {
        let profile = default_profile();

        let new_session = Codex::default().launch(&profile, None);
        let resumed = Codex::default().launch(&profile, Some(&session_id()));

        assert_eq!(new_session, AgentCommand::new("codex"));
        assert_eq!(
            resumed,
            AgentCommand::new("codex").with_arg("resume").with_arg(ID)
        );
    }

    #[test]
    fn an_added_profile_starts_codex_with_its_config_folder() {
        let profile = added_profile();

        let new_session = Codex::default().launch(&profile, None);
        let resumed = Codex::default().launch(&profile, Some(&session_id()));

        assert_eq!(
            new_session,
            AgentCommand::new("codex")
                .with_env("CODEX_HOME", "/home/user/.codex-work")
        );
        assert_eq!(
            resumed,
            AgentCommand::new("codex")
                .with_arg("resume")
                .with_arg(ID)
                .with_env("CODEX_HOME", "/home/user/.codex-work")
        );
    }

    #[test]
    fn a_default_profile_moved_to_another_folder_starts_codex_with_it() {
        let profile = Profile::default_for(
            DESCRIPTOR.id(),
            "Codex",
            PathBuf::from("/data/codex"),
        )
        .with_agent_default_dir(Path::new("/home/user/.codex"));

        let new_session = Codex::default().launch(&profile, None);
        let login = Codex::default().login(&profile);

        assert_eq!(
            new_session,
            AgentCommand::new("codex").with_env("CODEX_HOME", "/data/codex")
        );
        assert_eq!(
            login,
            AgentCommand::new("codex")
                .with_arg("login")
                .with_env("CODEX_HOME", "/data/codex")
        );
    }

    #[test]
    fn a_profile_in_the_default_folder_starts_codex_with_no_variables() {
        let profile = codex_profile(Path::new("/home/user/.codex"))
            .with_agent_default_dir(Path::new("/home/user/.codex"));

        let command = Codex::default().launch(&profile, None);

        assert_eq!(command, AgentCommand::new("codex"));
    }

    #[test]
    fn login_runs_codex_login_in_the_config_folder_of_the_profile() {
        assert_eq!(
            Codex::default().login(&default_profile()),
            AgentCommand::new("codex").with_arg("login")
        );
        assert_eq!(
            Codex::default().login(&added_profile()),
            AgentCommand::new("codex")
                .with_arg("login")
                .with_env("CODEX_HOME", "/home/user/.codex-work")
        );
    }

    #[test]
    fn sessions_limits_and_spend_come_from_their_sources() {
        let codex = Codex::default();

        let session_sources: Vec<_> = codex
            .session_sources()
            .iter()
            .map(|source| source.id())
            .collect();

        assert_eq!(session_sources, [SourceId::SessionFiles]);
        let limit_sources: Vec<_> = codex
            .limit_sources(no_programs(), no_terminals())
            .iter()
            .map(|source| source.id())
            .collect();
        assert_eq!(
            limit_sources,
            [
                SourceId::ChatGptUsageApi,
                SourceId::CodexAppServer,
                SourceId::LatestCodexSession
            ]
        );
        assert!(codex.spend_counter().is_some());
    }

    #[test]
    fn what_was_read_is_remembered_between_two_refreshes() {
        let temp = TempDir::new();
        let config_dir = temp.path().join(".codex-work");
        let path = config_dir
            .join("sessions/2026/11/16")
            .join(format!("rollout-2026-11-16T10-00-00-{ID}.jsonl"));
        write_file(&path, &first_line("/home/user/aaaa"));
        set_file_time(&path, at(1_794_823_200));
        let profile = codex_profile(&config_dir);
        let codex = Codex::default();

        let first = codex.session_sources()[0].sessions(&profile);
        write_file(&path, &first_line("/home/user/bbbb"));
        set_file_time(&path, at(1_794_823_200));
        let second = codex.session_sources()[0].sessions(&profile);
        let fresh = Codex::default().session_sources()[0].sessions(&profile);

        let folder = |sessions: &[crate::session::AgentSession]| {
            sessions[0].folder().to_path_buf()
        };
        let first = first.expect("the files are read");
        let second = second.expect("the files are read");
        let fresh = fresh.expect("the files are read");
        assert_eq!(folder(&first), PathBuf::from("/home/user/aaaa"));
        assert_eq!(folder(&second), PathBuf::from("/home/user/aaaa"));
        assert_eq!(folder(&fresh), PathBuf::from("/home/user/bbbb"));
    }
}
