//! The saved profiles in `agents.json` in the app's config folder
//! (`~/.config/otty`), next to `modes.json`.

use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use otty_agents::{SavedProfiles, SavedProfilesError};
use otty_app_i18n::{self as i18n, Key};
use thiserror::Error;

/// Why `agents.json` could not be read or written. Its text is in the
/// user's language, as the user is told it.
#[derive(Debug, Error)]
pub(crate) enum ProfilesFileError {
    /// The file or its folder could not be read or written.
    #[error("{}{}", i18n::t(Key::ErrIoPrefix), .0)]
    Io(#[from] std::io::Error),
    /// The file is not the layout of `agents.json`, or the profiles can
    /// not be written as text; carries what the JSON reader said.
    #[error("{}{}", i18n::t(Key::ErrJsonPrefix), .0)]
    Json(String),
}

impl From<SavedProfilesError> for ProfilesFileError {
    fn from(error: SavedProfilesError) -> Self {
        match error {
            SavedProfilesError::Read(json)
            | SavedProfilesError::Write(json) => Self::Json(format!("{json}")),
        }
    }
}

/// Read the saved profiles in `config_dir`, with the text that tells the
/// user when the file could not be read. A missing file is the first
/// start: no changes and nothing to tell. A file that can not be read
/// gives no changes and the text.
pub(crate) fn read_saved(config_dir: &Path) -> (SavedProfiles, Option<String>) {
    let path = profiles_file(config_dir);

    match read_file(&path) {
        Ok(saved) => (saved, None),
        Err(err) => (SavedProfiles::default(), Some(problem(&path, &err))),
    }
}

/// Write `saved` to `agents.json` in `config_dir`, making the folder when
/// it is missing. The text goes to a file next to it first, which then
/// takes its place, so a crash never leaves half a file.
pub(crate) fn write_saved(
    config_dir: &Path,
    saved: &SavedProfiles,
) -> Result<(), ProfilesFileError> {
    let path = profiles_file(config_dir);
    let text = saved.to_json()?;

    let written = std::fs::create_dir_all(config_dir).and_then(|()| {
        let tmp_path = path.with_extension("json.tmp");
        std::fs::write(&tmp_path, text)?;
        std::fs::rename(tmp_path, &path)
    });

    written.map_err(|err| {
        log::warn!("{}: {err}", path.display());
        ProfilesFileError::Io(err)
    })
}

/// Read the saved profiles from the file at `path`; a missing file is no
/// change.
fn read_file(path: &Path) -> Result<SavedProfiles, ProfilesFileError> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) if err.kind() == ErrorKind::NotFound => {
            return Ok(SavedProfiles::default());
        },
        Err(err) => return Err(err.into()),
    };

    Ok(SavedProfiles::from_json(&text)?)
}

/// Log why the file at `path` could not be read and return the text that
/// tells the user.
fn problem(path: &Path, error: &ProfilesFileError) -> String {
    log::warn!(
        "{}: {error}; using the default agent profiles",
        path.display()
    );

    i18n::with_error(Key::TplAgentsProfilesReadFailed, &format!("{error}"))
}

/// Return the path of the saved profiles in `config_dir`.
fn profiles_file(config_dir: &Path) -> PathBuf {
    config_dir.join("agents.json")
}

#[cfg(test)]
mod tests {
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

        let (saved, problem) = read_saved(folder.path());

        assert_eq!(saved, SavedProfiles::default());
        assert_eq!(problem, None);
    }

    #[test]
    fn the_saved_profiles_are_read_from_agents_json() {
        let folder = folder_with(
            "agents-file-read",
            r#"{"added_profiles":[{"agent":"codex","name":"Codex (Work)","config_dir":"/home/user/.codex-work"}]}"#,
        );

        let (saved, problem) = read_saved(folder.path());

        assert_eq!(problem, None);
        let names: Vec<_> =
            saved.added().iter().map(|profile| profile.name()).collect();
        assert_eq!(names, ["Codex (Work)"]);
    }

    #[test]
    fn a_broken_file_gives_no_change_and_tells_why() {
        let folder = folder_with("agents-file-broken", "{ not json");

        let (saved, problem) = read_saved(folder.path());

        assert_eq!(saved, SavedProfiles::default());
        let problem = problem.expect("the user is told");
        let start = i18n::with_error(
            Key::TplAgentsProfilesReadFailed,
            i18n::t(Key::ErrJsonPrefix),
        );
        assert!(problem.starts_with(&start), "{problem}");
        assert!(problem.len() > start.len());
    }

    #[test]
    fn saved_profiles_are_written_and_read_back() {
        let folder = TempDir::new("agents-file-write");
        let config_dir = folder.path().join("otty");
        let saved =
            SavedProfiles::default().with_added(otty_agents::Profile::added(
                otty_agents::AgentId::new("codex"),
                "Codex (Work)",
                PathBuf::from("/home/user/.codex-work"),
            ));

        let written = write_saved(&config_dir, &saved);
        let (read, problem) = read_saved(&config_dir);

        assert!(written.is_ok(), "{written:?}");
        assert_eq!(problem, None);
        assert_eq!(read, saved);
        assert!(!config_dir.join("agents.json.tmp").exists());
    }

    #[test]
    fn a_file_that_can_not_be_written_tells_why() {
        let folder = TempDir::new("agents-file-unwritable");
        std::fs::write(folder.path().join("otty"), "").expect("a file");

        let written =
            write_saved(&folder.path().join("otty"), &SavedProfiles::default());

        let error = written.expect_err("a file is in place of the folder");
        assert!(matches!(error, ProfilesFileError::Io(_)), "{error:?}");
        assert!(format!("{error}").starts_with(i18n::t(Key::ErrIoPrefix)));
    }

    #[test]
    fn a_file_that_can_not_be_read_tells_why() {
        let folder = TempDir::new("agents-file-folder");
        std::fs::create_dir(profiles_file(folder.path()))
            .expect("a folder in place of the file");

        let (saved, problem) = read_saved(folder.path());

        assert_eq!(saved, SavedProfiles::default());
        let problem = problem.expect("the user is told");
        let start = i18n::with_error(
            Key::TplAgentsProfilesReadFailed,
            i18n::t(Key::ErrIoPrefix),
        );
        assert!(problem.starts_with(&start), "{problem}");
    }
}
