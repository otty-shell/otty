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
        agents_with_opencode, agents_with_plugins, executable_script,
        providers_with_plugins, run, write,
    };

    const SESSION: &str = "3f1c2b7e-8d4a-4e6f-9b0c-1a2d3e4f5a6b";

    /// A made-up shell that answers "found" for every program, or for
    /// none, without reading the user's start files.
    fn shell(folder: &TempDir, finds: bool) -> String {
        let code = if finds { 0 } else { 1 };

        executable_script(folder.path(), "fake-shell", &format!("exit {code}"))
            .display()
            .to_string()
    }

    /// A made-up Coder session in `folder`, kept in `config_dir`.
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

    /// Issue 14: the built OpenCode plugin reads OpenCode's data folder:
    /// the profile is connected and its version comes from its database,
    /// which is never written. Spec 2026-11-23-accounts-apart-from-profiles,
    /// Decision 11: OpenCode finds no account.
    #[test]
    fn the_built_opencode_plugin_reads_its_data_folder() {
        let home = TempDir::new("agents-read-opencode");
        let data = home.path().join(".local/share/opencode");
        let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join(
            "../../../../../plugins/opencode/fixtures/.local/share/opencode",
        );
        std::fs::create_dir_all(&data).expect("the data folder");
        for file in ["auth.json", "opencode.db"] {
            std::fs::copy(fixtures.join(file), data.join(file))
                .expect("the made-up file");
        }
        let database = std::fs::read(data.join("opencode.db")).expect("read");
        let env = AgentEnv::new(home.path().to_path_buf());

        let loaded = read_profiles(
            &agents_with_opencode(home.path()),
            &[],
            &env,
            home.path(),
            home.path(),
            &shell(&home, true),
        );

        let opencode = loaded
            .profiles
            .iter()
            .find(|entry| entry.profile.agent().as_str() == "opencode")
            .expect("the OpenCode profile");
        assert_eq!(opencode.status, ProfileStatus::Connected);
        assert_eq!(opencode.version.as_deref(), Some("2.3.2"));
        assert!(
            loaded.accounts.accounts().is_empty(),
            "OpenCode has no provider part: its logins are no accounts"
        );
        assert_eq!(
            std::fs::read(data.join("opencode.db")).expect("read"),
            database,
            "the database is never written"
        );
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

        let found = read_profiles(
            &agents_with_plugins(home.path()),
            &[],
            &env,
            home.path(),
            home.path(),
            &finds,
        );
        let not_found = read_profiles(
            &agents_with_plugins(home.path()),
            &[],
            &env,
            home.path(),
            home.path(),
            &finds_nothing,
        );

        let connected = ProfileStatus::Connected;
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

    /// Spec 2026-11-23, Decision 4: with `~/.codex` and `~/.codex-work`
    /// on disk and nothing added, only `~/.codex` is a profile; an extra
    /// folder is one once the user adds it, with the version its sessions
    /// name. The read of the profiles never starts an agent's program, so a
    /// profile with no session has no version yet.
    #[test]
    fn only_the_default_and_the_added_folders_are_profiles() {
        let home = TempDir::new("agents-read-versions");
        let config_dir = home.path().join("otty");
        write(
            &home.path().join(".codex/config.toml"),
            "model = \"gpt-5-codex\"",
        );
        write(
            &home.path().join(".codex-work/config.toml"),
            "model = \"gpt-5-codex\"",
        );
        let semrush = home.path().join(".claude-semrush");
        write(
            &semrush.join("projects/-work/a1.jsonl"),
            r#"{"type":"user","version":"2.1.328","cwd":"/work","message":{"content":"Fix it"}}"#,
        );
        let env = AgentEnv::new(home.path().to_path_buf());
        let prints_a_version = executable_script(
            home.path(),
            "version-shell",
            "echo '2.1.0 (Claude Code)'",
        )
        .display()
        .to_string();
        let agents = agents_with_plugins(home.path());

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
            &format!(
                r#"{{"added_profiles":[{{"agent":"claude-code","name":"Semrush","config_dir":"{}"}}]}}"#,
                semrush.display()
            ),
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
        assert_eq!(
            folders(&before),
            [named(".claude", None), named(".codex", None)]
        );
        assert_eq!(
            folders(&after),
            [
                named(".claude", None),
                named(".codex", None),
                named(".claude-semrush", Some("2.1.328")),
            ]
        );
    }

    #[test]
    fn saved_profiles_are_read_from_agents_json_in_the_config_folder() {
        let home = TempDir::new("agents-read-saved");
        let config_dir = home.path().join("otty");
        let work = home.path().join(".codex-work");
        write(&work.join("config.toml"), "model = \"gpt-5-codex\"");
        write(
            &config_dir.join("agents.json"),
            &format!(
                r#"{{"default_profiles":{{"codex":{{"name":"Codex · Home"}}}},"added_profiles":[{{"agent":"codex","name":"Codex · Work","config_dir":"{}"}}]}}"#,
                work.display()
            ),
        );
        let env = AgentEnv::new(home.path().to_path_buf());
        let finds = shell(&home, true);

        let loaded = read_profiles(
            &agents_with_plugins(home.path()),
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
        assert_eq!(names, ["Claude Code", "Codex · Home", "Codex · Work"]);
        assert!(
            matches!(
                &loaded.profiles[2].status,
                ProfileStatus::NotLoggedIn(Some(need))
                    if need.id() == "not-logged-in"
            ),
            "{:?}",
            loaded.profiles[2].status
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
    /// folder that is there but empty is never "folder not found". Codex
    /// and Claude Code need a login in it, run with the folder's variable,
    /// and OpenCode, which works with no login, is found. Only a folder
    /// that is not there is not found.
    #[test]
    fn an_empty_added_folder_needs_what_its_agent_says_and_only_a_missing_one_is_not_found()
     {
        let home = TempDir::new("agents-read-empty");
        let config_dir = home.path().join("otty");
        let codex_new = home.path().join(".codex-new");
        let claude_new = home.path().join(".claude-new");
        let opencode_new = home.path().join("work/opencode");
        let codex_gone = home.path().join(".codex-gone");
        for folder in [&codex_new, &claude_new, &opencode_new] {
            std::fs::create_dir_all(folder).expect("the empty folder");
        }
        let added = |agent: &str, name: &str, folder: &Path| {
            format!(
                r#"{{"agent":"{agent}","name":"{name}","config_dir":"{}"}}"#,
                folder.display()
            )
        };
        let profiles = [
            added("codex", "Codex · New", &codex_new),
            added("claude-code", "Claude Code · New", &claude_new),
            added("opencode", "OpenCode · New", &opencode_new),
            added("codex", "Codex · Gone", &codex_gone),
        ];
        write(
            &config_dir.join("agents.json"),
            &format!(r#"{{"added_profiles":[{}]}}"#, profiles.join(",")),
        );
        let env = AgentEnv::new(home.path().to_path_buf());
        let finds = shell(&home, true);

        let loaded = read_profiles(
            &agents_with_plugins(home.path()),
            &[],
            &env,
            &config_dir,
            home.path(),
            &finds,
        );
        let opencode = read_profiles(
            &agents_with_opencode(home.path()),
            &[],
            &env,
            &config_dir,
            home.path(),
            &finds,
        );

        let status_of = |loaded: &LoadedProfiles, name: &str| {
            loaded
                .profiles
                .iter()
                .find(|entry| entry.profile.name() == name)
                .map(|entry| entry.status.clone())
                .expect("the profile")
        };
        let codex = status_of(&loaded, "Codex · New");
        let codex_login = login_command(&codex);
        assert_eq!(codex_login.program(), "codex");
        assert_eq!(codex_login.args(), ["login"]);
        assert_eq!(
            codex_login.env(),
            [("CODEX_HOME".to_string(), codex_new.display().to_string())]
        );
        let claude = status_of(&loaded, "Claude Code · New");
        let claude_login = login_command(&claude);
        assert_eq!(claude_login.program(), "claude");
        assert_eq!(claude_login.args(), ["auth", "login", "--claudeai"]);
        assert_eq!(
            claude_login.env(),
            [(
                "CLAUDE_CONFIG_DIR".to_string(),
                claude_new.display().to_string()
            )]
        );
        assert_eq!(
            status_of(&opencode, "OpenCode · New"),
            ProfileStatus::Connected
        );
        assert_eq!(
            status_of(&loaded, "Codex · Gone"),
            ProfileStatus::ConfigDirNotFound
        );
    }

    /// Spec 2026-11-19, Decision 7: a hidden profile comes apart from the
    /// others and is not checked, so none of its files is read.
    #[test]
    fn a_hidden_profile_comes_apart_and_is_not_checked() {
        let home = TempDir::new("agents-read-hidden");
        let config_dir = home.path().join("otty");
        let claude_dir = home.path().join(".claude");
        write(
            &claude_dir.join(".credentials.json"),
            r#"{"claudeAiOauth":{"accessToken":"made-up"}}"#,
        );
        write(
            &config_dir.join("agents.json"),
            &format!(
                r#"{{"hidden_profiles":[{{"agent":"claude-code","config_dir":"{}"}}]}}"#,
                claude_dir.display()
            ),
        );
        let env = AgentEnv::new(home.path().to_path_buf());
        let finds = shell(&home, true);

        let loaded = read_profiles(
            &agents_with_plugins(home.path()),
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
        assert_eq!(names, ["Codex"]);
        assert_eq!(hidden, ["Claude Code"]);
    }

    #[test]
    fn an_old_agents_json_with_hidden_and_a_program_shows_the_profiles_found_by_name()
     {
        let home = TempDir::new("agents-read-old");
        let config_dir = home.path().join("otty");
        let work = home.path().join(".codex-work");
        write(&work.join("config.toml"), "model = \"gpt-5-codex\"");
        write(
            &config_dir.join("agents.json"),
            &format!(
                r#"{{"default_profiles":{{"codex":{{"hidden":true,"program":"/no/such/codex"}}}},"added_profiles":[{{"agent":"codex","name":"Codex · Work","config_dir":"{}","program":"/no/such/codex","hidden":true}}]}}"#,
                work.display()
            ),
        );
        let env = AgentEnv::new(home.path().to_path_buf());
        let finds = shell(&home, true);

        let loaded = read_profiles(
            &agents_with_plugins(home.path()),
            &[],
            &env,
            &config_dir,
            home.path(),
            &finds,
        );

        let statuses = statuses(&loaded);
        assert_eq!(statuses[1], ("Codex", &ProfileStatus::ConfigDirNotFound));
        assert_eq!(statuses[2].0, "Codex · Work");
        assert!(
            matches!(statuses[2].1, ProfileStatus::NotLoggedIn(Some(_))),
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
        let work = home.path().join(".claude-work");
        write(
            &work.join(".claude.json"),
            r#"{"oauthAccount":{"accountUuid":"00000000-0000-4000-8000-00000000000a","emailAddress":"dev@example.com","organizationUuid":"00000000-0000-4000-8000-00000000000b","organizationName":"Made-up Org"}}"#,
        );
        write(
            &work.join(".credentials.json"),
            r#"{"claudeAiOauth":{"accessToken":"made-up","refreshToken":"made-up","expiresAt":4102444800000,"subscriptionType":"pro"}}"#,
        );
        write(
            &config_dir.join("agents.json"),
            &format!(
                r#"{{"added_profiles":[{{"agent":"claude-code","name":"Claude Code · Work","config_dir":"{}"}}]}}"#,
                work.display()
            ),
        );
        let env = AgentEnv::new(home.path().to_path_buf());
        let finds = shell(&home, true);

        let loaded = read_profiles(
            &agents_with_plugins(home.path()),
            &providers_with_plugins(home.path()),
            &env,
            &config_dir,
            home.path(),
            &finds,
        );

        let place = otty_agents::FoundIn::profile(
            AgentId::new("claude-code"),
            work.clone(),
        );
        let in_work: Vec<_> = loaded
            .accounts
            .accounts()
            .iter()
            .filter(|account| account.found_in().contains(&place))
            .collect();
        assert_eq!(in_work.len(), 1, "{:?}", loaded.accounts);
        assert_eq!(in_work[0].plugin(), "anthropic");
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
            .join("anthropic")
            .join("accounts")
            .join("6f0c1e2a");
        write(
            &own.join(".claude.json"),
            r#"{"oauthAccount":{"accountUuid":"00000000-0000-4000-8000-00000000000a","emailAddress":"own@example.com"}}"#,
        );
        write(
            &own.join(".credentials.json"),
            r#"{"claudeAiOauth":{"accessToken":"made-up","refreshToken":"made-up","expiresAt":4102444800000,"subscriptionType":"pro"}}"#,
        );
        write(
            &own.join("projects").join("-work").join("a.jsonl"),
            r#"{"type":"user","cwd":"/work","message":{"content":"Probe"}}"#,
        );
        write(
            &config_dir.join("agents.json"),
            r#"{"own_logins":[{"id":"6f0c1e2a","plugin":"anthropic","provider":"anthropic","agent":"claude-code"}]}"#,
        );
        let env = AgentEnv::new(home.path().to_path_buf());
        let finds = shell(&home, true);

        let loaded = read_profiles(
            &agents_with_plugins(home.path()),
            &providers_with_plugins(home.path()),
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
        assert_eq!(found[0].plugin(), "anthropic");
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
        std::fs::create_dir_all(home.path().join(".claude")).expect("folder");
        let env = AgentEnv::new(home.path().to_path_buf());
        let finds_nothing = shell(&home, false);

        let loaded = read_profiles(
            &agents_with_plugins(home.path()),
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
            r#"{"default_profiles":{"codex":{"name":"Codex · Home"}}}"#,
        );
        let broken = TempDir::new("agents-read-content-broken");
        write(&broken.path().join("agents.json"), "{ not json");
        let env = AgentEnv::new(home.path().to_path_buf());
        let finds = shell(&home, true);

        let loaded = read_profiles(
            &agents_with_plugins(home.path()),
            &[],
            &env,
            home.path(),
            home.path(),
            &finds,
        );
        let unread = read_profiles(
            &agents_with_plugins(home.path()),
            &[],
            &env,
            broken.path(),
            home.path(),
            &finds,
        );

        let saved = loaded.saved.expect("the file was read");
        assert!(
            saved
                .default_changes_of(&AgentId::new("codex"))
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

        let loaded =
            read_sessions(&agents_with_plugins(home.path()), profile.clone());

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
        let config_dir = home.path().join(".claude");
        std::fs::create_dir_all(config_dir.join("projects")).expect("folder");
        let env = AgentEnv::new(home.path().to_path_buf());
        let agents = agents_with_plugins(home.path());
        let finds_nothing = shell(&home, false);
        let profile = Profile::default_for(
            AgentId::new("claude-code"),
            "Claude Code",
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

    /// Issue 25: how long OTTY takes to read the sessions of the real
    /// `~/.codex` through the OpenAI plugin, and of the real OpenCode
    /// database through the OpenCode plugin, the first time and again
    /// with nothing changed. It only reads the folders. Run it by hand:
    /// `cargo test -p otty-app-agents-io --release measure_the_sessions --
    /// --ignored --nocapture`.
    #[test]
    #[ignore = "reads the real ~/.codex and OpenCode data; run by hand"]
    fn measure_the_sessions_read_of_the_real_folders() {
        let home = std::env::var_os("HOME").map(PathBuf::from).expect("HOME");
        let codex = Profile::default_for(
            AgentId::new("codex"),
            "Codex",
            home.join(".codex"),
        );
        let opencode = Profile::default_for(
            AgentId::new("opencode"),
            "OpenCode",
            home.join(".local/share/opencode"),
        );

        measure(&agents_with_plugins(&home), &codex);
        measure(&agents_with_opencode(&home), &opencode);
    }

    /// Prints how long three reads of the sessions of `profile` take.
    fn measure(agents: &AgentList, profile: &Profile) {
        for read in ["first", "second", "third"] {
            let started = std::time::Instant::now();
            let loaded = read_sessions(agents, profile.clone());
            let took = started.elapsed();

            let count = match &loaded.sessions {
                SessionsRead::Read(sessions) => sessions.len(),
                other => panic!("the sessions are read: {other:?}"),
            };
            println!(
                "{}: {read} read: {count} sessions in {took:?}",
                profile.name()
            );
        }
    }
}
