//! The config folder of Codex on this computer that OTTY shows by itself:
//! the default one, `CODEX_HOME` or `~/.codex`. The user adds other
//! folders as profiles (spec 2026-11-23, Decision 4).

use otty_plugin_sdk::agents::ProfilePlace;
use otty_plugin_sdk::env;

/// The variable that names another default config folder.
pub(crate) const CONFIG_VARIABLE: &str = "CODEX_HOME";

/// The default config folder in the home folder.
const DEFAULT_FOLDER: &str = ".codex";

/// The default config folder of Codex, the only place given.
pub(crate) fn profiles() -> Vec<ProfilePlace> {
    let folder = env::var(CONFIG_VARIABLE)
        .filter(|folder| !folder.trim().is_empty())
        .unwrap_or_else(|| format!("{}/{DEFAULT_FOLDER}", env::home()));

    vec![ProfilePlace {
        folder,
        is_default: true,
    }]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{as_part, host, path};

    /// Spec 2026-11-23, Decision 4: the fixture home holds `~/.codex-work`
    /// and many more Codex folders with logins and sessions, and still the
    /// only profile is `~/.codex`.
    #[test]
    fn the_only_profile_is_the_default_folder() {
        let host = host();

        let places = as_part(&host, profiles);

        assert_eq!(
            places,
            [ProfilePlace {
                folder: path(&host, ".codex"),
                is_default: true,
            }]
        );
    }

    #[test]
    fn the_variable_names_the_default_folder() {
        let host = host();
        let work = path(&host, ".codex-work");
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

        assert_eq!(places[0].folder, path(&host, ".codex"));
    }
}
