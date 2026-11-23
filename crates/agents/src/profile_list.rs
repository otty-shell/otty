//! The list of profiles the mode shows: the default profile of each known
//! agent with the user's changes, then the profiles the user added, and
//! the state of each. OTTY looks for no other folder of an agent (spec
//! 2026-11-23, Decision 4).

use std::path::Path;

use crate::agent::{Agent, agent_of};
use crate::env::AgentEnv;
use crate::profile::{Profile, ProfileStatus};
use crate::saved_profiles::{DefaultProfileChanges, SavedProfiles};

/// The profiles to show: for each agent of `agents` its default profile
/// with the user's changes from `saved`, then the profiles the user added,
/// in the order they were added (spec 2026-11-23, Decision 4). No other
/// folder of an agent is a profile until the user adds it. The profiles
/// the user hid are in the list too: the app sets them apart with
/// [`SavedProfiles::is_profile_hidden`], so that it can list them on
/// request.
///
/// The default profile's folder is the agent's default config folder (from
/// its variable in `env`, else in the home folder), unless the user moved
/// it. A default profile is left out when its folder does not exist and its
/// program is not found either: then the agent is not installed.
/// `is_program_found` says whether a program, given by its name, can be
/// started; the app looks it up the way it starts it. A program it does
/// not find is looked for at the agent's program places too (see
/// [`profile_status`]). An added profile of an agent not in `agents` is
/// left out.
///
/// Each profile knows whether its folder is the agent's default folder, so
/// its commands set the agent's config folder variable only when it is
/// not.
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
            || find_program(agent.as_ref(), &is_program_found);
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

    list
}

/// The state of `profile` of `agent`. The agent's program must be found:
/// `is_program_found` says whether a program, given by its name, can be
/// started, the same way as for [`profiles`]; when it says no, the first of
/// the agent's program places that holds a program is where it is, and the
/// agent starts it from there (issue 32). Then OTTY checks that the config
/// folder is there: only a folder that is not there is "folder not found",
/// and the agent is not asked about it. An empty folder is there: the agent
/// checks the rest and says what it needs, such as a login, or that it
/// needs nothing (issue 12 of spec 2026-11-23-accounts-apart-from-profiles).
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
    if !find_program(agent, &is_program_found) {
        return ProfileStatus::ProgramNotFound;
    }
    if !profile.config_dir().is_dir() {
        return ProfileStatus::ConfigDirNotFound;
    }

    agent.detect(profile)
}

/// Whether the program of `agent` can be started: the login shell finds
/// it by its name (`is_found_by_name`), else it is at one of the agent's
/// program places, the first that holds a program. The agent is told where
/// it starts it from.
fn find_program(
    agent: &dyn Agent,
    is_found_by_name: impl Fn(&str) -> bool,
) -> bool {
    let entry = agent.entry();
    if is_found_by_name(entry.program()) {
        agent.start_program_from(None);
        return true;
    }

    let place = entry
        .program_places()
        .iter()
        .find(|place| is_program_file(place))
        .cloned();
    let is_found = place.is_some();

    agent.start_program_from(place);
    is_found
}

