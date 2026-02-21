use std::fs;
use std::path::{Path, PathBuf};

use super::errors::SettingsError;
use super::model::SettingsData;

/// Status describing how settings were loaded from disk.
#[derive(Debug, Clone)]
pub(crate) enum SettingsLoadStatus {
    Loaded,
    Missing,
    Invalid(String),
}

/// Result of loading settings from disk.
#[derive(Debug, Clone)]
pub(crate) struct SettingsLoad {
    pub(crate) settings: SettingsData,
    pub(crate) status: SettingsLoadStatus,
}

pub(crate) fn load_settings() -> Result<SettingsLoad, SettingsError> {
    let path = settings_path();
    let data = match fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(SettingsLoad {
                settings: SettingsData::default(),
                status: SettingsLoadStatus::Missing,
            });
        },
        Err(err) => return Err(err.into()),
    };

    let parsed = match serde_json::from_str::<serde_json::Value>(&data) {
        Ok(value) => value,
        Err(err) => {
            return Ok(SettingsLoad {
                settings: SettingsData::default(),
                status: SettingsLoadStatus::Invalid(format!("{err}")),
            });
        },
    };

    Ok(SettingsLoad {
        settings: SettingsData::from_json(&parsed),
        status: SettingsLoadStatus::Loaded,
    })
}

pub(crate) fn save_settings(
    settings: &SettingsData,
) -> Result<(), SettingsError> {
    let path = settings_path();
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }

    let payload = serde_json::to_string_pretty(settings)?;
    write_atomic(&path, payload.as_bytes())?;

    Ok(())
}

fn settings_path() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        return Path::new(&home)
            .join(".config")
            .join("otty")
            .join("settings.json");
    }

    std::env::temp_dir().join("otty").join("settings.json")
}

fn write_atomic(path: &Path, payload: &[u8]) -> Result<(), std::io::Error> {
    let tmp_path = path.with_extension("json.tmp");
    fs::write(&tmp_path, payload)?;
    fs::rename(tmp_path, path)?;
    Ok(())
}
