//! The reads of Agents mode: the profiles with their state, and the
//! sessions of one profile. They read files and may start the user's shell,
//! so each runs on a thread of its own.

use std::path::{Path, PathBuf};

use iced::Task;
use iced::futures::channel::oneshot;
use otty_agents::{
    Agent, AgentEnv, AgentId, AgentSession, Profile, ProfileStatus,
};
use otty_app_agents::{
    AgentList, AgentsIntent, LoadedProfile, LoadedProfiles, LoadedSessions,
};

use crate::profiles_file::read_saved;
use crate::program_lookup::ProgramLookup;

/// Read the profiles in the background: the saved ones in `agents.json` in
/// `config_dir` and the default profile of each of `agents`, each with its
/// state. A program is looked up through the user's shell at `shell`. The
/// answer comes back as [`AgentsIntent::ProfilesLoaded`].
///
/// # Example
///
/// ```
/// use std::path::{Path, PathBuf};
/// use std::sync::Arc;
///
/// use otty_agents::AgentEnv;
///
/// let read = otty_app_agents_io::load_profiles(
///     Arc::from(otty_agents::known_agents()),
///     AgentEnv::new(PathBuf::from("/home/user"), None, None),
///     Path::new("/home/user/.config/otty"),
///     "/bin/zsh",
/// );
///
/// assert_eq!(read.units(), 1);
/// ```
pub fn load_profiles(
    agents: AgentList,
    env: AgentEnv,
    config_dir: &Path,
    shell: &str,
) -> Task<AgentsIntent> {
    let config_dir = config_dir.to_path_buf();
    let shell = shell.to_string();

    Task::perform(
        on_thread(move || read_profiles(&agents, &env, &config_dir, &shell)),
        |loaded| {
            let loaded = loaded.unwrap_or_else(|| {
                log::warn!("reading the agent profiles stopped");
                LoadedProfiles {
                    profiles: Vec::new(),
                    saved: None,
                    problem: None,
                }
            });
            AgentsIntent::ProfilesLoaded(loaded)
        },
    )
}

/// Read the sessions of `profile` in the background, with the agent of
/// `agents` it belongs to. The answer comes back as
/// [`AgentsIntent::SessionsLoaded`].
pub fn load_sessions(
    agents: AgentList,
    profile: Profile,
) -> Task<AgentsIntent> {
    let fallback = profile.clone();

    Task::perform(
        on_thread(move || read_sessions(&agents, profile)),
        move |loaded| {
            let loaded = loaded.unwrap_or_else(|| {
                log::warn!(
                    "reading the sessions of {} stopped",
                    fallback.name()
                );
                LoadedSessions {
                    profile: fallback,
                    sessions: Ok(Vec::new()),
                    missing_folders: Vec::new(),
                }
            });
            AgentsIntent::SessionsLoaded(loaded)
        },
    )
}

/// Find the folders directly inside the home folder of `env` that look like
/// a config folder of the agent `agent` of `agents` and are not one of
/// `profiles` yet, in the background. The answer comes back as
/// [`AgentsIntent::CandidatesLoaded`].
pub fn load_candidates(
    agents: AgentList,
    env: AgentEnv,
    agent: AgentId,
    profiles: Vec<Profile>,
) -> Task<AgentsIntent> {
    Task::perform(
        on_thread(move || find_candidates(&agents, &env, agent, &profiles)),
        move |folders| AgentsIntent::CandidatesLoaded {
            agent,
            folders: folders.unwrap_or_default(),
        },
    )
}

/// The folders offered for a new profile of `agent`; none for an agent
/// OTTY does not know.
fn find_candidates(
    agents: &[Box<dyn Agent>],
    env: &AgentEnv,
    agent: AgentId,
    profiles: &[Profile],
) -> Vec<PathBuf> {
    let agent = agents.iter().find(|known| known.descriptor().id() == agent);
    let Some(agent) = agent else {
        return Vec::new();
    };

    otty_agents::candidate_config_dirs(agent.as_ref(), env, profiles)
}

/// The profiles to show, each with its state.
fn read_profiles(
    agents: &[Box<dyn Agent>],
    env: &AgentEnv,
    config_dir: &Path,
    shell: &str,
) -> LoadedProfiles {
    let (saved, problem) = read_saved(config_dir, agents);
    let lookup = ProgramLookup::new(shell);
    let content = problem.is_none().then(|| saved.clone());

    let found = otty_agents::profiles(agents, env, &saved, |program| {
        lookup.is_found(program)
    });
    let profiles = found
        .into_iter()
        .filter_map(|profile| {
            let agent = agents
                .iter()
                .find(|agent| agent.descriptor().id() == profile.agent())?;
            let status = status_of(agent.as_ref(), &profile, &lookup);
            Some(LoadedProfile { profile, status })
        })
        .collect();

    LoadedProfiles {
        profiles,
        saved: content,
        problem,
    }
}

