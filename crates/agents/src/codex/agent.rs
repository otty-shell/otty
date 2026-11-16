//! Codex as an [`Agent`]: its name, its default config folder and the
//! commands that start it.

use std::path::PathBuf;
use std::sync::Arc;

use super::cache::SessionCache;
use super::detect;
use super::sessions::SessionFiles;
use crate::agent::{Agent, AgentDescriptor, AgentId};
use crate::command::{AgentCommand, profile_command};
use crate::env::AgentEnv;
use crate::limits::LimitSource;
use crate::profile::{Profile, ProfileStatus};
use crate::session::{SessionId, SessionSource};
use crate::spend::SpendCounter;

const DESCRIPTOR: AgentDescriptor =
    AgentDescriptor::new(AgentId::new("codex"), "Codex", "codex");
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
            None => env.home().join(".codex"),
        }
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

    fn limit_sources(&self) -> Vec<Box<dyn LimitSource>> {
        Vec::new()
    }

    fn spend_counter(&self) -> Option<Box<dyn SpendCounter>> {
        None
    }
}

/// `codex` of `profile`, with `CODEX_HOME` for an added profile.
fn start_command(profile: &Profile) -> AgentCommand {
    profile_command(profile, PROGRAM, CONFIG_DIR_VARIABLE)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::testing::{
        TempDir, at, codex_profile, set_file_time, write_file,
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
    fn sessions_come_from_the_files_and_there_are_no_limits_or_spend_yet() {
        let codex = Codex::default();

        let session_sources: Vec<_> = codex
            .session_sources()
            .iter()
            .map(|source| source.name())
            .collect();

        assert_eq!(session_sources, ["session files"]);
        assert!(codex.limit_sources().is_empty());
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
