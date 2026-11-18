//! The reads of Agents mode: the profiles with their state, and the
//! sessions of one profile. They read files and may start the user's shell,
//! so each runs on a thread of its own.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use iced::Task;
use otty_agents::{Agent, AgentEnv, AgentSession, Profile, SourceId, agent_of};
use otty_app_agents::{
    AgentList, AgentsIntent, LoadedProfile, LoadedProfiles, LoadedSessions,
    SessionsRead,
};

use crate::background::{failed_because_stopped, on_own_thread};
use crate::profiles_file::read_saved;
use crate::program_lookup::ProgramLookup;
use crate::time_zone::user_time_zone;

/// Read the profiles in the background: the saved ones in `agents.json` in
/// `config_dir` and the default profile of each of `agents`, each with its
/// state. A program is looked up through the user's shell at `shell`. The
/// answer comes back as [`AgentsIntent::ProfilesLoaded`], or as
/// [`AgentsIntent::ProfilesReadStopped`] when the read stops before it
/// answers, so the profiles shown stay.
///
/// # Example
///
/// ```
/// use std::path::{Path, PathBuf};
/// use std::sync::Arc;
///
/// use otty_agents::AgentEnv;
/// use otty_app_agents::AgentList;
///
/// let agents: AgentList = Arc::from(otty_agents::known_agents());
/// let read = otty_app_agents_io::load_profiles(
///     &agents,
///     &AgentEnv::new(PathBuf::from("/home/user")),
///     Path::new("/home/user/.config/otty"),
///     "/bin/zsh",
/// );
///
/// assert_eq!(read.units(), 1);
/// ```
pub fn load_profiles(
    agents: &AgentList,
    env: &AgentEnv,
    config_dir: &Path,
    shell: &str,
) -> Task<AgentsIntent> {
    let agents = Arc::clone(agents);
    let env = env.clone();
    let config_dir = config_dir.to_path_buf();
    let shell = shell.to_string();

    on_own_thread(
        String::from("reading the agent profiles"),
        move || {
            let loaded = read_profiles(&agents, &env, &config_dir, &shell);
            AgentsIntent::ProfilesLoaded(loaded)
        },
        || AgentsIntent::ProfilesReadStopped,
        |answer| answer,
    )
}

/// Read the sessions of `profile` in the background, with the agent of
/// `agents` it belongs to. The answer comes back as
/// [`AgentsIntent::SessionsLoaded`].
pub fn load_sessions(
    agents: &AgentList,
    profile: Profile,
) -> Task<AgentsIntent> {
    let sources = session_source_ids(agents, &profile);
    let task = format!("reading the sessions of {}", profile.name());
    let agents = Arc::clone(agents);
    let fallback = profile.clone();

    on_own_thread(
        task,
        move || read_sessions(&agents, profile),
        move || stopped_sessions(fallback, sources),
        AgentsIntent::SessionsLoaded,
    )
}

/// The profiles to show, each with its state, and the user's time zone,
/// which the dashboard writes its times in.
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
            let agent = agent_of(agents, profile.agent())?;
            let status =
                otty_agents::profile_status(agent, &profile, |program| {
                    lookup.is_found(program)
                });
            Some(LoadedProfile { profile, status })
        })
        .collect();

    LoadedProfiles {
        profiles,
        saved: content,
        problem,
        time_zone: user_time_zone(),
        read_at: SystemTime::now(),
    }
}

/// The sessions of `profile` from the first source of its agent that
/// answers, and the folders of those sessions that no longer exist.
fn read_sessions(
    agents: &[Box<dyn Agent>],
    profile: Profile,
) -> LoadedSessions {
    let Some(agent) = agent_of(agents, profile.agent()) else {
        return LoadedSessions {
            profile,
            sessions: SessionsRead::NoWay,
            missing_folders: Vec::new(),
        };
    };

    let walk = otty_agents::read_sessions(&agent.session_sources(), &profile)
        .map(|answer| answer.into_value());
    let sessions = SessionsRead::from_walk(walk);
    let missing_folders = match &sessions {
        SessionsRead::Read(sessions) => missing_folders(sessions),
        _ => Vec::new(),
    };

    LoadedSessions {
        profile,
        sessions,
        missing_folders,
    }
}

/// The ids of the session sources of the agent of `profile`, best first.
/// An agent OTTY does not know has none.
fn session_source_ids(
    agents: &[Box<dyn Agent>],
    profile: &Profile,
) -> &'static [SourceId] {
    agent_of(agents, profile.agent())
        .map(|agent| agent.session_source_ids())
        .unwrap_or_default()
}

