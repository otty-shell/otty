//! The checks of what the user typed for a profile: its name and its
//! config folder.

use std::path::Path;

use crate::agent::AgentId;
use crate::profile::Profile;

/// What is wrong with the name or the config folder the user typed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ProfileInputError {
    /// The name is empty or only spaces.
    #[error("the name is empty")]
    EmptyName,
    /// Another profile already has this name.
    #[error("another profile already has this name")]
    NameTaken,
    /// Another profile of the same agent already uses this folder.
    #[error("another profile of this agent already uses this folder")]
    ConfigDirTaken,
}

/// Checks the name typed for a profile: it is not empty and no other
/// profile has it. Spaces at the ends do not count.
///
/// `editing` is the profile being edited, if any; its own name is not
/// "another profile's".
///
/// ```
/// use otty_agents::{ProfileInputError, check_profile_name};
///
/// assert_eq!(
///     check_profile_name("  ", None, &[]),
///     Err(ProfileInputError::EmptyName)
/// );
/// assert_eq!(check_profile_name("Semrush", None, &[]), Ok(()));
/// ```
pub fn check_profile_name(
    name: &str,
    editing: Option<&Profile>,
    profiles: &[Profile],
) -> Result<(), ProfileInputError> {
    let name = name.trim();

    if name.is_empty() {
        return Err(ProfileInputError::EmptyName);
    }

    let is_taken =
        other_profiles(profiles, editing).any(|profile| profile.name() == name);
    if is_taken {
        return Err(ProfileInputError::NameTaken);
    }

    Ok(())
}

/// Checks the config folder typed for a profile of `agent`: no other
/// profile of the same agent uses it.
///
/// `editing` is the profile being edited, if any; its own folder is not
/// "another profile's".
pub fn check_profile_config_dir(
    agent: AgentId,
    config_dir: &Path,
    editing: Option<&Profile>,
    profiles: &[Profile],
) -> Result<(), ProfileInputError> {
    let is_taken = other_profiles(profiles, editing).any(|profile| {
        profile.agent() == agent && profile.config_dir() == config_dir
    });

    if is_taken {
        return Err(ProfileInputError::ConfigDirTaken);
    }

    Ok(())
}

/// The profiles of `profiles` other than the one being edited.
fn other_profiles<'a>(
    profiles: &'a [Profile],
    editing: Option<&'a Profile>,
) -> impl Iterator<Item = &'a Profile> {
    profiles
        .iter()
        .filter(move |profile| Some(*profile) != editing)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    const CLAUDE: AgentId = AgentId::new("claude-code");
    const CODEX: AgentId = AgentId::new("codex");

    fn profiles() -> [Profile; 3] {
        [
            Profile::default_for(
                CLAUDE,
                "Claude Code",
                PathBuf::from("/home/user/.claude"),
            ),
            Profile::added(
                CLAUDE,
                "Semrush",
                PathBuf::from("/home/user/.claude-semrush"),
            ),
            Profile::default_for(
                CODEX,
                "Codex",
                PathBuf::from("/home/user/.codex"),
            ),
        ]
    }

    #[test]
    fn an_empty_name_is_refused() {
        let profiles = profiles();

        assert_eq!(
            check_profile_name("", None, &profiles),
            Err(ProfileInputError::EmptyName)
        );
        assert_eq!(
            check_profile_name(" \t ", None, &profiles),
            Err(ProfileInputError::EmptyName)
        );
    }

    #[test]
    fn the_name_of_another_profile_is_refused() {
        let profiles = profiles();

        assert_eq!(
            check_profile_name("Semrush", None, &profiles),
            Err(ProfileInputError::NameTaken)
        );
        assert_eq!(
            check_profile_name("  Codex ", None, &profiles),
            Err(ProfileInputError::NameTaken)
        );
    }

    #[test]
    fn a_new_name_or_the_edited_profiles_own_name_is_fine() {
        let profiles = profiles();

        assert_eq!(check_profile_name("Work", None, &profiles), Ok(()));
        assert_eq!(
            check_profile_name("Semrush", Some(&profiles[1]), &profiles),
            Ok(())
        );
        assert_eq!(
            check_profile_name("Codex", Some(&profiles[1]), &profiles),
            Err(ProfileInputError::NameTaken)
        );
    }

    #[test]
    fn the_folder_of_another_profile_of_the_same_agent_is_refused() {
        let profiles = profiles();

        let taken = check_profile_config_dir(
            CLAUDE,
            Path::new("/home/user/.claude-semrush"),
            None,
            &profiles,
        );

        assert_eq!(taken, Err(ProfileInputError::ConfigDirTaken));
    }

    #[test]
    fn a_new_folder_or_one_of_another_agent_or_the_own_folder_is_fine() {
        let profiles = profiles();

        let new = check_profile_config_dir(
            CLAUDE,
            Path::new("/home/user/.claude-work"),
            None,
            &profiles,
        );
        let other_agent = check_profile_config_dir(
            CODEX,
            Path::new("/home/user/.claude-semrush"),
            None,
            &profiles,
        );
        let own = check_profile_config_dir(
            CLAUDE,
            Path::new("/home/user/.claude-semrush"),
            Some(&profiles[1]),
            &profiles,
        );

        assert_eq!(new, Ok(()));
        assert_eq!(other_agent, Ok(()));
        assert_eq!(own, Ok(()));
    }
}
