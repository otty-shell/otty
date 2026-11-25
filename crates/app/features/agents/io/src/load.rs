//! The reads of Agents mode: the profiles with their state, and the
//! sessions of one profile. They read files and may start the user's shell,
//! so each runs on a thread of its own.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use iced::Task;
use otty_agents::{
    Agent, AgentEnv, AgentSession, PluginProvider, Profile, SourceId, agent_of,
};
use otty_app_agents::{
    AgentList, AgentsIntent, LoadedProfile, LoadedProfiles, LoadedSessions,
    SessionsRead,
};

use crate::background::{failed_because_stopped, on_own_thread};
use crate::plugin_parts::ProviderList;
use crate::profiles_file::read_saved;
use crate::program_lookup::ProgramLookup;
use crate::time_zone::user_time_zone;

/// Read the profiles in the background: the saved ones in `agents.json` in
/// `config_dir` and the default profile of each of `agents`, each with its
/// state, and the accounts each provider part of `providers` finds in the
/// shown profiles of its own plugin and in the folders of its own logins
/// in the folder of the installed plugins `plugins_folder`. A program is
/// looked up through the user's shell at `shell`. The answer comes back as
/// [`AgentsIntent::ProfilesLoaded`], or as
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
/// let agents: AgentList = Arc::from(Vec::new());
/// let providers: otty_app_agents_io::ProviderList = Arc::from(Vec::new());
/// let read = otty_app_agents_io::load_profiles(
///     &agents,
///     &providers,
///     &AgentEnv::new(PathBuf::from("/home/user")),
///     Path::new("/home/user/.config/otty"),
///     Path::new("/home/user/.local/share/otty/plugins"),
///     "/bin/zsh",
/// );
///
/// assert_eq!(read.units(), 1);
/// ```
pub fn load_profiles(
    agents: &AgentList,
    providers: &ProviderList,
    env: &AgentEnv,
    config_dir: &Path,
    plugins_folder: &Path,
    shell: &str,
) -> Task<AgentsIntent> {
    let agents = Arc::clone(agents);
    let providers = Arc::clone(providers);
    let env = env.clone();
    let config_dir = config_dir.to_path_buf();
    let plugins_folder = plugins_folder.to_path_buf();
    let shell = shell.to_string();

    on_own_thread(
        String::from("reading the agent profiles"),
        move || {
            let loaded = read_profiles(
                &agents,
                &providers,
                &env,
                &config_dir,
                &plugins_folder,
                &shell,
            );
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

/// The profiles to show, each with its state, the accounts the provider
/// parts of `providers` find in them (spec
/// 2026-11-23-accounts-apart-from-profiles, Decision 5), the profiles the
/// user hid, which are not checked and whose files are not read, and the
/// user's time zone, which the dashboard writes its times in.
fn read_profiles(
    agents: &[Box<dyn Agent>],
    providers: &[PluginProvider],
    env: &AgentEnv,
    config_dir: &Path,
    plugins_folder: &Path,
    shell: &str,
) -> LoadedProfiles {
    let (saved, problem) = read_saved(config_dir);
    let lookup = ProgramLookup::new(shell);
    let content = problem.is_none().then(|| saved.clone());

    let found = otty_agents::profiles(agents, env, &saved, |program| {
        lookup.is_found(program)
    });
    let (hidden_profiles, shown): (Vec<Profile>, Vec<Profile>) = found
        .into_iter()
        .partition(|profile| saved.is_profile_hidden(profile));
    let profiles: Vec<LoadedProfile> = shown
        .into_iter()
        .filter_map(|profile| {
            let agent = agent_of(agents, profile.agent())?;
            let status =
                otty_agents::profile_status(agent, &profile, |program| {
                    lookup.is_found(program)
                });
            let version = agent.version(&profile);
            Some(LoadedProfile {
                profile,
                status,
                version,
            })
        })
        .collect();
    let shown: Vec<Profile> = profiles
        .iter()
        .map(|loaded| loaded.profile.clone())
        .collect();
    let accounts = otty_agents::find_accounts(
        agents,
        providers,
        &shown,
        saved.own_logins(),
        plugins_folder,
    );

    LoadedProfiles {
        profiles,
        hidden_profiles,
        accounts,
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
    use otty_agents::{AgentId, ProfileStatus, SourceFailure};
    use otty_app_testing::temp_dir::TempDir;

    use super::*;
    use crate::background::read_stopped;
    use crate::testing::{
        EXAMPLE_AGENT, agents_with_example, executable_script,
        providers_with_example, run, write,
    };

    const SESSION: &str = "3f1c2b7e-8d4a-4e6f-9b0c-1a2d3e4f5a6b";

    /// A made-up login of the example agent: `auth.json` with a key.
    const LOGIN: &str =
        r#"{"api_key": "ex-made-up-key-0001", "email": "dev@example.com"}"#;

    /// A made-up shell that answers "found" for every program, or for
    /// none, without reading the user's start files.
    fn shell(folder: &TempDir, finds: bool) -> String {
        let code = if finds { 0 } else { 1 };

        executable_script(folder.path(), "fake-shell", &format!("exit {code}"))
            .display()
            .to_string()
    }

    /// A made-up session of the example agent in `folder`, kept in
    /// `config_dir`.
    fn write_session(config_dir: &Path, id: &str, folder: &Path) {
        let line = format!(
            r#"{{"type":"meta","version":"1.4.2","cwd":"{}","title":"Fix the tabs"}}"#,
            folder.display()
        );
        write(
            &config_dir.join("sessions").join(format!("{id}.jsonl")),
            &line,
        );
    }

    /// The entry of `agents.json` that adds the profile `name` of the
    /// example agent in `folder`.
    fn added(name: &str, folder: &Path) -> String {
        format!(
            r#"{{"agent":"{EXAMPLE_AGENT}","name":"{name}","config_dir":"{}"}}"#,
            folder.display()
        )
    }

    fn statuses(loaded: &LoadedProfiles) -> Vec<(&str, &ProfileStatus)> {
        loaded
            .profiles
            .iter()
            .map(|entry| (entry.profile.name(), &entry.status))
            .collect()
    }

    /// Issue 14: a plugin with only its agent part on reads the profile's
    /// folder: the profile is connected and its version comes from its
    /// files, which are never written. Spec
    /// 2026-11-23-accounts-apart-from-profiles, Decision 11: with no
    /// provider part, its logins are no accounts.
    #[test]
    fn a_plugin_with_only_its_agent_part_reads_the_folder_and_finds_no_account()
    {
        let home = TempDir::new("agents-read-agent-only");
        let folder = home.path().join(".example-agent");
        write(&folder.join("auth.json"), LOGIN);
        write(&folder.join("state.json"), r#"{"last_version": "1.4.2"}"#);
        let env = AgentEnv::new(home.path().to_path_buf());

        let loaded = read_profiles(
            &agents_with_example(home.path()),
            &[],
            &env,
            home.path(),
            home.path(),
            &shell(&home, true),
        );

        let example = loaded
            .profiles
            .iter()
            .find(|entry| entry.profile.agent().as_str() == EXAMPLE_AGENT)
            .expect("the Example Agent profile");
        assert_eq!(example.status, ProfileStatus::Connected);
        assert_eq!(example.version.as_deref(), Some("1.4.2"));
        assert!(
            loaded.accounts.accounts().is_empty(),
            "no provider part: its logins are no accounts"
        );
        assert_eq!(
            std::fs::read_to_string(folder.join("auth.json")).expect("read"),
            LOGIN,
            "the files are never written"
        );
    }

    #[test]
    fn each_profile_has_its_state_and_a_program_not_found_says_so() {
        let home = TempDir::new("agents-read-found");
        write(&home.path().join(".example-agent/auth.json"), LOGIN);
        let empty_home = TempDir::new("agents-read-empty-home");
        let env = AgentEnv::new(home.path().to_path_buf());
        let empty_env = AgentEnv::new(empty_home.path().to_path_buf());
        let finds = shell(&home, true);
        let other = TempDir::new("agents-read-none");
        let finds_nothing = shell(&other, false);

        let before = SystemTime::now();

        let found = read_profiles(
            &agents_with_example(home.path()),
            &[],
            &env,
            home.path(),
            home.path(),
            &finds,
        );
        let not_found = read_profiles(
            &agents_with_example(home.path()),
            &[],
            &env,
            home.path(),
            home.path(),
            &finds_nothing,
        );
        let no_folder = read_profiles(
            &agents_with_example(empty_home.path()),
            &[],
            &empty_env,
            empty_home.path(),
            empty_home.path(),
            &finds,
        );

        assert_eq!(
            statuses(&found),
            [("Example Agent", &ProfileStatus::Connected)]
        );
        assert_eq!(
            statuses(&not_found),
            [("Example Agent", &ProfileStatus::ProgramNotFound)]
        );
        assert_eq!(
            statuses(&no_folder),
            [("Example Agent", &ProfileStatus::ConfigDirNotFound)]
        );
        assert_eq!(found.problem, None);
        assert!(
            found.read_at >= before,
            "the time of the read comes with it"
        );
    }

    /// Spec 2026-11-23, Decision 4: with `~/.example-agent` and
    /// `~/.example-agent-work` on disk and nothing added, only
    /// `~/.example-agent` is a profile; an extra folder is one once the
    /// user adds it, with the version its files name. The read of the
    /// profiles never starts an agent's program, so a profile whose files
    /// name no version has none yet.
    #[test]
    fn only_the_default_and_the_added_folders_are_profiles() {
        let home = TempDir::new("agents-read-versions");
        let config_dir = home.path().join("otty");
        write(&home.path().join(".example-agent/auth.json"), LOGIN);
        let work = home.path().join(".example-agent-work");
        write(&work.join("state.json"), r#"{"last_version": "1.4.2"}"#);
        let env = AgentEnv::new(home.path().to_path_buf());
        let prints_a_version = executable_script(
            home.path(),
            "version-shell",
            "echo '2.1.0 (Example Agent)'",
        )
        .display()
        .to_string();
        let agents = agents_with_example(home.path());

        let before = read_profiles(
            &agents,
            &[],
            &env,
            &config_dir,
            home.path(),
            &prints_a_version,
        );
        write(
            &config_dir.join("agents.json"),
            &format!(r#"{{"added_profiles":[{}]}}"#, added("Work", &work)),
        );
        let after = read_profiles(
            &agents,
            &[],
            &env,
            &config_dir,
            home.path(),
            &prints_a_version,
        );

        let folders = |loaded: &LoadedProfiles| {
            loaded
                .profiles
                .iter()
                .map(|entry| {
                    let folder = entry.profile.config_dir();
                    let folder = folder.strip_prefix(home.path()).ok();
                    (folder.map(Path::to_path_buf), entry.version.clone())
                })
                .collect::<Vec<_>>()
        };
        let named = |folder: &str, version: Option<&str>| {
            (Some(PathBuf::from(folder)), version.map(String::from))
        };
        assert_eq!(folders(&before), [named(".example-agent", None)]);
        assert_eq!(
            folders(&after),
            [
                named(".example-agent", None),
                named(".example-agent-work", Some("1.4.2")),
            ]
        );
    }

    #[test]
    fn saved_profiles_are_read_from_agents_json_in_the_config_folder() {
        let home = TempDir::new("agents-read-saved");
        let config_dir = home.path().join("otty");
        let work = home.path().join(".example-agent-work");
        write(&work.join("state.json"), "{}");
        write(
            &config_dir.join("agents.json"),
            &format!(
                r#"{{"default_profiles":{{"{EXAMPLE_AGENT}":{{"name":"Example (Home)"}}}},"added_profiles":[{}]}}"#,
                added("Example (Work)", &work)
            ),
        );
        let env = AgentEnv::new(home.path().to_path_buf());
        let finds = shell(&home, true);

        let loaded = read_profiles(
            &agents_with_example(home.path()),
            &[],
            &env,
            &config_dir,
            home.path(),
            &finds,
        );

        let names: Vec<_> = loaded
            .profiles
            .iter()
            .map(|entry| entry.profile.name())
            .collect();
        assert_eq!(names, ["Example (Home)", "Example (Work)"]);
        assert!(
            matches!(
                &loaded.profiles[1].status,
                ProfileStatus::NotLoggedIn(Some(need))
                    if need.id() == "not-logged-in"
            ),
            "{:?}",
            loaded.profiles[1].status
        );
    }

    /// The login need of `status`: the command its button runs.
    fn login_command(status: &ProfileStatus) -> &otty_agents::AgentCommand {
        let ProfileStatus::NotLoggedIn(Some(need)) = status else {
            panic!("a login need, not {status:?}");
        };
        let otty_agents::PartNeedAction::Run(command) = need.action() else {
            panic!("a command to run, not {:?}", need.action());
        };

        command
    }

    /// Issue 12 of spec 2026-11-23-accounts-apart-from-profiles: an added
    /// folder that is there but empty is never "folder not found". The
    /// example agent needs a login in it, run with the folder's variable.
    /// Only a folder that is not there is not found.
    #[test]
    fn an_empty_added_folder_needs_what_its_agent_says_and_only_a_missing_one_is_not_found()
     {
        let home = TempDir::new("agents-read-empty");
        let config_dir = home.path().join("otty");
        let new = home.path().join(".example-agent-new");
        let gone = home.path().join(".example-agent-gone");
        std::fs::create_dir_all(&new).expect("the empty folder");
        let profiles =
            [added("Example (New)", &new), added("Example (Gone)", &gone)];
        write(
            &config_dir.join("agents.json"),
            &format!(r#"{{"added_profiles":[{}]}}"#, profiles.join(",")),
        );
        let env = AgentEnv::new(home.path().to_path_buf());
        let finds = shell(&home, true);

        let loaded = read_profiles(
            &agents_with_example(home.path()),
            &[],
            &env,
            &config_dir,
            home.path(),
            &finds,
        );

        let status_of = |name: &str| {
            loaded
                .profiles
                .iter()
                .find(|entry| entry.profile.name() == name)
                .map(|entry| entry.status.clone())
                .expect("the profile")
        };
        let empty = status_of("Example (New)");
        let login = login_command(&empty);
        assert_eq!(login.program(), "example-agent");
        assert_eq!(login.args(), ["login"]);
        assert_eq!(
            login.env(),
            [("EXAMPLE_AGENT_HOME".to_string(), new.display().to_string())]
        );
        assert_eq!(
            status_of("Example (Gone)"),
            ProfileStatus::ConfigDirNotFound
        );
    }

    /// Spec 2026-11-19, Decision 7: a hidden profile comes apart from the
    /// others and is not checked, so none of its files is read.
    #[test]
    fn a_hidden_profile_comes_apart_and_is_not_checked() {
        let home = TempDir::new("agents-read-hidden");
        let config_dir = home.path().join("otty");
        let default = home.path().join(".example-agent");
        write(&default.join("auth.json"), LOGIN);
        let work = home.path().join(".example-agent-work");
        write(&work.join("auth.json"), LOGIN);
        write(
            &config_dir.join("agents.json"),
            &format!(
                r#"{{"hidden_profiles":[{{"agent":"{EXAMPLE_AGENT}","config_dir":"{}"}}],"added_profiles":[{}]}}"#,
                default.display(),
                added("Example (Work)", &work)
            ),
        );
        let env = AgentEnv::new(home.path().to_path_buf());
        let finds = shell(&home, true);

        let loaded = read_profiles(
            &agents_with_example(home.path()),
            &[],
            &env,
            &config_dir,
            home.path(),
            &finds,
        );

        let names: Vec<_> = loaded
            .profiles
            .iter()
            .map(|entry| entry.profile.name())
            .collect();
        let hidden: Vec<_> =
            loaded.hidden_profiles.iter().map(Profile::name).collect();
        assert_eq!(names, ["Example (Work)"]);
        assert_eq!(hidden, ["Example Agent"]);
    }

    #[test]
    fn an_old_agents_json_with_hidden_and_a_program_shows_the_profiles_found_by_name()
     {
        let home = TempDir::new("agents-read-old");
        let config_dir = home.path().join("otty");
        let work = home.path().join(".example-agent-work");
        write(&work.join("state.json"), "{}");
        write(
            &config_dir.join("agents.json"),
            &format!(
                r#"{{"default_profiles":{{"{EXAMPLE_AGENT}":{{"hidden":true,"program":"/no/such/example-agent"}}}},"added_profiles":[{{"agent":"{EXAMPLE_AGENT}","name":"Example (Work)","config_dir":"{}","program":"/no/such/example-agent","hidden":true}}]}}"#,
                work.display()
            ),
        );
        let env = AgentEnv::new(home.path().to_path_buf());
        let finds = shell(&home, true);

        let loaded = read_profiles(
            &agents_with_example(home.path()),
            &[],
            &env,
            &config_dir,
            home.path(),
            &finds,
        );

        let statuses = statuses(&loaded);
        assert_eq!(
            statuses[0],
            ("Example Agent", &ProfileStatus::ConfigDirNotFound)
        );
        assert_eq!(statuses[1].0, "Example (Work)");
        assert!(
            matches!(statuses[1].1, ProfileStatus::NotLoggedIn(Some(_))),
            "{statuses:?}"
        );
        assert_eq!(loaded.problem, None);
    }

    /// Spec 2026-11-23-accounts-apart-from-profiles, Decision 5: the
    /// provider part of a plugin finds the accounts in the shown profiles
    /// of its own agent part, wherever the user added them.
    #[test]
    fn the_provider_parts_find_the_accounts_in_the_shown_profiles() {
        let home = TempDir::new("agents-read-accounts");
        let config_dir = home.path().join("otty");
        let work = home.path().join(".example-agent-work");
        write(
            &work.join("auth.json"),
            r#"{"api_key": "ex-made-up-key-0002", "email": "dev@work.example.com"}"#,
        );
        write(
            &config_dir.join("agents.json"),
            &format!(
                r#"{{"added_profiles":[{}]}}"#,
                added("Example (Work)", &work)
            ),
        );
        let env = AgentEnv::new(home.path().to_path_buf());
        let finds = shell(&home, true);

        let loaded = read_profiles(
            &agents_with_example(home.path()),
            &providers_with_example(home.path()),
            &env,
            &config_dir,
            home.path(),
            &finds,
        );

        let place = otty_agents::FoundIn::profile(
            AgentId::new(EXAMPLE_AGENT),
            work.clone(),
        );
        let in_work: Vec<_> = loaded
            .accounts
            .accounts()
            .iter()
            .filter(|account| account.found_in().contains(&place))
            .collect();
        assert_eq!(in_work.len(), 1, "{:?}", loaded.accounts);
        assert_eq!(in_work[0].plugin(), "example");
        assert!(loaded.saved.is_some(), "the file was read");
    }

    /// Spec 2026-11-23-accounts-apart-from-profiles, Decision 6 and issue
    /// 13: the provider part finds the account of OTTY's own login in its
    /// folder under the folder of its plugin, and that folder is never a
    /// profile, so none of its sessions is read.
    #[test]
    fn an_own_login_gives_its_account_and_is_never_a_profile() {
        let home = TempDir::new("agents-read-own-login");
        let config_dir = home.path().join("otty");
        let plugins_folder = home.path().join("data").join("plugins");
        let own = plugins_folder
            .join("example")
            .join("accounts")
            .join("6f0c1e2a");
        write(
            &own.join("auth.json"),
            r#"{"api_key": "ex-made-up-key-0003", "email": "own@example.com"}"#,
        );
        write_session(&own, "a", Path::new("/work"));
        write(
            &config_dir.join("agents.json"),
            &format!(
                r#"{{"own_logins":[{{"id":"6f0c1e2a","plugin":"example","provider":"example-cloud","agent":"{EXAMPLE_AGENT}"}}]}}"#
            ),
        );
        let env = AgentEnv::new(home.path().to_path_buf());
        let finds = shell(&home, true);

        let loaded = read_profiles(
            &agents_with_example(home.path()),
            &providers_with_example(home.path()),
            &env,
            &config_dir,
            &plugins_folder,
            &finds,
        );

        let found: Vec<_> = loaded
            .accounts
            .accounts()
            .iter()
            .filter(|account| account.is_found_in_own_login("6f0c1e2a"))
            .collect();
        assert_eq!(found.len(), 1, "{:?}", loaded.accounts);
        assert_eq!(found[0].plugin(), "example");
        assert_eq!(
            found[0].found_in(),
            [otty_agents::FoundIn::own_login("6f0c1e2a", own.clone())]
        );
        let folders: Vec<&Path> = loaded
            .profiles
            .iter()
            .map(|entry| entry.profile.config_dir())
            .chain(loaded.hidden_profiles.iter().map(Profile::config_dir))
            .collect();
        assert!(!folders.contains(&own.as_path()), "{folders:?}");
    }

    #[test]
    fn a_broken_agents_json_is_told_and_the_defaults_are_used() {
        let home = TempDir::new("agents-read-broken");
        write(&home.path().join("agents.json"), "{ not json");
        std::fs::create_dir_all(home.path().join(".example-agent"))
            .expect("folder");
        let env = AgentEnv::new(home.path().to_path_buf());
        let finds_nothing = shell(&home, false);

        let loaded = read_profiles(
            &agents_with_example(home.path()),
            &[],
            &env,
            home.path(),
            home.path(),
            &finds_nothing,
        );

        assert!(loaded.problem.is_some());
        assert_eq!(loaded.profiles.len(), 1);
    }

    #[test]
    fn the_saved_content_comes_with_the_profiles_unless_it_was_broken() {
        let home = TempDir::new("agents-read-content");
        write(
            &home.path().join("agents.json"),
            &format!(
                r#"{{"default_profiles":{{"{EXAMPLE_AGENT}":{{"name":"Example (Home)"}}}}}}"#
            ),
        );
        let broken = TempDir::new("agents-read-content-broken");
        write(&broken.path().join("agents.json"), "{ not json");
        let env = AgentEnv::new(home.path().to_path_buf());
        let finds = shell(&home, true);

        let loaded = read_profiles(
            &agents_with_example(home.path()),
            &[],
            &env,
            home.path(),
            home.path(),
            &finds,
        );
        let unread = read_profiles(
            &agents_with_example(home.path()),
            &[],
            &env,
            broken.path(),
            home.path(),
            &finds,
        );

        let saved = loaded.saved.expect("the file was read");
        assert!(
            saved
                .default_changes_of(&AgentId::new(EXAMPLE_AGENT))
                .is_some_and(|changes| changes.name() == Some("Example (Home)"))
        );
        assert_eq!(unread.saved, None);
        assert!(unread.problem.is_some());
    }

    #[test]
    fn the_sessions_of_a_profile_come_with_their_missing_folders() {
        let home = TempDir::new("agents-sessions");
        let config_dir = home.path().join(".example-agent");
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
            AgentId::new(EXAMPLE_AGENT),
            "Example Agent",
            config_dir,
        );

        let loaded =
            read_sessions(&agents_with_example(home.path()), profile.clone());

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

        let loaded = read_sessions(&[], profile);

        assert_eq!(loaded.sessions, SessionsRead::NoWay);
    }

    #[test]
    fn the_reads_run_in_the_background_and_answer_with_an_intent() {
        let home = TempDir::new("agents-tasks");
        let config_dir = home.path().join(".example-agent");
        std::fs::create_dir_all(config_dir.join("sessions")).expect("folder");
        let env = AgentEnv::new(home.path().to_path_buf());
        let agents = agents_with_example(home.path());
        let finds_nothing = shell(&home, false);
        let profile = Profile::default_for(
            AgentId::new(EXAMPLE_AGENT),
            "Example Agent",
            config_dir,
        );

        let providers: ProviderList = Arc::from(Vec::new());
        let profiles = run(load_profiles(
            &agents,
            &providers,
            &env,
            home.path(),
            home.path(),
            &finds_nothing,
        ));
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
            AgentId::new("codex"),
            "Codex",
            PathBuf::from("/home/user/.codex"),
        );
        let stranger = Profile::added(
            AgentId::new("stranger"),
            "Stranger",
            PathBuf::from("/home/user/.stranger"),
        );

        let agents = vec![otty_agents::fixtures::codex_stand_in()];
        let sources = session_source_ids(&agents, &profile);
        let none = session_source_ids(&agents, &stranger);

        let loaded = stopped_sessions(profile.clone(), sources);
        let unknown = stopped_sessions(stranger, none);

        assert_eq!(loaded.profile, profile);
        assert_eq!(
            loaded.sessions,
            SessionsRead::Failed(vec![SourceFailure::new(
                SourceId::AgentPart,
                read_stopped()
            )])
        );
        assert_eq!(unknown.sessions, SessionsRead::NoWay);
    }
}
