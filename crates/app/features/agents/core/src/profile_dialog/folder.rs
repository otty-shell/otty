//! The config folder typed in the profile dialog, and what is wrong with it.

use std::path::{Path, PathBuf};

use otty_agents::{
    AgentId, Profile, ProfileInputError, check_profile_config_dir,
};

/// What is wrong with the config folder of the profile dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FolderProblem {
    /// No path was typed or picked.
    Empty,
    /// The path is not a full path, such as `work/.claude`.
    NotFull,
    /// Another profile of the same agent already uses the folder.
    Taken,
}

/// The full path of the folder typed as `text`: spaces at the ends do not
/// count, and `~` stands for the home folder `home`.
pub(crate) fn full_path(
    text: &str,
    home: &Path,
) -> Result<PathBuf, FolderProblem> {
    let text = text.trim();
    if text.is_empty() {
        return Err(FolderProblem::Empty);
    }

    let path = match text.strip_prefix('~') {
        Some("") => home.to_path_buf(),
        Some(rest) if rest.starts_with('/') => home.join(&rest[1..]),
        _ => PathBuf::from(text),
    };
    if !path.is_absolute() {
        return Err(FolderProblem::NotFull);
    }

    Ok(path)
}

/// Checks that `folder` is no other profile's config folder of `agent`.
pub(crate) fn check_folder(
    agent: AgentId,
    folder: &Path,
    editing: Option<&Profile>,
    profiles: &[Profile],
) -> Result<(), FolderProblem> {
    match check_profile_config_dir(agent, folder, editing, profiles) {
        Err(ProfileInputError::ConfigDirTaken) => Err(FolderProblem::Taken),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOME: &str = "/home/user";

    fn parse(text: &str) -> Result<PathBuf, FolderProblem> {
        full_path(text, Path::new(HOME))
    }

    #[test]
    fn a_full_path_is_taken_as_it_is_without_spaces_at_the_ends() {
        assert_eq!(
            parse("  /data/claude-work "),
            Ok(PathBuf::from("/data/claude-work"))
        );
    }

    #[test]
    fn a_tilde_stands_for_the_home_folder() {
        assert_eq!(
            parse("~/.claude-work"),
            Ok(PathBuf::from("/home/user/.claude-work"))
        );
        assert_eq!(parse("~"), Ok(PathBuf::from(HOME)));
    }

    #[test]
    fn an_empty_or_relative_path_is_refused() {
        assert_eq!(parse("   "), Err(FolderProblem::Empty));
        assert_eq!(parse(".claude-work"), Err(FolderProblem::NotFull));
        assert_eq!(parse("~work/.claude"), Err(FolderProblem::NotFull));
    }

    #[test]
    fn the_folder_of_another_profile_of_the_agent_is_taken() {
        let agent = AgentId::new("codex");
        let other = Profile::added(
            agent,
            "Codex · Work",
            PathBuf::from("/home/user/.codex-work"),
        );
        let folder = Path::new("/home/user/.codex-work");

        let taken =
            check_folder(agent, folder, None, std::slice::from_ref(&other));
        let own = check_folder(
            agent,
            folder,
            Some(&other),
            std::slice::from_ref(&other),
        );

        assert_eq!(taken, Err(FolderProblem::Taken));
        assert_eq!(own, Ok(()));
    }
}
