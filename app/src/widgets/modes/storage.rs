use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use super::errors::ModesError;
use super::state::ModesState;
use crate::domain::mode::Mode;
use crate::i18n::{self, Key};

/// Load the modes state at start: Terminal active, the pinned modes read
/// from `~/.config/otty/modes.json`, with the text to tell the user when the
/// file could not be read.
pub(super) fn load_initial_modes_state() -> (ModesState, Option<String>) {
    load_state_from_path(&modes_path())
}

/// Save `pinned` to `~/.config/otty/modes.json` atomically.
pub(crate) fn save_pinned(pinned: &[Mode]) -> Result<(), ModesError> {
    save_pinned_to_path(&modes_path(), pinned)
}

/// Read the pinned modes at `path`. A missing file is the first start: the
/// defaults, and nothing to tell. A file that cannot be read gives the
/// defaults and the text that tells the user. Unknown names are skipped and
/// the rest is kept; the pin rules leave out Terminal, repeats and entries
/// past the limit; both are only logged.
fn load_state_from_path(path: &Path) -> (ModesState, Option<String>) {
    let names = match read_pinned_names(path) {
        Ok(names) => names,
        Err(ModesError::Io(err)) if err.kind() == ErrorKind::NotFound => {
            return (ModesState::default(), None);
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
            return (ModesState::default(), Some(problem));
        },
    };

    let modes: Vec<Mode> = names
        .iter()
        .filter_map(|name| {
            let mode = Mode::from_storage_name(name);
            if mode.is_none() {
                log::warn!("{}: unknown mode {name:?} skipped", path.display());
            }
            mode
        })
        .collect();
    let state = ModesState::with_pinned(&modes);

    if state.pinned().len() < modes.len() {
        log::warn!(
            "{}: Terminal, repeated modes and modes past the pin limit \
             skipped",
            path.display()
        );
    }

    (state, None)
}

/// The names of the file's `pinned` list, in file order.
fn read_pinned_names(path: &Path) -> Result<Vec<String>, ModesError> {
    let contents = fs::read_to_string(path)?;
    let value: serde_json::Value = serde_json::from_str(&contents)?;

    let Some(entries) = value.get("pinned").and_then(|list| list.as_array())
    else {
        return Err(ModesError::NoPinnedList);
    };

    entries
        .iter()
        .map(|entry| {
            entry
                .as_str()
                .map(str::to_owned)
                .ok_or(ModesError::NoPinnedList)
        })
        .collect()
}

