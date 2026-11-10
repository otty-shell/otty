//! The pinned modes in `~/.config/otty/modes.json`: read once at start and
//! written in the background after each pin or unpin.

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use iced::Task;
use otty_app_domain::mode::Mode;
use otty_app_i18n::{self as i18n, Key};
use otty_app_modes::{DEFAULT_PINNED, ModesIntent};
use thiserror::Error;

/// Errors emitted while reading or writing `modes.json`.
#[derive(Debug, Error)]
enum ModesFileError {
    /// Filesystem operation failed.
    #[error("modes file IO failed: {0}")]
    Io(#[from] std::io::Error),
    /// JSON parsing or serialization failed.
    #[error("modes file JSON failed: {0}")]
    Json(#[from] serde_json::Error),
    /// The JSON has no `pinned` list of mode names.
    #[error("modes file has no \"pinned\" list of mode names")]
    NoPinnedList,
}

/// Read the pinned modes at start, in file order, with the text that tells
/// the user when the file could not be read. The pin rules are applied by
/// the modes feature.
///
/// # Examples
///
/// ```no_run
/// let (pinned, problem) = otty_app_modes_io::load_at_start();
///
/// assert!(problem.is_none() || pinned == otty_app_modes::DEFAULT_PINNED);
/// ```
pub fn load_at_start() -> (Vec<Mode>, Option<String>) {
    load_from_path(&modes_path())
}

/// Write `pinned` to `modes.json` in the background. The answer comes back
/// as [`ModesIntent::PinnedSaved`]; a failed write is logged here.
///
/// # Examples
///
/// ```
/// use otty_app_domain::mode::Mode;
///
/// let save = otty_app_modes_io::save(vec![Mode::Containers]);
///
/// assert_eq!(save.units(), 1);
/// ```
pub fn save(pinned: Vec<Mode>) -> Task<ModesIntent> {
    Task::perform(
        async move {
            save_to_path(&modes_path(), &pinned).map_err(|err| {
                log::warn!("saving the pinned modes failed: {err}");
                format!("{err}")
            })
        },
        ModesIntent::PinnedSaved,
    )
}

/// Read the pinned modes at `path`. A missing file is the first start: the
/// defaults, and nothing to tell. A file that cannot be read gives the
/// defaults and the text that tells the user. Unknown names are skipped and
/// the rest is kept; that is only logged.
fn load_from_path(path: &Path) -> (Vec<Mode>, Option<String>) {
    let names = match read_pinned_names(path) {
        Ok(names) => names,
        Err(ModesFileError::Io(err)) if err.kind() == ErrorKind::NotFound => {
            return (DEFAULT_PINNED.to_vec(), None);
        },
        Err(err) => {
            log::warn!(
                "{}: {err}; using the default pinned modes",
                path.display()
            );
            let problem = i18n::with_error(
                Key::TplPinnedModesReadFailed,
                &format!("{err}"),
            );
            return (DEFAULT_PINNED.to_vec(), Some(problem));
        },
    };

    let pinned = names
        .iter()
        .filter_map(|name| {
            let mode = Mode::from_storage_name(name);
            if mode.is_none() {
                log::warn!("{}: unknown mode {name:?} skipped", path.display());
            }
            mode
        })
        .collect();

    (pinned, None)
}

/// The names of the file's `pinned` list, in file order.
fn read_pinned_names(path: &Path) -> Result<Vec<String>, ModesFileError> {
    let contents = fs::read_to_string(path)?;
    let value: serde_json::Value = serde_json::from_str(&contents)?;

    let Some(entries) = value.get("pinned").and_then(|list| list.as_array())
    else {
        return Err(ModesFileError::NoPinnedList);
    };

    entries
        .iter()
        .map(|entry| {
            entry
                .as_str()
                .map(str::to_owned)
                .ok_or(ModesFileError::NoPinnedList)
        })
        .collect()
}

fn save_to_path(path: &Path, pinned: &[Mode]) -> Result<(), ModesFileError> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }

