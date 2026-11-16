//! The list of profiles the mode shows: the default profile of each known
//! agent with the user's changes, then the profiles the user added.

use crate::agent::Agent;
use crate::env::AgentEnv;
use crate::profile::Profile;
use crate::saved_profiles::{DefaultProfileChanges, SavedProfiles};

/// The profiles to show, hidden ones included: for each agent of `agents`
/// its default profile with the user's changes from `saved`, then the
/// profiles the user added, in the order they were added.
///
/// The default profile's folder is the agent's default config folder (from
/// its variable in `env`, else in the home folder), unless the user moved
/// it. A default profile is left out when its folder does not exist and its
/// program is not found either: then the agent is not installed.
/// `is_program_found` says whether a program, given by its name or by its
/// path, can be started; the app looks it up the way it starts it. An added
/// profile of an agent not in `agents` is left out.
///
/// Each profile knows whether its folder is the agent's default folder, so
/// its commands set the agent's config folder variable only when it is
/// not.
///
/// ```
/// use std::path::PathBuf;
///
/// use otty_agents::{AgentEnv, SavedProfiles, known_agents, profiles};
///
/// let env = AgentEnv::new(PathBuf::from("/no/such/home"), None, None);
/// let saved = SavedProfiles::default();
///
/// let list =
///     profiles(&known_agents(), &env, &saved, |program| program == "codex");
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
        let changes = saved.default_changes_of(agent.descriptor().id());
        let profile = default_profile(agent.as_ref(), env, changes);

        let command = agent.launch(&profile, None);
        let is_installed = profile.config_dir().is_dir()
            || is_program_found(command.program());
        if is_installed {
            list.push(profile);
        }
    }

    for profile in saved.added() {
        let agent = agents
            .iter()
            .find(|agent| agent.descriptor().id() == profile.agent());
        if let Some(agent) = agent {
            let default_dir = agent.default_config_dir(env);
            list.push(profile.clone().with_agent_default_dir(&default_dir));
        }
    }

    list
}

