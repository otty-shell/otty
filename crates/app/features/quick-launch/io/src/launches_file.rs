//! The launches file, `quick_launches.json` in the app's config folder
//! (`~/.config/otty`): read once at start and written when the tree
//! changes. Each finished write comes back as a quick launch intent. The
//! binary resolves the config folder once and gives it to every call.

use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use iced::Task;
use otty_app_i18n::{self as i18n, Key};
use otty_app_quick_launch::{QuickLaunchFile, QuickLaunchIntent};
use thiserror::Error;

/// Errors of reading or writing the launches file.
#[derive(Debug, Error)]
enum LaunchesFileError {
    #[error("{}{}", i18n::t(Key::ErrIoPrefix), .0)]
    Io(#[from] std::io::Error),
    #[error("{}{}", i18n::t(Key::ErrJsonPrefix), .0)]
    Json(#[from] serde_json::Error),
}

/// Read the launches in `config_dir` at start. A missing file is the first
/// start and gives an empty tree; a file that can not be read gives an
/// empty tree too, and the text that tells the user (the next change writes
/// over the file).
///
/// # Examples
///
/// ```no_run
/// use std::path::Path;
///
/// let (launches, problem) = otty_app_quick_launch_io::load_at_start(
///     Path::new("/home/user/.config/otty"),
/// );
///
/// assert!(problem.is_none() || launches.launch_count() == 0);
/// ```
pub fn load_at_start(config_dir: &Path) -> (QuickLaunchFile, Option<String>) {
    load_at_start_from(&launches_file(config_dir))
}

/// Write the launches to the launches file in `config_dir` in the
/// background; the answer comes back as
/// [`QuickLaunchIntent::PersistCompleted`] or
/// [`QuickLaunchIntent::PersistFailed`].
///
/// # Examples
///
/// ```
/// use std::path::Path;
///
/// use otty_app_quick_launch::QuickLaunchFile;
///
/// let save = otty_app_quick_launch_io::save(
///     Path::new("/home/user/.config/otty"),
///     QuickLaunchFile::default(),
/// );
///
/// assert_eq!(save.units(), 1);
/// ```
pub fn save(
    config_dir: &Path,
    data: QuickLaunchFile,
) -> Task<QuickLaunchIntent> {
    let path = launches_file(config_dir);

    Task::perform(
        async move { save_to(&path, &data).map_err(|err| format!("{err}")) },
        save_finished,
    )
}

fn save_finished(result: Result<(), String>) -> QuickLaunchIntent {
    match result {
        Ok(()) => QuickLaunchIntent::PersistCompleted,
        Err(message) => QuickLaunchIntent::PersistFailed(message),
    }
}

/// Read the launches at start from `path`, as [`load_at_start`] tells.
fn load_at_start_from(path: &Path) -> (QuickLaunchFile, Option<String>) {
    let error = match load_from(path) {
        Ok(data) => return (data, None),
        Err(err) => format!("{err}"),
    };

    log::warn!("Failed to load quick launches, using defaults: {error}");
    let problem = i18n::with_error(Key::TplLaunchesReadFailed, &error);

    (QuickLaunchFile::default(), Some(problem))
}

/// Read the launches from the file at `path`; a missing file gives an
/// empty tree.
fn load_from(path: &Path) -> Result<QuickLaunchFile, LaunchesFileError> {
    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(err) if err.kind() == ErrorKind::NotFound => {
            return Ok(QuickLaunchFile::default());
        },
        Err(err) => return Err(err.into()),
    };

    Ok(serde_json::from_str(&content)?)
}

/// Write the launches to `path`: next to it first, then moved there, so a
/// failed write never leaves half a file.
fn save_to(
    path: &Path,
    data: &QuickLaunchFile,
) -> Result<(), LaunchesFileError> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }

    let payload = serde_json::to_string_pretty(data)?;
    let tmp_path = path.with_extension("json.tmp");
    std::fs::write(&tmp_path, payload.as_bytes())?;
    std::fs::rename(tmp_path, path)?;

    Ok(())
}

/// Return the path of the launches file in `config_dir`.
fn launches_file(config_dir: &Path) -> PathBuf {
    config_dir.join("quick_launches.json")
}

#[cfg(test)]
mod tests {
    use std::fs;

    use otty_app_testing::temp_dir::TempDir;

    use super::*;

    /// A missing file is the first start: an empty tree, nothing to tell.
    #[test]
    fn at_start_a_missing_file_gives_an_empty_tree_and_tells_nothing() {
        let root = TempDir::new("launches-start-missing");

        let (data, problem) =
            load_at_start_from(&root.path().join("quick_launches.json"));

        assert!(data.launch_count() == 0);
        assert!(problem.is_none());
    }

    #[test]
    fn at_start_a_read_file_gives_its_launches_and_tells_nothing() {
        let root = TempDir::new("launches-start-read");
        let path = root.path().join("quick_launches.json");
        let saved = serde_json::json!({
            "version": 1,
            "root": {
                "title": "Root",
                "expanded": true,
                "children": [{
                    "kind": "Command",
                    "title": "Run",
                    "spec": {
                        "type": "Custom",
                        "custom": {
                            "program": "bash",
                            "args": [],
                            "env": [],
                            "working_directory": null
                        }
                    }
                }]
            }
        });
        fs::write(&path, saved.to_string())
            .expect("launches file should be written");

        let (data, problem) = load_at_start_from(&path);

        assert_eq!(data.launch_count(), 1);
        assert!(problem.is_none());
    }

    /// A broken file starts with an empty tree, as before, and the user is
    /// told: the next edit writes over the file.
    #[test]
    fn at_start_a_broken_file_gives_an_empty_tree_and_a_problem() {
        let root = TempDir::new("launches-start-broken");
        let path = root.path().join("quick_launches.json");
        fs::write(&path, "{ \"root\": [")
            .expect("launches file should be written");

        let (data, problem) = load_at_start_from(&path);

        let error =
            load_from(&path).expect_err("a broken file should not load");
        let expected =
            i18n::with_error(Key::TplLaunchesReadFailed, &format!("{error}"));
        assert!(data.launch_count() == 0);
        assert_eq!(problem.as_deref(), Some(expected.as_str()));
    }

    #[test]
    fn a_finished_save_reaches_the_feature() {
        assert!(matches!(
            save_finished(Ok(())),
            QuickLaunchIntent::PersistCompleted
        ));
    }

    #[test]
    fn a_failed_save_reaches_the_feature_with_its_reason() {
        assert!(matches!(
            save_finished(Err(String::from("disk full"))),
            QuickLaunchIntent::PersistFailed(message) if message == "disk full"
        ));
    }

    /// The launches are written to the file and read back.
    #[test]
    fn saved_launches_are_read_back() {
        let root = TempDir::new("launches-save");
        let path = root.path().join("otty").join("quick_launches.json");
        let saved: QuickLaunchFile =
            serde_json::from_value(serde_json::json!({
                "version": 1,
                "root": {"title": "Root", "expanded": true, "children": [{
                    "kind": "Command", "title": "Run",
                    "spec": {"type": "Custom", "custom": {"program": "bash",
                        "args": [], "env": [], "working_directory": null}}
                }]}
            }))
            .expect("the launches should parse");

        save_to(&path, &saved).expect("the launches should be written");
        let read = load_from(&path).expect("the launches should be read");

        assert_eq!(read.launch_count(), 1);
        assert!(!path.with_extension("json.tmp").exists());
    }
}