/// The sessions of `profile` from the first source of its agent that
/// answers, and the folders of those sessions that no longer exist.
fn read_sessions(
    agents: &[Box<dyn Agent>],
    profile: Profile,
) -> LoadedSessions {
    let agent = agents
        .iter()
        .find(|agent| agent.descriptor().id() == profile.agent());
    let Some(agent) = agent else {
        return LoadedSessions {
            profile,
            sessions: Err(Vec::new()),
            missing_folders: Vec::new(),
        };
    };

    let sessions =
        otty_agents::read_sessions(&agent.session_sources(), &profile)
            .map(|answer| answer.into_value());
    let missing_folders = match &sessions {
        Ok(sessions) => missing_folders(sessions),
        Err(_) => Vec::new(),
    };

    LoadedSessions {
        profile,
        sessions,
        missing_folders,
    }
}

/// The state of `profile` of `agent`. The agent's program is started by
/// its name, so it must be found by `lookup`; the agent checks the rest.
fn status_of(
    agent: &dyn Agent,
    profile: &Profile,
    lookup: &ProgramLookup<'_>,
) -> ProfileStatus {
    let command = agent.launch(profile, None);
    if !lookup.is_found(command.program()) {
        return ProfileStatus::ProgramNotFound;
    }

    agent.detect(profile)
}

/// The folders of `sessions` that no longer exist, each once.
fn missing_folders(sessions: &[AgentSession]) -> Vec<PathBuf> {
    let mut folders: Vec<PathBuf> = sessions
        .iter()
        .map(|session| session.folder().to_path_buf())
        .collect();
    folders.sort();
    folders.dedup();

    folders.retain(|folder| !folder.is_dir());
    folders
}

/// Run `work` on a thread of its own and wait for its answer without
/// holding up the runtime. `None` when the thread ended without one.
pub(crate) async fn on_thread<T: Send + 'static>(
    work: impl FnOnce() -> T + Send + 'static,
) -> Option<T> {
    let (sender, receiver) = oneshot::channel();
    std::thread::spawn(move || {
        let _ = sender.send(work());
    });

    receiver.await.ok()
}

#[cfg(all(test, unix))]
pub(crate) mod tests {
    use std::sync::Arc;

    use iced::futures::StreamExt;
    use iced::futures::executor::block_on;
    use iced_test::runtime::Action;
    use iced_test::runtime::task::into_stream;
    use otty_agents::{AgentId, BillingKind, known_agents};
    use otty_app_testing::temp_dir::TempDir;

    use super::*;
    use crate::testing::executable_script;

    const SESSION: &str = "3f1c2b7e-8d4a-4e6f-9b0c-1a2d3e4f5a6b";

    /// A made-up shell that answers "found" for every program, or for
    /// none, without reading the user's start files.
    fn shell(folder: &TempDir, finds: bool) -> String {
        let code = if finds { 0 } else { 1 };

        executable_script(folder.path(), "fake-shell", &format!("exit {code}"))
            .display()
            .to_string()
    }

    fn write(path: &Path, text: &str) {
        std::fs::create_dir_all(path.parent().expect("a parent"))
            .expect("the folder is made");
        std::fs::write(path, text).expect("the file is written");
    }

    /// A made-up Claude Code session in `folder`, kept in `config_dir`.
    fn write_session(config_dir: &Path, id: &str, folder: &Path) {
        let line = format!(
            r#"{{"type":"user","cwd":"{}","sessionId":"{id}","timestamp":"2026-11-16T10:00:00.000Z","message":{{"role":"user","content":"Fix the tabs"}}}}"#,
            folder.display()
        );
        write(
            &config_dir
                .join("projects")
                .join("-work")
                .join(format!("{id}.jsonl")),
            &line,
        );
    }

    fn statuses(loaded: &LoadedProfiles) -> Vec<(&str, &ProfileStatus)> {
        loaded
            .profiles
            .iter()
            .map(|entry| (entry.profile.name(), &entry.status))
            .collect()
    }

