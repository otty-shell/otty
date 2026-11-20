//! The list of profiles the mode shows: the default profile of each known
//! agent with the user's changes, then the profiles the user added, then
//! the extra folders of the agents OTTY found in the home folder, and the
//! state of each.

use std::path::{Path, PathBuf};

use crate::agent::{Agent, agent_of};
use crate::env::AgentEnv;
use crate::profile::{Profile, ProfileStatus};
use crate::saved_profiles::{DefaultProfileChanges, SavedProfiles};

/// The profiles to show: for each agent of `agents`
/// its default profile with the user's changes from `saved`, then the
/// profiles the user added, in the order they were added, then the
/// profiles OTTY found (spec Decision 2).
///
/// The default profile's folder is the agent's default config folder (from
/// its variable in `env`, else in the home folder), unless the user moved
/// it. A default profile is left out when its folder does not exist and its
/// program is not found either: then the agent is not installed.
/// `is_program_found` says whether a program, given by its name, can be
/// started; the app looks it up the way it starts it. An added
/// profile of an agent not in `agents` is left out.
///
/// Each profile knows whether its folder is the agent's default folder, so
/// its commands set the agent's config folder variable only when it is
/// not.
///
/// A found profile is a folder in the home folder named after the agent's
/// config folder with a `-` or `_` and more, such as `~/.claude-work`, that
/// holds the agent's login or a session and is no other profile's folder.
/// It is named after the rest of the folder name, "Work"; when another
/// profile has that name, after its agent too, "Claude Code · Work". They
/// come by agent, then by folder name. Only the folders are read; nothing
/// is written.
///
/// ```
/// use std::path::PathBuf;
///
/// use otty_agents::fixtures::codex_stand_in;
/// use otty_agents::{AgentEnv, SavedProfiles, profiles};
///
/// let env = AgentEnv::new(PathBuf::from("/no/such/home"));
/// let saved = SavedProfiles::default();
///
/// let list = profiles(&[codex_stand_in()], &env, &saved, |program| {
///     program == "codex"
/// });
///
/// assert_eq!(list.len(), 1);
/// assert_eq!(list[0].name(), "Codex");
/// ```
pub fn profiles(
    agents: &[Box<dyn Agent>],
    env: &AgentEnv,
    saved: &SavedProfiles,
    is_program_found: impl Fn(&str) -> bool,
) -> Vec<Profile> {
    let mut list = Vec::new();

    for agent in agents {
        let changes = saved.default_changes_of(agent.entry().id());
        let profile = default_profile(agent.as_ref(), env, changes);

        let is_installed = profile.config_dir().is_dir()
            || is_program_found(agent.entry().program());
        if is_installed {
            list.push(profile);
        }
    }

    for profile in saved.added() {
        if let Some(agent) = agent_of(agents, profile.agent()) {
            let default_dir = agent.default_config_dir(env);
            list.push(profile.clone().with_agent_default_dir(&default_dir));
        }
    }

    for agent in agents {
        let found = found_profiles(agent.as_ref(), env, &list);
        list.extend(found);
    }

    list
}

/// The state of `profile` of `agent`. The agent's program is started by
/// its name, so it must be found: `is_program_found` says whether a
/// program, given by its name, can be started, the same way as for
/// [`profiles`]. The agent checks the rest.
///
/// ```
/// use std::path::PathBuf;
///
/// use otty_agents::fixtures::codex_stand_in;
/// use otty_agents::{AgentId, Profile, ProfileStatus, profile_status};
///
/// let profile = Profile::default_for(
///     AgentId::new("codex"),
///     "Codex",
///     PathBuf::from("/no/such/home/.codex"),
/// );
///
/// let status = profile_status(codex_stand_in().as_ref(), &profile, |_| false);
///
/// assert_eq!(status, ProfileStatus::ProgramNotFound);
/// ```
pub fn profile_status(
    agent: &dyn Agent,
    profile: &Profile,
    is_program_found: impl Fn(&str) -> bool,
) -> ProfileStatus {
    if !is_program_found(agent.entry().program()) {
        return ProfileStatus::ProgramNotFound;
    }

    agent.detect(profile)
}

