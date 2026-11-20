//! The config folders of Claude Code on this computer: the default one,
//! `CLAUDE_CONFIG_DIR` or `~/.claude`, and the folders next to it named
//! `.claude-<name>` or `.claude_<name>` that hold a login or a session.

use std::collections::BTreeSet;
use std::path::Path;

use otty_plugin_sdk::{ProfilePlace, ProfileStatus, env, files};

use crate::detect::status;
use crate::glob::escaped;

/// The variable that names another default config folder.
pub(crate) const CONFIG_VARIABLE: &str = "CLAUDE_CONFIG_DIR";

/// The default config folder in the home folder.
const DEFAULT_FOLDER: &str = ".claude";

/// The files that make a folder next to the default one a candidate:
/// a login, a global config file, settings, or a session file.
const CANDIDATE_FILES: [(&str, usize); 4] = [
    (".credentials.json", 1),
    (".claude.json", 1),
    ("settings.json", 1),
    ("projects/*/*.jsonl", 3),
];

/// The config folders of Claude Code, the default one first, then the
/// others found next to `~/.claude`, by name.
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

/// The folders `~/.claude-<name>` and `~/.claude_<name>` that hold a
/// login or a session file, by name.
fn found_folders(home: &str) -> Vec<String> {
    let base = format!("{}/{DEFAULT_FOLDER}[-_]*", escaped(home));

    let mut with_sessions = BTreeSet::new();
    let mut candidates = BTreeSet::new();
    for (file, depth) in CANDIDATE_FILES {
        let found = files::list(&format!("{base}/{file}")).unwrap_or_default();
        for info in found {
            let Some(folder) = ancestor(&info.path, depth) else {
                continue;
            };
            if file.ends_with(".jsonl") {
                with_sessions.insert(folder.clone());
            }
            candidates.insert(folder);
        }
    }

    candidates
        .into_iter()
        .filter(|folder| {
            with_sessions.contains(folder)
                || matches!(status(folder), ProfileStatus::Connected(_))
        })
        .collect()
}

/// The folder `depth` levels above `path`.
fn ancestor(path: &str, depth: usize) -> Option<String> {
    let mut folder = Path::new(path);
    for _ in 0..depth {
        folder = folder.parent()?;
    }

    Some(folder.display().to_string())
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
            (".claude", true),
            (".claude-account", false),
            (".claude-api", false),
            (".claude-console", false),
            (".claude-deepseek", false),
            (".claude-email", false),
            (".claude-gateway", false),
            (".claude-helper", false),
            (".claude-local", false),
            (".claude-mixed", false),
            (".claude-nullkey", false),
            (".claude-prepaid", false),
            (".claude-prepaid-max", false),
            (".claude-token", false),
            (".claude-work", false),
            (".claude_old", false),
        ]
        .into_iter()
        .map(|(name, is_default)| (path(&host, name), is_default))
        .collect();
        assert_eq!(found, expected);
    }

    #[test]
    fn the_variable_names_the_default_folder() {
        let host = host();
        let work = path(&host, ".claude-work");
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
    fn a_folder_is_the_given_number_of_levels_up() {
        assert_eq!(
            ancestor("/home/dev/.claude-x/projects/-a/s.jsonl", 3).as_deref(),
            Some("/home/dev/.claude-x")
        );
        assert_eq!(ancestor("/", 1), None);
    }
}
