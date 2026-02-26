use std::path::{Path, PathBuf};

use super::errors::QuickLaunchError;
use super::model::QuickLaunchFile;
use super::state::QuickLaunchState;

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

/// Load quick launches from disk.
pub(crate) fn load_quick_launches() -> Result<QuickLaunchFile, QuickLaunchError>
{
    let path = quick_launches_path();
    let content = match std::fs::read_to_string(&path) {
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

/// Load initial state from disk, falling back to defaults on error.
pub(crate) fn load_initial_quick_launch_state() -> QuickLaunchState {
    match load_quick_launches() {
        Ok(data) => QuickLaunchState::with_data(data),
        Err(err) => {
            log::warn!("Failed to load quick launches, using defaults: {err}");
            QuickLaunchState::default()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::quick_launch::model::{
        CommandSpec, CustomCommand, QuickLaunch, QuickLaunchFolder,
        QuickLaunchNode,
    };

    #[test]
    fn given_valid_json_when_deserialized_then_structure_is_correct() {
        let json = serde_json::json!({
            "version": 1,
            "root": {
                "title": "Root",
                "expanded": true,
                "children": [
                    {
                        "node_type": "Command",
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
                children: vec![QuickLaunchNode::Command(QuickLaunch {
                    title: String::from("Run"),
                    spec: CommandSpec::Custom {
                        custom: CustomCommand {
                            program: String::from("bash"),
                            args: Vec::new(),
                            env: Vec::new(),
                            working_directory: None,
                        },
                    },
                })],
            },
        };
        let json = serde_json::to_string(&data).expect("should serialize");
        let parsed: QuickLaunchFile =
            serde_json::from_str(&json).expect("should deserialize");
        assert_eq!(parsed.root.children.len(), 1);
    }
}
