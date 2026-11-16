//! The saved profiles in `agents.json` in the app's config folder
//! (`~/.config/otty`), next to `modes.json`.

use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use otty_agents::{Agent, SavedProfiles};
use otty_app_i18n::{self as i18n, Key};

/// Read the saved profiles in `config_dir`, with the text that tells the
/// user when the file could not be read. A missing file is the first
/// start: no changes and nothing to tell. A file that can not be read
/// gives no changes and the text.
pub(crate) fn read_saved(
    config_dir: &Path,
    agents: &[Box<dyn Agent>],
) -> (SavedProfiles, Option<String>) {
    let path = profiles_file(config_dir);

    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) if err.kind() == ErrorKind::NotFound => {
            return (SavedProfiles::default(), None);
        },
        Err(err) => {
            return (
                SavedProfiles::default(),
                Some(problem(&path, &format!("{err}"))),
            );
        },
    };

    match SavedProfiles::from_json(&text, agents) {
        Ok(saved) => (saved, None),
        Err(err) => (
            SavedProfiles::default(),
            Some(problem(&path, &format!("{err}"))),
        ),
    }
}

/// Log why the file at `path` could not be read and return the text that
/// tells the user.
fn problem(path: &Path, error: &str) -> String {
    log::warn!(
        "{}: {error}; using the default agent profiles",
        path.display()
    );

    i18n::with_error(Key::TplAgentsProfilesReadFailed, error)
}

/// Return the path of the saved profiles in `config_dir`.
fn profiles_file(config_dir: &Path) -> PathBuf {
    config_dir.join("agents.json")
}

#[cfg(test)]
mod tests {
    use otty_agents::known_agents;
    use otty_app_testing::temp_dir::TempDir;

    use super::*;

    fn folder_with(name: &str, contents: &str) -> TempDir {
        let folder = TempDir::new(name);
        std::fs::write(profiles_file(folder.path()), contents)
            .expect("agents.json should be written");

        folder
    }

    #[test]
    fn a_missing_file_is_no_change_and_nothing_to_tell() {
        let folder = TempDir::new("agents-file-missing");

        let (saved, problem) = read_saved(folder.path(), &known_agents());

        assert_eq!(saved, SavedProfiles::default());
        assert_eq!(problem, None);
    }

    #[test]
    fn the_saved_profiles_are_read_from_agents_json() {
        let folder = folder_with(
            "agents-file-read",
            r#"{"added_profiles":[{"agent":"codex","name":"Codex · Work","config_dir":"/home/user/.codex-work"}]}"#,
        );

        let (saved, problem) = read_saved(folder.path(), &known_agents());

        assert_eq!(problem, None);
        let names: Vec<_> =
            saved.added().iter().map(|profile| profile.name()).collect();
        assert_eq!(names, ["Codex · Work"]);
    }

    #[test]
    fn a_broken_file_gives_no_change_and_tells_why() {
        let folder = folder_with("agents-file-broken", "{ not json");

        let (saved, problem) = read_saved(folder.path(), &known_agents());

        assert_eq!(saved, SavedProfiles::default());
        let problem = problem.expect("the user is told");
        let start = i18n::with_error(Key::TplAgentsProfilesReadFailed, "");
        assert!(problem.starts_with(&start), "{problem}");
        assert!(problem.len() > start.len());
    }

    #[test]
    fn a_file_that_can_not_be_read_tells_why() {
        let folder = TempDir::new("agents-file-folder");
        std::fs::create_dir(profiles_file(folder.path()))
            .expect("a folder in place of the file");

        let (saved, problem) = read_saved(folder.path(), &known_agents());

        assert_eq!(saved, SavedProfiles::default());
        assert!(problem.is_some());
    }
}
