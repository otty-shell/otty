//! The config folder typed in the profile dialog, and what is wrong with it.

use std::path::Path;

use otty_agents::{
    AgentId, Profile, ProfileInputError, check_profile_config_dir,
    is_own_login_folder,
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
    /// The folder is inside OTTY's own login folders of a plugin, which
    /// Remove of an own login deletes (issue 13 of spec
    /// 2026-11-23-accounts-apart-from-profiles).
    OwnLogin,
}

impl From<PathProblem> for FolderProblem {
    fn from(problem: PathProblem) -> Self {
        match problem {
            PathProblem::Empty => Self::Empty,
            PathProblem::NotFull => Self::NotFull,
        }
    }
}

/// Checks that `folder` is no other profile's config folder of `agent`,
/// and not inside OTTY's own login folders in `plugins_folder`.
pub(crate) fn check_folder(
    agent: &AgentId,
    folder: &Path,
    editing: Option<&Profile>,
    profiles: &[&Profile],
    plugins_folder: Option<&Path>,
) -> Result<(), FolderProblem> {
    let is_own_login = plugins_folder
        .is_some_and(|plugins| is_own_login_folder(folder, plugins));
    if is_own_login {
        return Err(FolderProblem::OwnLogin);
    }

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

        let taken = check_folder(&agent, folder, None, &[&other], None);
        let own = check_folder(&agent, folder, Some(&other), &[&other], None);

        assert_eq!(taken, Err(FolderProblem::Taken));
        assert_eq!(own, Ok(()));
    }
}