/// What a read of the sessions of `profile` that stopped before it ended
/// gives: each of `sources` failed because the read stopped, so the
/// sessions read before stay and the user sees why. With no sources the
/// profile has no way to read sessions.
fn stopped_sessions(profile: Profile, sources: &[SourceId]) -> LoadedSessions {
    LoadedSessions {
        profile,
        sessions: SessionsRead::from_walk(Err(failed_because_stopped(sources))),
        missing_folders: Vec::new(),
    }
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

#[cfg(all(test, unix))]
mod tests {
    use otty_agents::{
        AgentId, BillingKind, ProfileStatus, SourceFailure, known_agents,
    };
    use otty_app_testing::temp_dir::TempDir;

    use super::*;
    use crate::background::read_stopped;
    use crate::testing::{executable_script, run, write};

    const SESSION: &str = "3f1c2b7e-8d4a-4e6f-9b0c-1a2d3e4f5a6b";

    /// A made-up shell that answers "found" for every program, or for
    /// none, without reading the user's start files.
    fn shell(folder: &TempDir, finds: bool) -> String {
        let code = if finds { 0 } else { 1 };

        executable_script(folder.path(), "fake-shell", &format!("exit {code}"))
            .display()
            .to_string()
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

    #[test]
    fn each_profile_has_its_state_and_a_program_not_found_says_so() {
        let home = TempDir::new("agents-read-found");
        let claude_dir = home.path().join(".claude");
        write(
            &claude_dir.join(".credentials.json"),
            r#"{"claudeAiOauth":{"accessToken":"made-up"}}"#,
        );
        let env = AgentEnv::new(home.path().to_path_buf());
        let finds = shell(&home, true);
        let other = TempDir::new("agents-read-none");
        let finds_nothing = shell(&other, false);

        let before = SystemTime::now();

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
        assert!(
            found.read_at >= before,
            "the time of the read comes with it"
        );
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
        let env = AgentEnv::new(home.path().to_path_buf());
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
        let env = AgentEnv::new(home.path().to_path_buf());
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
        let env = AgentEnv::new(home.path().to_path_buf());
        let finds_nothing = shell(&home, false);

        let loaded =
            read_profiles(&known_agents(), &env, home.path(), &finds_nothing);

        assert!(loaded.problem.is_some());
        assert_eq!(loaded.profiles.len(), 1);
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
        let env = AgentEnv::new(home.path().to_path_buf());
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
        let SessionsRead::Read(sessions) = loaded.sessions else {
            panic!("the sessions are read");
        };
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

        assert_eq!(loaded.sessions, SessionsRead::NoWay);
    }

    #[test]
    fn the_reads_run_in_the_background_and_answer_with_an_intent() {
        let home = TempDir::new("agents-tasks");
        let config_dir = home.path().join(".claude");
        std::fs::create_dir_all(config_dir.join("projects")).expect("folder");
        let env = AgentEnv::new(home.path().to_path_buf());
        let agents: AgentList = Arc::from(known_agents());
        let finds_nothing = shell(&home, false);
        let profile = Profile::default_for(
            AgentId::new("claude-code"),
            "Claude Code",
            config_dir,
        );

        let profiles =
            run(load_profiles(&agents, &env, home.path(), &finds_nothing));
        let sessions = run(load_sessions(&agents, profile.clone()));

        assert!(matches!(
            profiles.as_slice(),
            [AgentsIntent::ProfilesLoaded(loaded)] if loaded.profiles.len() == 1
        ));
        assert_eq!(
            sessions,
            [AgentsIntent::SessionsLoaded(LoadedSessions {
                profile,
                sessions: SessionsRead::Read(Vec::new()),
                missing_folders: Vec::new(),
            })]
        );
    }

    /// A read of the sessions that stopped before it ended fails at each
    /// source because it stopped, so the sessions read before stay; it
    /// never says the profile has no sessions.
    #[test]
    fn a_sessions_read_that_stopped_fails_because_it_stopped() {
        let profile = Profile::default_for(
            AgentId::new("claude-code"),
            "Claude Code",
            PathBuf::from("/home/user/.claude"),
        );
        let stranger = Profile::added(
            AgentId::new("stranger"),
            "Stranger",
            PathBuf::from("/home/user/.stranger"),
        );

        let sources = session_source_ids(&known_agents(), &profile);
        let none = session_source_ids(&known_agents(), &stranger);

        let loaded = stopped_sessions(profile.clone(), sources);
        let unknown = stopped_sessions(stranger, none);

        assert_eq!(loaded.profile, profile);
        assert_eq!(
            loaded.sessions,
            SessionsRead::Failed(vec![SourceFailure::new(
                SourceId::SessionFiles,
                read_stopped()
            )])
        );
        assert_eq!(unknown.sessions, SessionsRead::NoWay);
    }
}