/// The default profile of `agent` with the user's `changes`, if any.
fn default_profile(
    agent: &dyn Agent,
    env: &AgentEnv,
    changes: Option<&DefaultProfileChanges>,
) -> Profile {
    let descriptor = agent.descriptor();
    let default_dir = agent.default_config_dir(env);

    let Some(changes) = changes else {
        return Profile::default_for(
            descriptor.id(),
            descriptor.name(),
            default_dir,
        );
    };

    let name = changes.name().unwrap_or(descriptor.name());
    let config_dir = changes.config_dir().unwrap_or(&default_dir);
    let profile =
        Profile::default_for(descriptor.id(), name, config_dir.to_path_buf())
            .with_agent_default_dir(&default_dir)
            .with_hidden(changes.is_hidden())
            .with_admin_key(changes.admin_key().cloned())
            .with_monthly_budget(changes.monthly_budget());

    match changes.program() {
        Some(program) => profile.with_program(program.to_path_buf()),
        None => profile,
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;
    use crate::admin_key::AdminKeyRef;
    use crate::agent::AgentId;
    use crate::command::AgentCommand;
    use crate::dollars::Dollars;
    use crate::known_agents;
    use crate::testing::TempDir;

    const CLAUDE: AgentId = AgentId::new("claude-code");
    const CODEX: AgentId = AgentId::new("codex");

    fn env_in(home: &Path) -> AgentEnv {
        AgentEnv::new(home.to_path_buf(), None, None)
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
        let claude_dir = make_dir(&temp.path().join(".claude"));
        let env = env_in(temp.path());

        let list = profiles(
            &known_agents(),
            &env,
            &SavedProfiles::default(),
            no_program,
        );

        assert_eq!(
            list,
            [Profile::default_for(CLAUDE, "Claude Code", claude_dir)]
        );
        assert_eq!(
            known_agents()[0].launch(&list[0], None),
            AgentCommand::new("claude")
        );
    }

    #[test]
    fn a_default_profile_is_in_the_folder_of_the_agents_variable() {
        let temp = TempDir::new();
        make_dir(&temp.path().join(".codex"));
        let codex_home = make_dir(&temp.path().join("data/codex"));
        let env = AgentEnv::new(
            temp.path().to_path_buf(),
            None,
            Some(codex_home.clone()),
        );

        let list = profiles(
            &known_agents(),
            &env,
            &SavedProfiles::default(),
            no_program,
        );

        assert_eq!(list, [Profile::default_for(CODEX, "Codex", codex_home)]);
        assert_eq!(
            known_agents()[1].launch(&list[0], None),
            AgentCommand::new("codex")
        );
    }

    #[test]
    fn a_default_profile_with_no_folder_is_shown_when_its_program_is_found() {
        let temp = TempDir::new();
        let env = env_in(temp.path());

        let list = profiles(
            &known_agents(),
            &env,
            &SavedProfiles::default(),
            |program| program == "claude",
        );

        assert_eq!(
            list,
            [Profile::default_for(
                CLAUDE,
                "Claude Code",
                temp.path().join(".claude")
            )]
        );
    }

    #[test]
    fn the_program_path_the_user_set_is_the_one_looked_up() {
        let temp = TempDir::new();
        let env = env_in(temp.path());
        let saved = SavedProfiles::default().with_default_changes(
            DefaultProfileChanges::new(CODEX)
                .with_program(PathBuf::from("/opt/codex/codex")),
        );

        let list = profiles(&known_agents(), &env, &saved, |program| {
            program == "/opt/codex/codex"
        });

        let names: Vec<_> = list.iter().map(Profile::name).collect();
        assert_eq!(names, ["Codex"]);
        assert_eq!(list[0].program(), Some(Path::new("/opt/codex/codex")));
    }

    #[test]
    fn the_users_changes_apply_to_the_default_profile() {
        let temp = TempDir::new();
        make_dir(&temp.path().join(".claude"));
        let moved_dir = make_dir(&temp.path().join("data/claude"));
        let env = env_in(temp.path());
        let saved = SavedProfiles::default().with_default_changes(
            DefaultProfileChanges::new(CLAUDE)
                .with_name("Claude Code · Home")
                .with_config_dir(moved_dir.clone())
                .with_program(PathBuf::from("/opt/claude/bin/claude")),
        );

        let list = profiles(&known_agents(), &env, &saved, no_program);

        let expected = Profile::default_for(
            CLAUDE,
            "Claude Code · Home",
            moved_dir.clone(),
        )
        .with_program(PathBuf::from("/opt/claude/bin/claude"))
        .with_agent_default_dir(&temp.path().join(".claude"));
        assert_eq!(list, [expected]);
        assert_eq!(
            known_agents()[0].launch(&list[0], None),
            AgentCommand::new("/opt/claude/bin/claude")
                .with_env("CLAUDE_CONFIG_DIR", moved_dir.to_string_lossy())
        );
    }

    #[test]
    fn the_admin_key_and_the_budget_apply_to_the_default_profile() {
        let temp = TempDir::new();
        make_dir(&temp.path().join(".claude"));
        let env = env_in(temp.path());
        let admin_key =
            AdminKeyRef::EnvVar(String::from("ANTHROPIC_ADMIN_KEY"));
        let budget = Dollars::from_micro_dollars(300_000_000);
        let saved = SavedProfiles::default().with_default_changes(
            DefaultProfileChanges::new(CLAUDE)
                .with_admin_key(Some(admin_key.clone()))
                .with_monthly_budget(Some(budget)),
        );

        let list = profiles(&known_agents(), &env, &saved, no_program);

        assert_eq!(list[0].admin_key(), Some(&admin_key));
        assert_eq!(list[0].monthly_budget(), Some(budget));
    }

    #[test]
    fn a_hidden_default_profile_stays_in_the_list_marked_hidden() {
        let temp = TempDir::new();
        let codex_dir = make_dir(&temp.path().join(".codex"));
        let env = env_in(temp.path());
        let saved = SavedProfiles::default().with_default_changes(
            DefaultProfileChanges::new(CODEX).with_hidden(true),
        );

        let list = profiles(&known_agents(), &env, &saved, no_program);

        assert_eq!(
            list,
            [
                Profile::default_for(CODEX, "Codex", codex_dir)
                    .with_hidden(true)
            ]
        );
    }

    #[test]
    fn a_hidden_default_profile_of_an_agent_not_installed_is_left_out() {
        let temp = TempDir::new();
        let env = env_in(temp.path());
        let saved = SavedProfiles::default().with_default_changes(
            DefaultProfileChanges::new(CODEX).with_hidden(true),
        );

        let list = profiles(&known_agents(), &env, &saved, no_program);

        assert!(list.is_empty());
    }

    #[test]
    fn added_profiles_come_after_the_default_ones_in_their_order() {
        let temp = TempDir::new();
        let claude_dir = make_dir(&temp.path().join(".claude"));
        let env = env_in(temp.path());
        let codex_work = Profile::added(
            CODEX,
            "Codex · Work",
            temp.path().join(".codex-work"),
        )
        .with_hidden(true);
        let claude_work = Profile::added(
            CLAUDE,
            "Claude Code · Work",
            temp.path().join(".claude-work"),
        );
        let future = Profile::added(
            AgentId::new("future-agent"),
            "Future",
            temp.path().join(".future"),
        );
        let saved = SavedProfiles::default()
            .with_added(codex_work.clone())
            .with_added(future)
            .with_added(claude_work.clone());

        let list = profiles(&known_agents(), &env, &saved, no_program);

        assert_eq!(
            list,
            [
                Profile::default_for(CLAUDE, "Claude Code", claude_dir),
                codex_work,
                claude_work,
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
                DefaultProfileChanges::new(CODEX).with_config_dir(moved_dir),
            )
            .with_added(Profile::added(
                CODEX,
                "Codex · Old",
                temp.path().join(".codex"),
            ));

        let list = profiles(&known_agents(), &env, &saved, no_program);

        let commands: Vec<_> = list
            .iter()
            .map(|profile| known_agents()[1].launch(profile, None))
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
}
