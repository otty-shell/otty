//! The config folder of a new profile: the folders found in the home
//! folder, the folder proposed from the profile's name, and creating it.

use std::path::{Path, PathBuf};

use crate::agent::{Agent, AgentDescriptor};
use crate::env::AgentEnv;
use crate::profile::Profile;

/// Why the config folder of a new profile could not be created.
#[derive(Debug, thiserror::Error)]
pub enum CreateConfigDirError {
    /// Something that is not a folder is already at the path.
    #[error("{} is already there and is not a folder", .0.display())]
    NotAFolder(PathBuf),
    /// The folder is already there and has files in it.
    #[error("the folder {} already has files in it", .0.display())]
    NotEmpty(PathBuf),
    /// The folder could not be created or looked into.
    #[error("the folder {} can not be created: {source}", path.display())]
    Io {
        /// The path of the folder.
        path: PathBuf,
        /// What the system said.
        #[source]
        source: std::io::Error,
    },
}

/// The folders directly inside the home folder that look like a config
/// folder of `agent` and are not the folder of one of its `profiles` yet,
/// sorted by path. Empty when the home folder can not be read.
pub fn candidate_config_dirs(
    agent: &dyn Agent,
    env: &AgentEnv,
    profiles: &[Profile],
) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(env.home()) else {
        return Vec::new();
    };

    let id = agent.descriptor().id();
    let is_used = |dir: &Path| {
        profiles
            .iter()
            .any(|profile| profile.agent() == id && profile.config_dir() == dir)
    };
    let mut candidates: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .filter(|path| agent.looks_like_config_dir(path) && !is_used(path))
        .collect();

    candidates.sort();
    candidates
}

/// The folder proposed for a new profile of `agent` named `name`: the
/// agent's config folder name in the home folder, a `-` and the name in
/// lowercase, such as `~/.claude-semrush` for "Semrush".
///
/// Only the ASCII letters and digits of the name are kept; each run of
/// other characters becomes one `-`, and no `-` is left at the ends.
/// `None` when nothing is left.
///
/// ```
/// use std::path::PathBuf;
///
/// use otty_agents::{Agent, AgentEnv, ClaudeCode, proposed_config_dir};
///
/// let env = AgentEnv::new(PathBuf::from("/home/user"), None, None);
///
/// let folder = proposed_config_dir(
///     ClaudeCode::default().descriptor(),
///     &env,
///     "Semrush",
/// );
///
/// assert_eq!(folder, Some(PathBuf::from("/home/user/.claude-semrush")));
/// ```
pub fn proposed_config_dir(
    agent: &AgentDescriptor,
    env: &AgentEnv,
    name: &str,
) -> Option<PathBuf> {
    let suffix = folder_suffix(name)?;
    let folder = format!("{}-{suffix}", agent.config_dir_name());

    Some(env.home().join(folder))
}

/// Creates the config folder of a new profile at `path`, with the folders
/// on the way.
///
/// An empty folder already at `path` is used as it is. Anything else
/// already at `path` is refused, so OTTY never mixes a new profile into
/// files it does not know.
pub fn create_config_dir(path: &Path) -> Result<(), CreateConfigDirError> {
    let io_error = |source| CreateConfigDirError::Io {
        path: path.to_path_buf(),
        source,
    };

    if !path.exists() {
        return std::fs::create_dir_all(path).map_err(io_error);
    }
    if !path.is_dir() {
        return Err(CreateConfigDirError::NotAFolder(path.to_path_buf()));
    }

    let mut entries = std::fs::read_dir(path).map_err(io_error)?;
    if entries.next().is_some() {
        return Err(CreateConfigDirError::NotEmpty(path.to_path_buf()));
    }

    Ok(())
}