    /// Run `task` to its end and return the intents it answers with.
    pub(crate) fn run(task: Task<AgentsIntent>) -> Vec<AgentsIntent> {
        let stream = into_stream(task).expect("a task");
        let actions: Vec<Action<AgentsIntent>> = block_on(stream.collect());

        actions
            .into_iter()
            .filter_map(|action| match action {
                Action::Output(intent) => Some(intent),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn each_profile_has_its_state_and_a_program_not_found_says_so() {
        let home = TempDir::new("agents-read-found");
        let claude_dir = home.path().join(".claude");
        write(
            &claude_dir.join(".credentials.json"),
            r#"{"claudeAiOauth":{"accessToken":"made-up"}}"#,
        );
        let env = AgentEnv::new(home.path().to_path_buf(), None, None);
        let finds = shell(&home, true);
        let other = TempDir::new("agents-read-none");
        let finds_nothing = shell(&other, false);

        let found = read_profiles(&known_agents(), &env, home.path(), &finds);
        let not_found =
            read_profiles(&known_agents(), &env, home.path(), &finds_nothing);

        let connected = ProfileStatus::Connected {
            account: None,
            plan: None,
            billing: Some(BillingKind::UsageWindows),
        };
        assert_eq!(
            statuses(&found),
            [
                ("Claude Code", &connected),
                ("Codex", &ProfileStatus::ConfigDirNotFound),
            ]
        );
        assert_eq!(
            statuses(&not_found),
            [("Claude Code", &ProfileStatus::ProgramNotFound)]
        );
        assert_eq!(found.problem, None);
    }

    #[test]
    fn saved_profiles_are_read_from_agents_json_in_the_config_folder() {
        let home = TempDir::new("agents-read-saved");
        let config_dir = home.path().join("otty");
        let work = home.path().join(".codex-work");
        std::fs::create_dir_all(&work).expect("the folder is made");
        write(
            &config_dir.join("agents.json"),
            &format!(
                r#"{{"default_profiles":{{"codex":{{"name":"Codex · Home"}}}},"added_profiles":[{{"agent":"codex","name":"Codex · Work","config_dir":"{}"}}]}}"#,
                work.display()
            ),
        );
        let env = AgentEnv::new(home.path().to_path_buf(), None, None);
        let finds = shell(&home, true);

        let loaded = read_profiles(&known_agents(), &env, &config_dir, &finds);

        let names: Vec<_> = loaded
            .profiles
            .iter()
            .map(|entry| entry.profile.name())
            .collect();
        assert_eq!(names, ["Claude Code", "Codex · Home", "Codex · Work"]);
        assert_eq!(loaded.profiles[2].status, ProfileStatus::NotLoggedIn);
    }

    #[test]
    fn an_old_agents_json_with_hidden_and_a_program_shows_the_profiles_found_by_name()
     {
        let home = TempDir::new("agents-read-old");
        let config_dir = home.path().join("otty");
        let work = home.path().join(".codex-work");
        std::fs::create_dir_all(&work).expect("the folder is made");
        write(
            &config_dir.join("agents.json"),
            &format!(
                r#"{{"default_profiles":{{"codex":{{"hidden":true,"program":"/no/such/codex"}}}},"added_profiles":[{{"agent":"codex","name":"Codex · Work","config_dir":"{}","program":"/no/such/codex","hidden":true}}]}}"#,
                work.display()
            ),
        );
        let env = AgentEnv::new(home.path().to_path_buf(), None, None);
        let finds = shell(&home, true);

        let loaded = read_profiles(&known_agents(), &env, &config_dir, &finds);

        assert_eq!(
            statuses(&loaded)[1..],
            [
                ("Codex", &ProfileStatus::ConfigDirNotFound),
                ("Codex · Work", &ProfileStatus::NotLoggedIn),
            ]
        );
        assert_eq!(loaded.problem, None);
    }

    #[test]
    fn a_broken_agents_json_is_told_and_the_defaults_are_used() {
        let home = TempDir::new("agents-read-broken");
        write(&home.path().join("agents.json"), "{ not json");
        std::fs::create_dir_all(home.path().join(".claude")).expect("folder");
        let env = AgentEnv::new(home.path().to_path_buf(), None, None);
        let finds_nothing = shell(&home, false);

        let loaded =
            read_profiles(&known_agents(), &env, home.path(), &finds_nothing);

        assert!(loaded.problem.is_some());
        assert_eq!(loaded.profiles.len(), 1);
    }

    #[test]
    fn found_folders_are_those_of_the_agent_no_profile_uses() {
        let home = TempDir::new("agents-candidates");
        for name in [".claude-work", ".claude-used"] {
            let folder = home.path().join(name);
            std::fs::create_dir_all(folder.join("projects")).expect("folder");
            write(&folder.join(".claude.json"), "{}");
        }
        let env = AgentEnv::new(home.path().to_path_buf(), None, None);
        let claude = AgentId::new("claude-code");
        let used =
            Profile::added(claude, "Used", home.path().join(".claude-used"));

        let found = find_candidates(
            &known_agents(),
            &env,
            claude,
            std::slice::from_ref(&used),
        );
        let unknown = find_candidates(
            &known_agents(),
            &env,
            AgentId::new("stranger"),
            &[],
        );
        let answers = run(load_candidates(
            Arc::from(known_agents()),
            env,
            claude,
            vec![used],
        ));

        assert_eq!(found, [home.path().join(".claude-work")]);
        assert!(unknown.is_empty());
        assert_eq!(
            answers,
            [AgentsIntent::CandidatesLoaded {
                agent: claude,
                folders: vec![home.path().join(".claude-work")],
            }]
        );
    }

    #[test]
    fn the_saved_content_comes_with_the_profiles_unless_it_was_broken() {
        let home = TempDir::new("agents-read-content");
        write(
            &home.path().join("agents.json"),
            r#"{"default_profiles":{"codex":{"name":"Codex · Home"}}}"#,
        );
        let broken = TempDir::new("agents-read-content-broken");
        write(&broken.path().join("agents.json"), "{ not json");
        let env = AgentEnv::new(home.path().to_path_buf(), None, None);
        let finds = shell(&home, true);

        let loaded = read_profiles(&known_agents(), &env, home.path(), &finds);
        let unread =
            read_profiles(&known_agents(), &env, broken.path(), &finds);

        let saved = loaded.saved.expect("the file was read");
        assert!(
            saved
                .default_changes_of(AgentId::new("codex"))
                .is_some_and(|changes| changes.name() == Some("Codex · Home"))
        );
        assert_eq!(unread.saved, None);
        assert!(unread.problem.is_some());
    }

    #[test]
    fn the_sessions_of_a_profile_come_with_their_missing_folders() {
        let home = TempDir::new("agents-sessions");
        let config_dir = home.path().join(".claude");
        let project = home.path().join("otty");
        let gone = home.path().join("gone");
        std::fs::create_dir_all(&project).expect("the project folder");
        write_session(&config_dir, SESSION, &project);
        write_session(
            &config_dir,
            "9e8d7c6b-5a4f-4e3d-8c2b-1a0f9e8d7c6b",
            &gone,
        );
        let profile = Profile::default_for(
            AgentId::new("claude-code"),
            "Claude Code",
            config_dir,
        );

        let loaded = read_sessions(&known_agents(), profile.clone());

        assert_eq!(loaded.profile, profile);
        let sessions = loaded.sessions.expect("the sessions are read");
        assert_eq!(sessions.len(), 2);
        assert_eq!(loaded.missing_folders, [gone]);
    }

    #[test]
    fn a_profile_of_an_unknown_agent_has_no_way_to_read_sessions() {
        let profile = Profile::added(
            AgentId::new("stranger"),
            "Stranger",
            PathBuf::from("/home/user/.stranger"),
        );

        let loaded = read_sessions(&known_agents(), profile);

        assert_eq!(loaded.sessions, Err(Vec::new()));
    }

    #[test]
    fn the_reads_run_in_the_background_and_answer_with_an_intent() {
        let home = TempDir::new("agents-tasks");
        let config_dir = home.path().join(".claude");
        std::fs::create_dir_all(config_dir.join("projects")).expect("folder");
        let env = AgentEnv::new(home.path().to_path_buf(), None, None);
        let agents: AgentList = Arc::from(known_agents());
        let finds_nothing = shell(&home, false);
        let profile = Profile::default_for(
            AgentId::new("claude-code"),
            "Claude Code",
            config_dir,
        );

        let profiles = run(load_profiles(
            Arc::clone(&agents),
            env,
            home.path(),
            &finds_nothing,
        ));
        let sessions = run(load_sessions(agents, profile.clone()));

        assert!(matches!(
            profiles.as_slice(),
            [AgentsIntent::ProfilesLoaded(loaded)] if loaded.profiles.len() == 1
        ));
        assert_eq!(
            sessions,
            [AgentsIntent::SessionsLoaded(LoadedSessions {
                profile,
                sessions: Ok(Vec::new()),
                missing_folders: Vec::new(),
            })]
        );
    }
}