/// Whether `path` is a file its owner, group or anyone may run.
fn is_program_file(path: &Path) -> bool {
    let Ok(metadata) = std::fs::metadata(path) else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
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

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;
    use crate::agent::AgentId;
    use crate::command::AgentCommand;
    #[cfg(unix)]
    use crate::testing::write_program;
    use crate::testing::{
        MockAgent, TempDir, mock_entry, test_agents, write_file,
    };

    fn coder_id() -> AgentId {
        AgentId::new("coder")
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
        let coder_dir = make_dir(&temp.path().join(".coder"));
        let env = env_in(temp.path());

        let list = profiles(
            &test_agents(),
            &env,
            &SavedProfiles::default(),
            no_program,
        );

        assert_eq!(
            list,
            [Profile::default_for(coder_id(), "Coder", coder_dir)]
        );
        assert_eq!(
            test_agents()[0].launch(&list[0], None),
            AgentCommand::new("coder")
        );
    }

    #[test]
    fn a_default_profile_is_in_the_folder_of_the_agents_variable() {
        let temp = TempDir::new();
        make_dir(&temp.path().join(".coder"));
        let coder_home = make_dir(&temp.path().join("data/coder"));
        let env = AgentEnv::new(temp.path().to_path_buf())
            .with_config_dir("CODER_HOME", coder_home.clone());

        let list = profiles(
            &test_agents(),
            &env,
            &SavedProfiles::default(),
            no_program,
        );

        assert_eq!(
            list,
            [Profile::default_for(coder_id(), "Coder", coder_home)]
        );
        assert_eq!(
            test_agents()[0].launch(&list[0], None),
            AgentCommand::new("coder")
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
            |program| program == "coder",
        );

        assert_eq!(
            list,
            [Profile::default_for(
                coder_id(),
                "Coder",
                temp.path().join(".coder")
            )]
        );
    }

    #[test]
    fn the_users_changes_apply_to_the_default_profile() {
        let temp = TempDir::new();
        make_dir(&temp.path().join(".coder"));
        let moved_dir = make_dir(&temp.path().join("data/coder"));
        let env = env_in(temp.path());
        let saved = SavedProfiles::default().with_default_changes(
            DefaultProfileChanges::new(coder_id())
                .with_name("Coder · Home")
                .with_config_dir(moved_dir.clone()),
        );

        let list = profiles(&test_agents(), &env, &saved, no_program);

        let expected =
            Profile::default_for(coder_id(), "Coder · Home", moved_dir.clone())
                .with_agent_default_dir(&temp.path().join(".coder"));
        assert_eq!(list, [expected]);
        assert_eq!(
            test_agents()[0].launch(&list[0], None),
            AgentCommand::new("coder")
                .with_env("CODER_HOME", moved_dir.to_string_lossy())
        );
    }

    #[test]
    fn a_changed_default_profile_of_an_agent_not_installed_is_left_out() {
        let temp = TempDir::new();
        let env = env_in(temp.path());
        let saved = SavedProfiles::default().with_default_changes(
            DefaultProfileChanges::new(coder_id()).with_name("Coder · Home"),
        );

        let list = profiles(&test_agents(), &env, &saved, no_program);

        assert!(list.is_empty());
    }

    #[test]
    fn added_profiles_come_after_the_default_ones_in_their_order() {
        let temp = TempDir::new();
        let coder_dir = make_dir(&temp.path().join(".coder"));
        let env = env_in(temp.path());
        let coder_work = Profile::added(
            coder_id(),
            "Coder · Work",
            temp.path().join(".coder-work"),
        );
        let coder_home = Profile::added(
            coder_id(),
            "Coder · Home",
            temp.path().join(".coder-home"),
        );
        let future = Profile::added(
            AgentId::new("future-agent"),
            "Future",
            temp.path().join(".future"),
        );
        let saved = SavedProfiles::default()
            .with_added(coder_work.clone())
            .with_added(future)
            .with_added(coder_home.clone());

        let list = profiles(&test_agents(), &env, &saved, no_program);

        assert_eq!(
            list,
            [
                Profile::default_for(coder_id(), "Coder", coder_dir),
                coder_work,
                coder_home,
            ]
        );
    }

    #[test]
    fn an_added_profile_in_the_agents_default_folder_sets_no_variable() {
        let temp = TempDir::new();
        let moved_dir = make_dir(&temp.path().join("data/coder"));
        let env = env_in(temp.path());
        let saved = SavedProfiles::default()
            .with_default_changes(
                DefaultProfileChanges::new(coder_id())
                    .with_config_dir(moved_dir),
            )
            .with_added(Profile::added(
                coder_id(),
                "Coder · Old",
                temp.path().join(".coder"),
            ));

        let list = profiles(&test_agents(), &env, &saved, no_program);

        let commands: Vec<_> = list
            .iter()
            .map(|profile| test_agents()[0].launch(profile, None))
            .collect();
        assert_eq!(
            commands,
            [
                AgentCommand::new("coder").with_env(
                    "CODER_HOME",
                    temp.path().join("data/coder").to_string_lossy()
                ),
                AgentCommand::new("coder"),
            ]
        );
    }

    /// A Coder session file and a Coder login in `config_dir`, made up.
    fn coder_session_and_login(config_dir: &Path) {
        write_file(
            &config_dir.join(
                "sessions/2026/11/19/rollout-2026-11-19T10-00-00-made-up.jsonl",
            ),
            r#"{"type":"session_meta","payload":{"id":"made-up","cwd":"/work"}}"#,
        );
        write_file(&config_dir.join("auth.json"), "{}");
    }

    /// Spec 2026-11-23, Decision 4: OTTY does not look for other folders
    /// of the agent in the home folder, even ones with a login and a
    /// session.
    #[test]
    fn an_extra_folder_of_the_agent_is_not_a_profile() {
        let temp = TempDir::new();
        let coder_dir = make_dir(&temp.path().join(".coder"));
        coder_session_and_login(&temp.path().join(".coder-work"));
        coder_session_and_login(&temp.path().join(".coder_old"));
        let env = env_in(temp.path());

        let list = profiles(
            &test_agents(),
            &env,
            &SavedProfiles::default(),
            no_program,
        );

        assert_eq!(
            list,
            [Profile::default_for(coder_id(), "Coder", coder_dir)]
        );
    }

    /// Spec 2026-11-23, Decision 4: another folder of the agent is a
    /// profile once the user adds it.
    #[test]
    fn an_extra_folder_the_user_added_is_a_profile() {
        let temp = TempDir::new();
        let coder_dir = make_dir(&temp.path().join(".coder"));
        let work = temp.path().join(".coder-work");
        coder_session_and_login(&work);
        let env = env_in(temp.path());
        let added = Profile::added(coder_id(), "Coder · Work", work.clone());
        let saved = SavedProfiles::default().with_added(added.clone());

        let list = profiles(&test_agents(), &env, &saved, no_program);

        assert_eq!(
            list,
            [Profile::default_for(coder_id(), "Coder", coder_dir), added]
        );
        assert_eq!(
            test_agents()[0].launch(&list[1], None),
            AgentCommand::new("coder")
                .with_env("CODER_HOME", work.to_string_lossy())
        );
    }

    /// A made-up agent whose program is `mock-agent`.
    fn mock() -> MockAgent {
        let mut agent = MockAgent::new();
        agent
            .expect_entry()
            .return_const(mock_entry("mock", "mock-agent"));
        agent
    }

    /// An added profile of the made-up agent in the folder `folder`.
    fn mock_profile_in(folder: &Path) -> Profile {
        Profile::added(AgentId::new("mock"), "Mock", folder.to_path_buf())
    }

    /// An added profile of the made-up agent in a folder that is not
    /// there.
    fn mock_profile() -> Profile {
        mock_profile_in(Path::new("/no/such/mock"))
    }

    fn finds_mock(program: &str) -> bool {
        program == "mock-agent"
    }

    #[test]
    fn a_profile_whose_program_is_not_found_says_so_before_any_check() {
        let mut agent = mock();
        agent.expect_detect().never();

        let status = profile_status(&agent, &mock_profile(), no_program);

        assert_eq!(status, ProfileStatus::ProgramNotFound);
    }

    /// A made-up agent whose program `mock-agent` its installer puts at
    /// one of `places`.
    fn mock_with_places(places: Vec<PathBuf>) -> MockAgent {
        let mut agent = MockAgent::new();
        agent.expect_entry().return_const(
            mock_entry("mock", "mock-agent").with_program_places(places),
        );
        agent
    }

    /// Issue 32: the login shell does not find `mock-agent` by its name,
    /// but its installer put it at the second of its program places.
    #[cfg(unix)]
    #[test]
    fn a_program_at_a_program_place_is_found_when_the_shell_finds_nothing() {
        let temp = TempDir::new();
        let place = temp.path().join("bin/mock-agent");
        write_program(&place);
        let mut agent = mock_with_places(vec![
            temp.path().join("missing/mock-agent"),
            place,
        ]);
        agent
            .expect_detect()
            .return_const(ProfileStatus::NotLoggedIn(None));

        let status =
            profile_status(&agent, &mock_profile_in(temp.path()), no_program);

        assert_eq!(status, ProfileStatus::NotLoggedIn(None));
    }

    /// Issue 32: a default profile with no folder is shown when its
    /// program is at a program place.
    #[cfg(unix)]
    #[test]
    fn a_default_profile_with_no_folder_is_shown_when_its_program_is_at_a_place()
     {
        let temp = TempDir::new();
        let home = make_dir(&temp.path().join("home"));
        let place = temp.path().join("opt/mock/bin/mock-agent");
        write_program(&place);
        let mut agent = mock_with_places(vec![place]);
        agent
            .expect_default_config_dir()
            .return_const(home.join(".mock"));
        let agents: Vec<Box<dyn Agent>> = vec![Box::new(agent)];

        let list = profiles(
            &agents,
            &env_in(&home),
            &SavedProfiles::default(),
            no_program,
        );

        assert_eq!(
            list,
            [Profile::default_for(
                AgentId::new("mock"),
                "mock",
                home.join(".mock")
            )]
        );
    }

    /// Issue 32: a file that is not a program, or no file, at a program
    /// place is no program.
    #[cfg(unix)]
    #[test]
    fn a_place_with_no_program_finds_nothing() {
        let temp = TempDir::new();
        let not_a_program = temp.path().join("bin/mock-agent");
        write_file(&not_a_program, "not a program");
        let mut agent = mock_with_places(vec![
            not_a_program,
            temp.path().join("bin"),
            temp.path().join("missing/mock-agent"),
        ]);
        agent.expect_detect().never();

        let status = profile_status(&agent, &mock_profile(), no_program);

        assert_eq!(status, ProfileStatus::ProgramNotFound);
    }

    #[test]
    fn a_profile_whose_program_is_found_has_the_state_its_agent_checks() {
        let temp = TempDir::new();
        let mut agent = mock();
        agent
            .expect_detect()
            .return_const(ProfileStatus::NotLoggedIn(None));

        let status =
            profile_status(&agent, &mock_profile_in(temp.path()), finds_mock);

        assert_eq!(status, ProfileStatus::NotLoggedIn(None));
    }

    /// Issue 12 of spec 2026-11-23-accounts-apart-from-profiles: OTTY
    /// itself tells a folder that is not there, and does not ask the
    /// agent.
    #[test]
    fn a_folder_that_is_not_there_is_not_found_and_the_agent_is_not_asked() {
        let mut agent = mock();
        agent.expect_detect().never();

        let status = profile_status(&agent, &mock_profile(), finds_mock);

        assert_eq!(status, ProfileStatus::ConfigDirNotFound);
    }

    /// Issue 12: an empty folder is there. The agent says what it needs,
    /// such as a login, or that it needs nothing.
    #[test]
    fn an_empty_folder_is_there_and_its_agent_says_what_it_needs() {
        let temp = TempDir::new();
        let mut agent = mock();
        agent
            .expect_detect()
            .times(1)
            .return_const(ProfileStatus::Connected);

        let status =
            profile_status(&agent, &mock_profile_in(temp.path()), finds_mock);

        assert_eq!(status, ProfileStatus::Connected);
    }

    /// Issue 12: a file in place of the folder is no folder.
    #[test]
    fn a_file_in_place_of_the_folder_is_not_found() {
        let temp = TempDir::new();
        let file = temp.path().join("not-a-folder");
        write_file(&file, "text");
        let mut agent = mock();
        agent.expect_detect().never();

        let status =
            profile_status(&agent, &mock_profile_in(&file), finds_mock);

        assert_eq!(status, ProfileStatus::ConfigDirNotFound);
    }
}