/// `name` in lowercase ASCII letters and digits, with each run of other
/// characters turned into one `-` and no `-` at the ends. `None` when
/// nothing is left.
fn folder_suffix(name: &str) -> Option<String> {
    let mut suffix = String::new();
    let mut is_after_other = false;

    for character in name.chars() {
        if !character.is_ascii_alphanumeric() {
            is_after_other = true;
            continue;
        }

        if is_after_other && !suffix.is_empty() {
            suffix.push('-');
        }
        suffix.push(character.to_ascii_lowercase());
        is_after_other = false;
    }

    (!suffix.is_empty()).then_some(suffix)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::AgentId;
    use crate::claude_code::ClaudeCode;
    use crate::codex::Codex;
    use crate::testing::{TempDir, write_file};

    fn env_in(home: &Path) -> AgentEnv {
        AgentEnv::new(home.to_path_buf(), None, None)
    }

    fn make_claude_dir(path: &Path) {
        std::fs::create_dir_all(path.join("projects")).expect("create folder");
        write_file(&path.join(".claude.json"), "{}");
    }

    fn propose(name: &str) -> Option<PathBuf> {
        let env = env_in(Path::new("/home/user"));

        proposed_config_dir(ClaudeCode::default().descriptor(), &env, name)
    }

    #[test]
    fn candidates_are_the_folders_of_the_agent_that_are_no_profile_yet() {
        let temp = TempDir::new();
        let home = temp.path();
        make_claude_dir(&home.join(".claude-work"));
        make_claude_dir(&home.join(".claude-home"));
        make_claude_dir(&home.join(".claude-used"));
        make_claude_dir(&home.join("projects/.claude-deep"));
        std::fs::create_dir_all(home.join(".codex-work/sessions"))
            .expect("create folder");
        write_file(&home.join(".claude-file"), "{}");
        let profiles = [
            Profile::added(
                ClaudeCode::default().descriptor().id(),
                "Claude Code · Used",
                home.join(".claude-used"),
            ),
            Profile::added(
                AgentId::new("codex"),
                "Codex · Odd",
                home.join(".claude-work"),
            ),
        ];

        let candidates = candidate_config_dirs(
            &ClaudeCode::default(),
            &env_in(home),
            &profiles,
        );

        assert_eq!(
            candidates,
            [home.join(".claude-home"), home.join(".claude-work")]
        );
    }

    #[test]
    fn candidates_follow_what_each_agent_looks_like() {
        let temp = TempDir::new();
        let home = temp.path();
        make_claude_dir(&home.join(".claude-work"));
        write_file(&home.join(".codex-work/auth.json"), "{}");

        let candidates =
            candidate_config_dirs(&Codex::default(), &env_in(home), &[]);

        assert_eq!(candidates, [home.join(".codex-work")]);
    }

    #[test]
    fn a_home_folder_that_can_not_be_read_has_no_candidates() {
        let temp = TempDir::new();
        let env = env_in(&temp.path().join("missing"));

        let candidates =
            candidate_config_dirs(&ClaudeCode::default(), &env, &[]);

        assert!(candidates.is_empty());
    }

    #[test]
    fn the_proposed_folder_is_the_agents_folder_with_the_name() {
        let env = env_in(Path::new("/home/user"));

        let claude = proposed_config_dir(
            ClaudeCode::default().descriptor(),
            &env,
            "Semrush",
        );
        let codex =
            proposed_config_dir(Codex::default().descriptor(), &env, "Semrush");

        assert_eq!(claude, Some(PathBuf::from("/home/user/.claude-semrush")));
        assert_eq!(codex, Some(PathBuf::from("/home/user/.codex-semrush")));
    }

    #[test]
    fn the_proposed_folder_keeps_only_letters_digits_and_single_dashes() {
        assert_eq!(
            propose("Work Account 2"),
            Some(PathBuf::from("/home/user/.claude-work-account-2"))
        );
        assert_eq!(
            propose("  --My__Team / Ops!--  "),
            Some(PathBuf::from("/home/user/.claude-my-team-ops"))
        );
        assert_eq!(
            propose("Team ✓"),
            Some(PathBuf::from("/home/user/.claude-team"))
        );
    }

    #[test]
    fn a_name_with_nothing_usable_has_no_proposed_folder() {
        assert_eq!(propose(""), None);
        assert_eq!(propose("   "), None);
        assert_eq!(propose("-- ! --"), None);
    }

    #[test]
    fn a_new_folder_is_created_with_the_folders_on_the_way() {
        let temp = TempDir::new();
        let path = temp.path().join("home/.claude-semrush");

        let created = create_config_dir(&path);

        assert!(created.is_ok());
        assert!(path.is_dir());
    }

    #[test]
    fn an_empty_folder_already_there_is_used() {
        let temp = TempDir::new();
        let path = temp.path().join(".claude-semrush");
        std::fs::create_dir_all(&path).expect("create folder");

        assert!(create_config_dir(&path).is_ok());
    }

    #[test]
    fn a_folder_with_files_in_it_is_refused() {
        let temp = TempDir::new();
        let path = temp.path().join(".claude-semrush");
        write_file(&path.join("settings.json"), "{}");

        let created = create_config_dir(&path);

        assert!(matches!(
            created,
            Err(CreateConfigDirError::NotEmpty(refused)) if refused == path
        ));
    }

    #[test]
    fn a_file_at_the_path_is_refused() {
        let temp = TempDir::new();
        let path = temp.path().join(".claude-semrush");
        write_file(&path, "");

        let created = create_config_dir(&path);

        assert!(matches!(
            created,
            Err(CreateConfigDirError::NotAFolder(refused)) if refused == path
        ));
    }

    #[test]
    fn a_folder_that_can_not_be_created_is_an_error() {
        let temp = TempDir::new();
        write_file(&temp.path().join("file"), "");
        let path = temp.path().join("file/.claude-semrush");

        let created = create_config_dir(&path);

        assert!(matches!(
            created,
            Err(CreateConfigDirError::Io { path: failed, .. }) if failed == path
        ));
    }
}
