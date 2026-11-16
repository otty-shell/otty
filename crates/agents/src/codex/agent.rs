//! Codex as an [`Agent`]: its name, its default config folder and the
//! commands that start it.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::app_server::AppServer;
use super::cache::SessionCache;
use super::detect;
use super::session_limits::SessionRecord;
use super::sessions::SessionFiles;
use super::usage::UsageRequest;
use crate::agent::{Agent, AgentDescriptor, AgentId};
use crate::command::{AgentCommand, profile_command};
use crate::env::AgentEnv;
use crate::https::UreqClient;
use crate::limits::LimitSource;
use crate::profile::{Profile, ProfileStatus};
use crate::program::StartProgram;
use crate::session::{SessionId, SessionSource};
use crate::spend::SpendCounter;

const DESCRIPTOR: AgentDescriptor =
    AgentDescriptor::new(AgentId::new("codex"), "Codex", "codex", ".codex");
/// The name of the program, looked up on `PATH`.
const PROGRAM: &str = "codex";
/// The variable that points Codex at a config folder.
const CONFIG_DIR_VARIABLE: &str = "CODEX_HOME";

/// Codex, OpenAI's coding agent.
///
/// It remembers what it read from the session files, so that the next
/// refresh reads only the files that changed. The sources it gives share
/// that memory with it, so the app keeps one `Codex` for the whole run: a
/// new one reads every file again.
///
/// ```
/// use otty_agents::{Agent, Codex};
///
/// assert_eq!(Codex::default().descriptor().name(), "Codex");
/// ```
#[derive(Debug, Default)]
pub struct Codex {
    read_files: Arc<SessionCache>,
}

impl Agent for Codex {
    fn descriptor(&self) -> &AgentDescriptor {
        &DESCRIPTOR
    }

    fn default_config_dir(&self, env: &AgentEnv) -> PathBuf {
        match env.codex_home() {
            Some(codex_home) => codex_home.to_path_buf(),
            None => env.home().join(DESCRIPTOR.config_dir_name()),
        }
    }

    /// A folder with an `auth.json` file or a `sessions/` folder inside.
    fn looks_like_config_dir(&self, dir: &Path) -> bool {
        dir.join("auth.json").is_file() || dir.join("sessions").is_dir()
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
    ) -> Vec<Box<dyn LimitSource>> {
        let request = UsageRequest::new(Box::new(UreqClient::new()));
        let app_server = AppServer::new(programs);

        vec![
            Box::new(request),
            Box::new(app_server),
            Box::new(SessionRecord),
        ]
    }

    fn spend_counter(&self) -> Option<Box<dyn SpendCounter>> {
        None
    }
}

/// `codex` of `profile`, with `CODEX_HOME` when its folder is not the
/// default one.
pub(super) fn start_command(profile: &Profile) -> AgentCommand {
    profile_command(profile, PROGRAM, CONFIG_DIR_VARIABLE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{
        TempDir, at, codex_profile, no_programs, set_file_time, write_file,
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
    fn the_program_path_of_the_profile_is_used_instead_of_codex() {
        let profile =
            default_profile().with_program(PathBuf::from("/opt/codex/codex"));

        let command = Codex::default().launch(&profile, None);

        assert_eq!(command, AgentCommand::new("/opt/codex/codex"));
    }

    #[test]
    fn a_folder_with_a_login_or_sessions_looks_like_codex() {
        let temp = TempDir::new();
        let with_login = temp.path().join(".codex-a");
        let with_sessions = temp.path().join(".codex-b");
        write_file(&with_login.join("auth.json"), "{}");
        std::fs::create_dir_all(with_sessions.join("sessions"))
            .expect("create folder");
        let codex = Codex::default();

        assert!(codex.looks_like_config_dir(&with_login));
        assert!(codex.looks_like_config_dir(&with_sessions));
    }

    #[test]
    fn a_folder_with_neither_does_not_look_like_codex() {
        let temp = TempDir::new();
        let empty = temp.path().join(".codex-a");
        let wrong_kinds = temp.path().join(".codex-b");
        std::fs::create_dir_all(&empty).expect("create folder");
        std::fs::create_dir_all(wrong_kinds.join("auth.json"))
            .expect("create folder");
        write_file(&wrong_kinds.join("sessions"), "");
        let codex = Codex::default();

        assert!(!codex.looks_like_config_dir(&empty));
        assert!(!codex.looks_like_config_dir(&wrong_kinds));
        assert!(!codex.looks_like_config_dir(&temp.path().join("none")));
    }

    #[test]
    fn sessions_come_from_the_files_limits_from_three_sources_and_no_spend_yet()
    {
        let codex = Codex::default();

        let session_sources: Vec<_> = codex
            .session_sources()
            .iter()
            .map(|source| source.name())
            .collect();

        assert_eq!(session_sources, ["session files"]);
        let limit_sources: Vec<_> = codex
            .limit_sources(no_programs())
            .iter()
            .map(|source| source.name())
            .collect();
        assert_eq!(
            limit_sources,
            ["usage request", "app-server", "session file"]
        );
        assert!(codex.spend_counter().is_none());
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