    let names: Vec<&str> =
        pinned.iter().map(|mode| mode.storage_name()).collect();
    let payload =
        serde_json::to_string_pretty(&serde_json::json!({ "pinned": names }))?;
    write_atomic(path, payload.as_bytes())?;

    Ok(())
}

fn modes_path() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        return Path::new(&home)
            .join(".config")
            .join("otty")
            .join("modes.json");
    }

    std::env::temp_dir().join("otty").join("modes.json")
}

/// Write to a temporary file next to `path`, then rename it over `path`,
/// so a crash never leaves a half-written file.
fn write_atomic(path: &Path, payload: &[u8]) -> Result<(), std::io::Error> {
    let tmp_path = path.with_extension("json.tmp");
    fs::write(&tmp_path, payload)?;
    fs::rename(tmp_path, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use otty_app_modes::Modes;
    use otty_app_testing::temp_dir::TempDir;

    use super::*;

    /// The `modes.json` path in a folder of its own.
    fn modes_file(folder: &TempDir) -> PathBuf {
        folder.path().join("otty").join("modes.json")
    }

    /// A folder with `contents` written as its modes file.
    fn folder_with(test_name: &str, contents: &str) -> TempDir {
        let folder = TempDir::new(test_name);
        let path = modes_file(&folder);
        fs::create_dir_all(path.parent().expect("path has a parent"))
            .expect("temporary directory should be created");
        fs::write(&path, contents).expect("modes file should be written");

        folder
    }

    /// The modes the switcher pins after reading `contents` at start.
    fn pinned_at_start(contents: &str, test_name: &str) -> Vec<Mode> {
        let folder = folder_with(test_name, contents);

        let (pinned, _problem) = load_from_path(&modes_file(&folder));

        Modes::with_pinned(&pinned).pinned().to_vec()
    }

    #[test]
    fn a_missing_file_gives_containers_pinned() {
        let folder = TempDir::new("modes-missing");

        let (pinned, _problem) = load_from_path(&modes_file(&folder));

        assert_eq!(pinned, vec![Mode::Containers]);
    }

    /// A missing file is the first start: nothing to tell.
    #[test]
    fn a_missing_file_tells_nothing() {
        let folder = TempDir::new("modes-missing-silent");

        let (_pinned, problem) = load_from_path(&modes_file(&folder));

        assert!(problem.is_none());
    }

    /// A broken file gives the default pinned modes, as before, and the
    /// user is told: the next pin or unpin writes over the file.
    #[test]
    fn a_broken_file_gives_the_defaults_and_a_problem() {
        for (index, contents) in ["{ \"pinned\": [", "{}", "{\"pinned\": [1]}"]
            .into_iter()
            .enumerate()
        {
            let folder =
                folder_with(&format!("modes-broken-told-{index}"), contents);
            let path = modes_file(&folder);

            let (pinned, problem) = load_from_path(&path);

            let error = read_pinned_names(&path)
                .expect_err("a broken file should not read");
            let expected = i18n::with_error(
                Key::TplPinnedModesReadFailed,
                &format!("{error}"),
            );
            assert_eq!(pinned, vec![Mode::Containers], "{contents}");
            assert_eq!(
                problem.as_deref(),
                Some(expected.as_str()),
                "{contents}"
            );
        }
    }

    /// A file that cannot be read, here a folder in its place, gives the
    /// defaults and a problem.
    #[test]
    fn an_unreadable_file_gives_the_defaults_and_a_problem() {
        let folder = TempDir::new("modes-unreadable");
        let path = modes_file(&folder);
        fs::create_dir_all(&path).expect("folder should be created");

        let (pinned, problem) = load_from_path(&path);

        assert_eq!(pinned, vec![Mode::Containers]);
        assert!(problem.is_some_and(|text| text.contains("modes file IO")));
    }

    /// Unknown names may come from a newer version: the rest is kept and
    /// only the log says so.
    #[test]
    fn skipped_names_tell_nothing() {
        let folder = folder_with(
            "modes-skipped-silent",
            "{\"pinned\": [\"kubernetes\", \"containers\"]}",
        );

        let (pinned, problem) = load_from_path(&modes_file(&folder));

        assert_eq!(pinned, vec![Mode::Containers]);
        assert!(problem.is_none());
    }

    #[test]
    fn broken_json_gives_the_defaults() {
        let pinned = pinned_at_start("{ \"pinned\": [", "modes-broken");

        assert_eq!(pinned, vec![Mode::Containers]);
    }

    #[test]
    fn a_file_without_a_pinned_list_gives_the_defaults() {
        for (index, contents) in [
            "[]",
            "{}",
            "{\"pinned\": \"containers\"}",
            "{\"pinned\": [1]}",
        ]
        .into_iter()
        .enumerate()
        {
            let pinned =
                pinned_at_start(contents, &format!("modes-shape-{index}"));

            assert_eq!(pinned, vec![Mode::Containers], "{contents}");
        }
    }

    #[test]
    fn an_empty_list_keeps_nothing_pinned() {
        let pinned = pinned_at_start("{\"pinned\": []}", "modes-empty");

        assert!(pinned.is_empty());
    }

    #[test]
    fn unknown_names_are_skipped_and_the_rest_is_kept() {
        let pinned = pinned_at_start(
            "{\"pinned\": [\"kubernetes\", \"containers\", \"agents\"]}",
            "modes-unknown",
        );

        assert_eq!(pinned, vec![Mode::Containers]);
    }

    #[test]
    fn only_unknown_names_leave_nothing_pinned() {
        let pinned =
            pinned_at_start("{\"pinned\": [\"kubernetes\"]}", "modes-only");

        assert!(pinned.is_empty());
    }

    #[test]
    fn terminal_and_repeated_entries_are_skipped() {
        let pinned = pinned_at_start(
            "{\"pinned\": [\"terminal\", \"containers\", \"containers\"]}",
            "modes-repeats",
        );

        assert_eq!(pinned, vec![Mode::Containers]);
    }

    #[test]
    fn more_than_four_entries_keep_at_most_four_pinned() {
        let pinned = pinned_at_start(
            "{\"pinned\": [\"containers\", \"kubernetes\", \"agents\", \
             \"postgres\", \"aws\", \"containers\"]}",
            "modes-many",
        );

        assert_eq!(pinned, vec![Mode::Containers]);
    }

    #[test]
    fn a_file_saved_with_the_old_docker_name_keeps_containers_pinned() {
        let pinned =
            pinned_at_start("{\"pinned\": [\"docker\"]}", "modes-old-name");

        assert_eq!(pinned, vec![Mode::Containers]);
    }

    #[test]
    fn the_old_and_the_new_name_together_pin_containers_once() {
        let pinned = pinned_at_start(
            "{\"pinned\": [\"docker\", \"containers\"]}",
            "modes-both-names",
        );

        assert_eq!(pinned, vec![Mode::Containers]);
    }

    #[test]
    fn saved_pinned_modes_load_back() {
        for pinned in [vec![Mode::Containers], Vec::new()] {
            let folder = TempDir::new("modes-round-trip");
            let path = modes_file(&folder);

            save_to_path(&path, &pinned).expect("modes file should save");
            let (loaded, _problem) = load_from_path(&path);

            assert_eq!(loaded, pinned);
        }
    }

    #[test]
    fn the_file_lists_pinned_modes_by_name() {
        let folder = TempDir::new("modes-format");
        let path = modes_file(&folder);

        save_to_path(&path, &[Mode::Containers])
            .expect("modes file should save");
        let saved: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(&path).expect("modes file should read"),
        )
        .expect("modes file should be JSON");

        assert_eq!(saved, serde_json::json!({ "pinned": ["containers"] }));
        assert!(!path.with_extension("json.tmp").exists());
    }
}