fn save_pinned_to_path(path: &Path, pinned: &[Mode]) -> Result<(), ModesError> {
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
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    /// A temporary folder with a `modes.json` path in it, removed on drop.
    struct TempModesFile {
        dir: PathBuf,
    }

    impl TempModesFile {
        fn path(&self) -> PathBuf {
            self.dir.join("otty").join("modes.json")
        }

        fn new(test_name: &str) -> Self {
            let stamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock should be after the epoch")
                .as_nanos();
            let dir = std::env::temp_dir().join(format!(
                "otty-modes-{test_name}-{stamp}-{}",
                std::process::id()
            ));

            Self { dir }
        }

        /// Write `contents` as the modes file.
        fn with_contents(test_name: &str, contents: &str) -> Self {
            let file = Self::new(test_name);
            let path = file.path();
            fs::create_dir_all(path.parent().expect("path has a parent"))
                .expect("temporary directory should be created");
            fs::write(&path, contents).expect("modes file should be written");

            file
        }
    }

    impl Drop for TempModesFile {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }

    fn loaded_pinned(contents: &str, test_name: &str) -> Vec<Mode> {
        let file = TempModesFile::with_contents(test_name, contents);

        let (state, _problem) = load_state_from_path(&file.path());

        state.pinned().to_vec()
    }

    #[test]
    fn a_missing_file_gives_containers_pinned_and_terminal_active() {
        let file = TempModesFile::new("missing");

        let (state, _problem) = load_state_from_path(&file.path());

        assert_eq!(state.pinned(), &[Mode::Containers]);
        assert_eq!(state.active(), Mode::Terminal);
        assert!(!state.is_menu_open());
    }

    /// A missing file is the first start: nothing to tell.
    #[test]
    fn a_missing_file_tells_nothing() {
        let file = TempModesFile::new("missing-silent");

        let (_state, problem) = load_state_from_path(&file.path());

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
            let file = TempModesFile::with_contents(
                &format!("broken-told-{index}"),
                contents,
            );

            let (state, problem) = load_state_from_path(&file.path());

            let error = read_pinned_names(&file.path())
                .expect_err("a broken file should not read");
            let expected = i18n::with_error(
                Key::TplPinnedModesReadFailed,
                &format!("{error}"),
            );
            assert_eq!(state.pinned(), &[Mode::Containers], "{contents}");
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
        let file = TempModesFile::new("unreadable");
        fs::create_dir_all(file.path()).expect("folder should be created");

        let (state, problem) = load_state_from_path(&file.path());

        assert_eq!(state.pinned(), &[Mode::Containers]);
        assert!(problem.is_some_and(|text| text.contains("modes file IO")));
    }

    /// Unknown names may come from a newer version: the rest is kept and
    /// only the log says so.
    #[test]
    fn skipped_names_tell_nothing() {
        let file = TempModesFile::with_contents(
            "skipped-silent",
            "{\"pinned\": [\"kubernetes\", \"containers\"]}",
        );

        let (state, problem) = load_state_from_path(&file.path());

        assert_eq!(state.pinned(), &[Mode::Containers]);
        assert!(problem.is_none());
    }

    #[test]
    fn broken_json_gives_the_defaults() {
        let pinned = loaded_pinned("{ \"pinned\": [", "broken");

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
            let pinned = loaded_pinned(contents, &format!("shape-{index}"));

            assert_eq!(pinned, vec![Mode::Containers], "{contents}");
        }
    }

    #[test]
    fn an_empty_list_keeps_nothing_pinned() {
        let pinned = loaded_pinned("{\"pinned\": []}", "empty");

        assert!(pinned.is_empty());
    }

    #[test]
    fn unknown_names_are_skipped_and_the_rest_is_kept() {
        let pinned = loaded_pinned(
            "{\"pinned\": [\"kubernetes\", \"containers\", \"agents\"]}",
            "unknown",
        );

        assert_eq!(pinned, vec![Mode::Containers]);
    }

    #[test]
    fn only_unknown_names_leave_nothing_pinned() {
        let pinned = loaded_pinned("{\"pinned\": [\"kubernetes\"]}", "only");

        assert!(pinned.is_empty());
    }

    #[test]
    fn terminal_and_repeated_entries_are_skipped() {
        let pinned = loaded_pinned(
            "{\"pinned\": [\"terminal\", \"containers\", \"containers\"]}",
            "repeats",
        );

        assert_eq!(pinned, vec![Mode::Containers]);
    }

    #[test]
    fn more_than_four_entries_keep_at_most_four_pinned() {
        let pinned = loaded_pinned(
            "{\"pinned\": [\"containers\", \"kubernetes\", \"agents\", \
             \"postgres\", \"aws\", \"containers\"]}",
            "many",
        );

        assert_eq!(pinned, vec![Mode::Containers]);
    }

    #[test]
    fn a_file_saved_with_the_old_docker_name_keeps_containers_pinned() {
        let pinned = loaded_pinned("{\"pinned\": [\"docker\"]}", "old-name");

        assert_eq!(pinned, vec![Mode::Containers]);
    }

    #[test]
    fn the_old_and_the_new_name_together_pin_containers_once() {
        let pinned = loaded_pinned(
            "{\"pinned\": [\"docker\", \"containers\"]}",
            "both-names",
        );

        assert_eq!(pinned, vec![Mode::Containers]);
    }

    #[test]
    fn saved_pinned_modes_load_back() {
        for pinned in [vec![Mode::Containers], Vec::new()] {
            let file = TempModesFile::new("round-trip");

            save_pinned_to_path(&file.path(), &pinned)
                .expect("modes file should save");
            let (state, _problem) = load_state_from_path(&file.path());

            assert_eq!(state.pinned(), pinned.as_slice());
        }
    }

    #[test]
    fn the_file_lists_pinned_modes_by_name() {
        let file = TempModesFile::new("format");

        save_pinned_to_path(&file.path(), &[Mode::Containers])
            .expect("modes file should save");
        let saved: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(file.path()).expect("modes file should read"),
        )
        .expect("modes file should be JSON");

        assert_eq!(saved, serde_json::json!({ "pinned": ["containers"] }));
        assert!(!file.path().with_extension("json.tmp").exists());
    }
}
