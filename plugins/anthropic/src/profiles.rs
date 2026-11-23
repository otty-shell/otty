//! The config folder of Claude Code on this computer that OTTY shows by
//! itself: the default one, `CLAUDE_CONFIG_DIR` or `~/.claude`. The user
//! adds other folders as profiles (spec 2026-11-23, Decision 4).

use otty_plugin_sdk::agents::ProfilePlace;
use otty_plugin_sdk::env;

/// The variable that names another default config folder.
pub(crate) const CONFIG_VARIABLE: &str = "CLAUDE_CONFIG_DIR";

/// The default config folder in the home folder.
const DEFAULT_FOLDER: &str = ".claude";

/// The default config folder of Claude Code, the only place given.
pub(crate) fn profiles() -> Vec<ProfilePlace> {
    vec![ProfilePlace {
        folder: default_folder(),
        is_default: true,
    }]
}

/// The config folder Claude Code uses with no variable set by OTTY:
/// `CLAUDE_CONFIG_DIR`, else `~/.claude`.
pub(crate) fn default_folder() -> String {
    env::var(CONFIG_VARIABLE)
        .filter(|folder| !folder.trim().is_empty())
        .unwrap_or_else(|| format!("{}/{DEFAULT_FOLDER}", env::home()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{as_part, host, path};

    /// Spec 2026-11-23, Decision 4: the fixture home holds
    /// `~/.claude-work` and many more Claude Code folders with logins and
    /// sessions, and still the only profile is `~/.claude`.
    #[test]
    fn the_only_profile_is_the_default_folder() {
        let host = host();

        let places = as_part(&host, profiles);

        assert_eq!(
            places,
            [ProfilePlace {
                folder: path(&host, ".claude"),
                is_default: true,
            }]
        );
    }

    #[test]
    fn the_variable_names_the_default_folder() {
        let host = host();
        let work = path(&host, ".claude-work");
        let host = host.with_var(CONFIG_VARIABLE, &work);

        let places = as_part(&host, profiles);

        assert_eq!(
            places,
            [ProfilePlace {
                folder: work,
                is_default: true,
            }]
        );
    }

    #[test]
    fn a_blank_variable_leaves_the_folder_in_the_home_folder() {
        let host = host().with_var(CONFIG_VARIABLE, "  ");

        let places = as_part(&host, profiles);

        assert_eq!(places[0].folder, path(&host, ".claude"));
    }
}
