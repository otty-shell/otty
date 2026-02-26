use std::fs;
use std::path::{Path, PathBuf};

use super::errors::QuickLaunchError;
use super::model::QuickLaunchFile;

/// Load quick launch data from storage.
pub(crate) fn load_quick_launches()
-> Result<Option<QuickLaunchFile>, QuickLaunchError> {
    load_quick_launches_from(&quick_launches_path())
}

/// Save quick launch data to storage.
pub(crate) fn save_quick_launches(
    data: &QuickLaunchFile,
) -> Result<(), QuickLaunchError> {
    save_quick_launches_to(&quick_launches_path(), data)
}

fn load_quick_launches_from(
    path: &Path,
) -> Result<Option<QuickLaunchFile>, QuickLaunchError> {
    let data = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(None);
        },
        Err(err) => return Err(err.into()),
    };

    let parsed = serde_json::from_str(&data)?;
    Ok(Some(parsed))
}

fn save_quick_launches_to(
    path: &Path,
    data: &QuickLaunchFile,
) -> Result<(), QuickLaunchError> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }

    let payload = serde_json::to_string_pretty(data)?;
    write_atomic(path, payload.as_bytes())?;

    Ok(())
}

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

fn write_atomic(path: &Path, payload: &[u8]) -> Result<(), std::io::Error> {
    let tmp_path = path.with_extension("json.tmp");
    fs::write(&tmp_path, payload)?;
    fs::rename(tmp_path, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};
    use std::{fs, process};

    use super::super::model::QuickLaunchFile;
    use super::{load_quick_launches_from, save_quick_launches_to};

    struct TempDirGuard {
        path: PathBuf,
    }

    impl TempDirGuard {
        fn new() -> Self {
            let stamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "otty-quick-launches-storage-{}-{stamp}",
                process::id(),
            ));
            fs::create_dir_all(&path).expect("failed to create temporary dir");
            Self { path }
        }

        fn file_path(&self) -> PathBuf {
            self.path.join("quick_launches.json")
        }
    }

    impl Drop for TempDirGuard {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn given_saved_payload_when_loading_then_round_trip_succeeds() {
        let temp_dir = TempDirGuard::new();
        let path = temp_dir.file_path();
        let payload = QuickLaunchFile::empty();

        save_quick_launches_to(&path, &payload).expect("save should succeed");
        let loaded = load_quick_launches_from(&path).expect("load should work");

        assert!(loaded.is_some());
        let loaded = loaded.expect("payload should exist");
        assert_eq!(loaded.version, payload.version);
        assert_eq!(loaded.root.title, payload.root.title);
    }

    #[test]
    fn given_corrupted_json_when_loading_then_returns_json_error() {
        let temp_dir = TempDirGuard::new();
        let path = temp_dir.file_path();
        fs::write(&path, "{not valid json")
            .expect("failed to write corrupted payload");

        let result = load_quick_launches_from(&path);

        assert!(matches!(result, Err(super::QuickLaunchError::Json(_))));
    }
}