/// The default profile of `agent` with the user's `changes`, if any.
fn default_profile(
    agent: &dyn Agent,
    env: &AgentEnv,
    changes: Option<&DefaultProfileChanges>,
) -> Profile {
    let entry = agent.entry();
    let default_dir = agent.default_config_dir(env);

    let Some(changes) = changes else {
        return Profile::default_for(
            entry.id().clone(),
            entry.name(),
            default_dir,
        );
    };

    let name = changes.name().unwrap_or(entry.name());
    let config_dir = changes.config_dir().unwrap_or(&default_dir);
    Profile::default_for(entry.id().clone(), name, config_dir.to_path_buf())
        .with_agent_default_dir(&default_dir)
}

/// The profiles of `agent` found on this computer
/// ([`Agent::found_config_dirs`]) that no profile of `listed` uses, named
/// so that no profile of `listed` or found before has its name.
fn found_profiles(
    agent: &dyn Agent,
    env: &AgentEnv,
    listed: &[Profile],
) -> Vec<Profile> {
    let entry = agent.entry();
    let default_dir = agent.default_config_dir(env);
    let base = agent.config_folder();
    let mut found: Vec<Profile> = Vec::new();

    for folder in agent.found_config_dirs(env) {
        let is_listed = listed.iter().any(|profile| {
            profile.agent() == entry.id() && profile.config_dir() == folder
        });
        if is_listed {
            continue;
        }

        let name = match folder_suffix(&folder, base.name()) {
            Some(suffix) => found_name(suffix),
            None => folder_name(&folder),
        };
        let Some(name) = name else {
            continue;
        };

        let is_taken = |name: &str| {
            listed
                .iter()
                .chain(&found)
                .any(|other| other.name() == name)
        };
        let name = if is_taken(&name) {
            format!("{} · {name}", entry.name())
        } else {
            name
        };
        if is_taken(&name) {
            continue;
        }

        let profile = Profile::found(entry.id().clone(), name, folder)
            .with_agent_default_dir(&default_dir);
        found.push(profile);
    }

    found
}

/// The folders in the home folder of `env` named after the config folder
/// of `agent` with a `-` or `_` after it, such as `~/.claude-work`, that
/// hold its login or a session, by name: what
/// [`Agent::found_config_dirs`] gives for a reader built into OTTY.
pub(crate) fn found_in_home<A: Agent + ?Sized>(
    agent: &A,
    env: &AgentEnv,
) -> Vec<PathBuf> {
    let base = agent.config_folder();
    let id = agent.entry().id();

    folders_in(env.home())
        .into_iter()
        .filter(|folder| folder_suffix(folder, base.name()).is_some())
        .filter(|folder| {
            let candidate = Profile::found(id.clone(), "", folder.clone());
            let is_logged_in = matches!(
                agent.detect(&candidate),
                ProfileStatus::Connected { .. }
            );

            is_logged_in || agent.has_sessions(&candidate)
        })
        .collect()
}

/// The folders in `home`, by name. A home folder that can not be read has
/// none.
fn folders_in(home: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(home) else {
        return Vec::new();
    };

    let mut folders: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    folders.sort();
    folders
}

/// What follows the agent's config folder name `base` in the name of
/// `folder` when it starts with a `-` or `_`, such as `-work` of
/// `.claude-work`; `None` for a folder named otherwise.
fn folder_suffix<'a>(folder: &'a Path, base: &str) -> Option<&'a str> {
    let name = folder.file_name()?.to_str()?;
    let rest = name.strip_prefix(base)?;

    if !rest.starts_with(['-', '_']) {
        return None;
    }

    Some(rest)
}

/// The name of a found profile whose folder is not named after the
/// agent's config folder: the folder's own name, such as "agent-data".
fn folder_name(folder: &Path) -> Option<String> {
    let name = folder.file_name()?.to_str()?;

    (!name.is_empty()).then(|| name.to_string())
}

