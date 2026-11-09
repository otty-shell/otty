use std::path::{Path, PathBuf};

use super::errors::QuickLaunchError;
use super::state::QuickLaunchState;
use super::types::QuickLaunchFile;
use crate::i18n::{self, Key};

/// Load quick launches from the file at `path`; a missing file gives an
/// empty tree.
fn load_quick_launches_from(
    path: &Path,
) -> Result<QuickLaunchFile, QuickLaunchError> {
    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(QuickLaunchFile::default());
        },
        Err(err) => return Err(err.into()),
    };
    let data: QuickLaunchFile = serde_json::from_str(&content)?;
    Ok(data)
}

/// Save quick launches to disk atomically.
pub(crate) fn save_quick_launches(
    data: &QuickLaunchFile,
) -> Result<(), QuickLaunchError> {
    let path = quick_launches_path();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let payload = serde_json::to_string_pretty(data)?;
    let tmp_path = path.with_extension("json.tmp");
    std::fs::write(&tmp_path, payload.as_bytes())?;
    std::fs::rename(tmp_path, path)?;
    Ok(())
}

/// Load the state at start from `~/.config/otty/quick_launches.json`, with
/// the text to tell the user when the file could not be read.
pub(super) fn load_initial_quick_launch_state()
-> (QuickLaunchState, Option<String>) {
    initial_state_from(&quick_launches_path())
}

/// Load the state at start from `path`. A file that cannot be read gives an
/// empty tree and the text that tells the user; a missing file is the first
/// start and tells nothing.
fn initial_state_from(path: &Path) -> (QuickLaunchState, Option<String>) {
    let error = match load_quick_launches_from(path) {
        Ok(data) => return (QuickLaunchState::with_data(data), None),
        Err(err) => format!("{err}"),
    };

    log::warn!("Failed to load quick launches, using defaults: {error}");
    let problem = i18n::with_error(Key::TplLaunchesReadFailed, &error);

    (QuickLaunchState::default(), Some(problem))
}

/// Return the path to the quick launches JSON file.
fn quick_launches_path() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        return Path::new(&home)
            .join(".config")
            .join("otty")
            .join("quick_launches.json");
    }
    std::env::temp_dir()
        .join("otty")
        .join("quick_launches.json")
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::domain::exit_behavior::ExitBehavior;
    use crate::domain::quick_launch::{
        CommandSpec, CustomCommand, QuickLaunch,
    };
    use crate::testing::temp_dir::TempDir;
    use crate::widgets::quick_launch::types::{
        QuickLaunchFolder, QuickLaunchNode,
    };

    /// A missing file is the first start: an empty tree, nothing to tell.
    #[test]
    fn at_start_a_missing_file_gives_an_empty_tree_and_tells_nothing() {
        let root = TempDir::new("launches-start-missing");

        let (state, problem) =
            initial_state_from(&root.path().join("quick_launches.json"));

        assert!(state.data().root.children.is_empty());
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

        let (state, problem) = initial_state_from(&path);

        assert_eq!(state.data().root.children.len(), 1);
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

        let (state, problem) = initial_state_from(&path);

        let error = load_quick_launches_from(&path)
            .expect_err("a broken file should not load");
        let expected =
            i18n::with_error(Key::TplLaunchesReadFailed, &format!("{error}"));
        assert!(state.data().root.children.is_empty());
        assert_eq!(problem.as_deref(), Some(expected.as_str()));
    }

    #[test]
    fn given_valid_json_when_deserialized_then_structure_is_correct() {
        let json = serde_json::json!({
            "version": 1,
            "root": {
                "title": "Root",
                "expanded": true,
                "children": [
                    {
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
                    }
                ]
            }
        });

        let data: QuickLaunchFile =
            serde_json::from_value(json).expect("should deserialize");
        assert_eq!(data.root.children.len(), 1);
        assert_eq!(data.root.children[0].title(), "Run");
    }

    #[test]
    fn given_quick_launch_file_when_serialized_then_round_trips() {
        let data = QuickLaunchFile {
            version: 1,
            root: QuickLaunchFolder {
                title: String::from("Root"),
                expanded: true,
                children: vec![QuickLaunchNode::Command(QuickLaunch::new(
                    String::from("Run"),
                    CommandSpec::Custom {
                        custom: CustomCommand::new(
                            String::from("bash"),
                            Vec::new(),
                            Vec::new(),
                            None,
                        ),
                    },
                    ExitBehavior::Close,
                ))],
            },
        };
        let json = serde_json::to_string(&data).expect("should serialize");
        let parsed: QuickLaunchFile =
            serde_json::from_str(&json).expect("should deserialize");
        assert_eq!(parsed.root.children.len(), 1);
    }
}
