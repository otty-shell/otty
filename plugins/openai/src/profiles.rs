//! The config folders of Codex on this computer: the default one,
//! `CODEX_HOME` or `~/.codex`, and the folders next to it named
//! `.codex-<name>` or `.codex_<name>` that hold a login or a session.

use std::collections::BTreeSet;

use otty_plugin_sdk::{ProfilePlace, env, files};

use crate::detect::connection;
use crate::glob::escaped;
use crate::sessions::session_file_globs;

/// The variable that names another default config folder.
pub(crate) const CONFIG_VARIABLE: &str = "CODEX_HOME";

/// The default config folder in the home folder.
const DEFAULT_FOLDER: &str = ".codex";

/// The login file that makes a folder next to the default one a
/// candidate.
const LOGIN_FILE: &str = "auth.json";

/// The config folders of Codex, the default one first, then the others
/// found next to `~/.codex`, by name.
pub(crate) fn profiles() -> Vec<ProfilePlace> {
    let home = env::home();
    let default = env::var(CONFIG_VARIABLE)
        .filter(|folder| !folder.trim().is_empty())
        .unwrap_or_else(|| format!("{home}/{DEFAULT_FOLDER}"));

    let found: Vec<ProfilePlace> = found_folders(&home)
        .into_iter()
        .filter(|folder| *folder != default)
        .map(|folder| ProfilePlace {
            folder,
            is_default: false,
        })
        .collect();

    let mut places = vec![ProfilePlace {
        folder: default,
        is_default: true,
    }];
    places.extend(found);

    places
}

/// The folders `~/.codex-<name>` and `~/.codex_<name>` that hold a
/// session file or a login, by name.
fn found_folders(home: &str) -> Vec<String> {
    let folders = format!("{}/{DEFAULT_FOLDER}[-_]*", escaped(home));

    let mut with_sessions = BTreeSet::new();
    for glob in session_file_globs(&folders, "*") {
        for file in files::list(&glob).unwrap_or_default() {
            if let Some(folder) = folder_in_home(home, &file.path) {
                with_sessions.insert(folder);
            }
        }
    }

    let logins = files::list(&format!("{folders}/{LOGIN_FILE}"));
    let logged_in = logins
        .unwrap_or_default()
        .into_iter()
        .filter_map(|file| folder_in_home(home, &file.path))
        .filter(|folder| !with_sessions.contains(folder))
        .filter(|folder| connection(folder).is_some())
        .collect::<Vec<String>>();

    with_sessions.extend(logged_in);
    with_sessions.into_iter().collect()
}

/// The folder right inside `home` that holds `path`.
fn folder_in_home(home: &str, path: &str) -> Option<String> {
    let inside = path.strip_prefix(home)?.strip_prefix('/')?;
    let name = inside.split('/').next().filter(|name| !name.is_empty())?;

    Some(format!("{home}/{name}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{as_part, host, path};

    #[test]
    fn profiles_are_the_default_folder_and_the_found_ones_by_name() {
        let host = host();

        let places = as_part(&host, profiles);

        let found: Vec<(String, bool)> = places
            .into_iter()
            .map(|place| (place.folder, place.is_default))
            .collect();
        let expected: Vec<(String, bool)> = [
            (".codex", true),
            (".codex-both", false),
            (".codex-expired", false),
            (".codex-gateway", false),
            (".codex-idtoken", false),
            (".codex-limits", false),
            (".codex-nameless", false),
            (".codex-nomode", false),
            (".codex-plus", false),
            (".codex-proxy", false),
            (".codex-work", false),
            (".codex_old", false),
        ]
        .into_iter()
        .map(|(name, is_default)| (path(&host, name), is_default))
        .collect();
        assert_eq!(found, expected);
    }

    #[test]
    fn the_variable_names_the_default_folder() {
        let host = host();
        let work = path(&host, ".codex-work");
        let host = host.with_var(CONFIG_VARIABLE, &work);

        let places = as_part(&host, profiles);

        assert_eq!(
            places[0],
            ProfilePlace {
                folder: work.clone(),
                is_default: true,
            }
        );
        assert_eq!(
            places.iter().filter(|place| place.folder == work).count(),
            1
        );
    }

    #[test]
    fn a_path_belongs_to_the_folder_right_inside_home() {
        assert_eq!(
            folder_in_home("/home/dev", "/home/dev/.codex-x/sessions/a.jsonl")
                .as_deref(),
            Some("/home/dev/.codex-x")
        );
        assert_eq!(folder_in_home("/home/dev", "/data/.codex-x/a"), None);
        assert_eq!(folder_in_home("/home/dev", "/home/devil/.codex"), None);
    }
}
