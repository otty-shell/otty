use std::fs;
use std::path::{Path, PathBuf};

use thiserror::Error;

use super::model::QuickCommandsFile;

/// Errors emitted while reading or writing quick command storage.
#[derive(Debug, Error)]
pub(crate) enum QuickCommandsError {
    #[error("quick commands IO failed")]
    Io(#[from] std::io::Error),
    #[error("quick commands JSON failed")]
    Json(#[from] serde_json::Error),
}

pub(crate) fn load_quick_commands()
-> Result<Option<QuickCommandsFile>, QuickCommandsError> {
    let path = quick_commands_path();
    let data = match fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(None);
        },
        Err(err) => return Err(err.into()),
    };

    let parsed = serde_json::from_str(&data)?;
    Ok(Some(parsed))
}

pub(crate) fn save_quick_commands(
    data: &QuickCommandsFile,
) -> Result<(), QuickCommandsError> {
    let path = quick_commands_path();
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }

    let payload = serde_json::to_string_pretty(data)?;
    write_atomic(&path, payload.as_bytes())?;

    Ok(())
}

fn quick_commands_path() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        return Path::new(&home)
            .join(".config")
            .join("otty")
            .join("quick_commands.json");
    }

    std::env::temp_dir()
        .join("otty")
        .join("quick_commands.json")
}

fn write_atomic(path: &Path, payload: &[u8]) -> Result<(), std::io::Error> {
    let tmp_path = path.with_extension("json.tmp");
    fs::write(&tmp_path, payload)?;
    fs::rename(tmp_path, path)?;
    Ok(())
}