/// The name of a found profile from the end of its folder name: each word
/// between `-`, `_` and `.` with a capital first letter, such as "My Team"
/// for `-my-team`. `None` when there is no word.
fn found_name(suffix: &str) -> Option<String> {
    let words: Vec<String> = suffix
        .split(['-', '_', '.'])
        .filter(|word| !word.is_empty())
        .map(capitalized)
        .collect();

    (!words.is_empty()).then(|| words.join(" "))
}

/// `word` with its first letter in upper case.
fn capitalized(word: &str) -> String {
    let mut characters = word.chars();

    match characters.next() {
        Some(first) => first.to_uppercase().chain(characters).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;
    use crate::agent::AgentId;
    use crate::command::AgentCommand;
    use crate::testing::{
        MockAgent, TempDir, mock_entry, test_agents, write_file,
    };

    fn codex_id() -> AgentId {
        AgentId::new("codex")
    }

    fn env_in(home: &Path) -> AgentEnv {
        AgentEnv::new(home.to_path_buf())
    }

    fn make_dir(path: &Path) -> PathBuf {
        std::fs::create_dir_all(path).expect("create test folder");
        path.to_path_buf()
    }

    fn no_program(_program: &str) -> bool {
        false
    }

    #[test]
    fn a_default_profile_is_in_the_default_folder_in_the_home_folder() {
        let temp = TempDir::new();
        let codex_dir = make_dir(&temp.path().join(".codex"));
        let env = env_in(temp.path());

        let list = profiles(
            &test_agents(),
            &env,
            &SavedProfiles::default(),
            no_program,
        );

        assert_eq!(
            list,
            [Profile::default_for(codex_id(), "Codex", codex_dir)]
        );
        assert_eq!(
            test_agents()[0].launch(&list[0], None),
            AgentCommand::new("codex")
        );
    }

    #[test]
    fn a_default_profile_is_in_the_folder_of_the_agents_variable() {
        let temp = TempDir::new();
        make_dir(&temp.path().join(".codex"));
        let codex_home = make_dir(&temp.path().join("data/codex"));
        let env = AgentEnv::new(temp.path().to_path_buf())
            .with_config_dir("CODEX_HOME", codex_home.clone());

        let list = profiles(
            &test_agents(),
            &env,
            &SavedProfiles::default(),
            no_program,
        );

        assert_eq!(
            list,
            [Profile::default_for(codex_id(), "Codex", codex_home)]
        );
        assert_eq!(
            test_agents()[0].launch(&list[0], None),
            AgentCommand::new("codex")
        );
    }

    #[test]
    fn a_default_profile_with_no_folder_is_shown_when_its_program_is_found() {
        let temp = TempDir::new();
        let env = env_in(temp.path());

        let list = profiles(
            &test_agents(),
            &env,
            &SavedProfiles::default(),
            |program| program == "codex",
        );

        assert_eq!(
            list,
            [Profile::default_for(
                codex_id(),
                "Codex",
                temp.path().join(".codex")
            )]
        );
    }

    #[test]
    fn the_users_changes_apply_to_the_default_profile() {
        let temp = TempDir::new();
        make_dir(&temp.path().join(".codex"));
        let moved_dir = make_dir(&temp.path().join("data/codex"));
        let env = env_in(temp.path());
        let saved = SavedProfiles::default().with_default_changes(
            DefaultProfileChanges::new(codex_id())
                .with_name("Codex · Home")
                .with_config_dir(moved_dir.clone()),
        );

        let list = profiles(&test_agents(), &env, &saved, no_program);

        let expected =
            Profile::default_for(codex_id(), "Codex · Home", moved_dir.clone())
                .with_agent_default_dir(&temp.path().join(".codex"));
        assert_eq!(list, [expected]);
        assert_eq!(
            test_agents()[0].launch(&list[0], None),
            AgentCommand::new("codex")
                .with_env("CODEX_HOME", moved_dir.to_string_lossy())
        );
    }

    #[test]
    fn a_changed_default_profile_of_an_agent_not_installed_is_left_out() {
        let temp = TempDir::new();
        let env = env_in(temp.path());
        let saved = SavedProfiles::default().with_default_changes(
            DefaultProfileChanges::new(codex_id()).with_name("Codex · Home"),
        );

        let list = profiles(&test_agents(), &env, &saved, no_program);

        assert!(list.is_empty());
    }

    #[test]
    fn added_profiles_come_after_the_default_ones_in_their_order() {
        let temp = TempDir::new();
        let codex_dir = make_dir(&temp.path().join(".codex"));
        let env = env_in(temp.path());
        let codex_work = Profile::added(
            codex_id(),
            "Codex · Work",
            temp.path().join(".codex-work"),
        );
        let codex_home = Profile::added(
            codex_id(),
            "Codex · Home",
            temp.path().join(".codex-home"),
        );
        let future = Profile::added(
            AgentId::new("future-agent"),
            "Future",
            temp.path().join(".future"),
        );
        let saved = SavedProfiles::default()
            .with_added(codex_work.clone())
            .with_added(future)
            .with_added(codex_home.clone());

        let list = profiles(&test_agents(), &env, &saved, no_program);

        assert_eq!(
            list,
            [
                Profile::default_for(codex_id(), "Codex", codex_dir),
                codex_work,
                codex_home,
            ]
        );
    }

    #[test]
    fn an_added_profile_in_the_agents_default_folder_sets_no_variable() {
        let temp = TempDir::new();
        let moved_dir = make_dir(&temp.path().join("data/codex"));
        let env = env_in(temp.path());
        let saved = SavedProfiles::default()
            .with_default_changes(
                DefaultProfileChanges::new(codex_id())
                    .with_config_dir(moved_dir),
            )
            .with_added(Profile::added(
                codex_id(),
                "Codex · Old",
                temp.path().join(".codex"),
            ));

        let list = profiles(&test_agents(), &env, &saved, no_program);

        let commands: Vec<_> = list
            .iter()
            .map(|profile| test_agents()[0].launch(profile, None))
            .collect();
        assert_eq!(
            commands,
            [
                AgentCommand::new("codex").with_env(
                    "CODEX_HOME",
                    temp.path().join("data/codex").to_string_lossy()
                ),
                AgentCommand::new("codex"),
            ]
        );
    }

    /// A Codex session file in `config_dir`, made up.
    fn codex_session(config_dir: &Path) {
        write_file(
            &config_dir.join(
                "sessions/2026/11/19/rollout-2026-11-19T10-00-00-made-up.jsonl",
            ),
            r#"{"type":"session_meta","payload":{"id":"made-up","cwd":"/work"}}"#,
        );
    }

    fn found_names(list: &[Profile]) -> Vec<(&str, &Path)> {
        list.iter()
            .filter(|profile| profile.is_found())
            .map(|profile| (profile.name(), profile.config_dir()))
            .collect()
    }

    #[test]
    fn an_extra_folder_is_a_profile_named_after_it() {
        let temp = TempDir::new();
        make_dir(&temp.path().join(".codex"));
        let work = temp.path().join(".codex-work");
        codex_session(&work);
        let env = env_in(temp.path());

        let list = profiles(
            &test_agents(),
            &env,
            &SavedProfiles::default(),
            no_program,
        );

        assert_eq!(
            list,
            [
                Profile::default_for(
                    codex_id(),
                    "Codex",
                    temp.path().join(".codex")
                ),
                Profile::found(codex_id(), "Work", work.clone()),
            ]
        );
        assert_eq!(
            test_agents()[0].launch(&list[1], None),
            AgentCommand::new("codex")
                .with_env("CODEX_HOME", work.to_string_lossy())
        );
    }

    #[test]
    fn an_extra_folder_with_sessions_is_a_profile() {
        let temp = TempDir::new();
        let semrush = temp.path().join(".codex-semrush");
        codex_session(&semrush);
        let env = env_in(temp.path());

        let list = profiles(
            &test_agents(),
            &env,
            &SavedProfiles::default(),
            no_program,
        );

        assert_eq!(found_names(&list), [("Semrush", semrush.as_path())]);
    }

    #[test]
    fn a_folder_without_a_login_or_sessions_is_skipped() {
        let temp = TempDir::new();
        make_dir(&temp.path().join(".codex-old/sessions"));
        write_file(&temp.path().join(".codex-empty/config.toml"), "");
        codex_session(&temp.path().join(".codexette"));
        codex_session(&temp.path().join("codex-work"));
        let env = env_in(temp.path());

        let list = profiles(
            &test_agents(),
            &env,
            &SavedProfiles::default(),
            no_program,
        );

        assert!(list.is_empty(), "{list:?}");
    }

    #[test]
    fn a_found_folder_that_is_a_saved_profile_is_not_listed_twice() {
        let temp = TempDir::new();
        let work = temp.path().join(".codex-work");
        codex_session(&work);
        let env = env_in(temp.path());
        let added = Profile::added(codex_id(), "Codex · Office", work.clone());
        let saved = SavedProfiles::default().with_added(added.clone());

        let list = profiles(&test_agents(), &env, &saved, no_program);

        assert_eq!(list, [added]);
    }

    #[test]
    fn a_found_folder_that_is_the_moved_default_profile_is_not_listed() {
        let temp = TempDir::new();
        let work = temp.path().join(".codex-work");
        codex_session(&work);
        let env = env_in(temp.path());
        let saved = SavedProfiles::default().with_default_changes(
            DefaultProfileChanges::new(codex_id())
                .with_config_dir(work.clone()),
        );

        let list = profiles(&test_agents(), &env, &saved, no_program);

        assert_eq!(list.len(), 1);
        assert!(list[0].is_default());
        assert_eq!(list[0].config_dir(), work);
    }

    #[test]
    fn a_found_name_taken_by_another_profile_gets_its_agents_name() {
        let temp = TempDir::new();
        let codex_work = temp.path().join(".codex-work");
        codex_session(&codex_work);
        let codex_team = temp.path().join(".codex_my-team");
        codex_session(&codex_team);
        let env = env_in(temp.path());
        let saved = SavedProfiles::default().with_added(Profile::added(
            codex_id(),
            "Work",
            temp.path().join("data/codex"),
        ));

        let list = profiles(&test_agents(), &env, &saved, no_program);

        assert_eq!(
            found_names(&list),
            [
                ("Codex · Work", codex_work.as_path()),
                ("My Team", codex_team.as_path()),
            ]
        );
    }

    #[test]
    fn the_name_of_a_found_folder_is_its_words_capitalized() {
        assert_eq!(found_name("work"), Some(String::from("Work")));
        assert_eq!(found_name("-my-team_2"), Some(String::from("My Team 2")));
        assert_eq!(found_name("Semrush"), Some(String::from("Semrush")));
        assert_eq!(found_name("--"), None);
    }

    /// A made-up agent whose program is `mock-agent`.
    fn mock() -> MockAgent {
        let mut agent = MockAgent::new();
        agent
            .expect_entry()
            .return_const(mock_entry("mock", "mock-agent"));
        agent
    }

    fn mock_profile() -> Profile {
        Profile::added(AgentId::new("mock"), "Mock", PathBuf::from("/m"))
    }

    #[test]
    fn a_profile_whose_program_is_not_found_says_so_before_any_check() {
        let mut agent = mock();
        agent.expect_detect().never();

        let status = profile_status(&agent, &mock_profile(), no_program);

        assert_eq!(status, ProfileStatus::ProgramNotFound);
    }

    #[test]
    fn a_profile_whose_program_is_found_has_the_state_its_agent_checks() {
        let mut agent = mock();
        agent
            .expect_detect()
            .return_const(ProfileStatus::NotLoggedIn);

        let status = profile_status(&agent, &mock_profile(), |program| {
            program == "mock-agent"
        });

        assert_eq!(status, ProfileStatus::NotLoggedIn);
    }
}
