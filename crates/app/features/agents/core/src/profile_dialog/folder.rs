//! The config folder typed in the profile dialog, and what is wrong with it.

use std::path::Path;

use otty_agents::{
    AgentId, Profile, ProfileInputError, check_profile_config_dir,
};

use crate::folder_field::PathProblem;

/// What is wrong with the config folder of the profile dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FolderProblem {
    /// No path was typed or picked.
    Empty,
    /// The path is not a full path, such as `work/.coder`.
    NotFull,
    /// Another profile of the same agent already uses the folder.
    Taken,
}

impl From<PathProblem> for FolderProblem {
    fn from(problem: PathProblem) -> Self {
        match problem {
            PathProblem::Empty => Self::Empty,
            PathProblem::NotFull => Self::NotFull,
        }
    }
}

/// Checks that `folder` is no other profile's config folder of `agent`.
pub(crate) fn check_folder(
    agent: &AgentId,
    folder: &Path,
    editing: Option<&Profile>,
    profiles: &[&Profile],
) -> Result<(), FolderProblem> {
    match check_profile_config_dir(agent, folder, editing, profiles) {
        Err(ProfileInputError::ConfigDirTaken) => Err(FolderProblem::Taken),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn the_folder_of_another_profile_of_the_agent_is_taken() {
        let agent = AgentId::new("codex");
        let other = Profile::added(
            agent.clone(),
            "Codex · Work",
            PathBuf::from("/home/user/.codex-work"),
        );
        let folder = Path::new("/home/user/.codex-work");

        let taken = check_folder(&agent, folder, None, &[&other]);
        let own = check_folder(&agent, folder, Some(&other), &[&other]);

        assert_eq!(taken, Err(FolderProblem::Taken));
        assert_eq!(own, Ok(()));
    }
}
