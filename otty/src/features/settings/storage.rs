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
    settings: SettingsData,
    status: SettingsLoadStatus,
}

impl SettingsLoad {
    /// Build a settings load result from explicit parts.
    pub(crate) fn new(
        settings: SettingsData,
        status: SettingsLoadStatus,
    ) -> Self {
        Self { settings, status }
    }

    /// Consume the value and return both payload and status.
    pub(crate) fn into_parts(self) -> (SettingsData, SettingsLoadStatus) {
        (self.settings, self.status)
    }
}

pub(crate) fn load_settings() -> Result<SettingsLoad, SettingsError> {
    load_settings_from_path(&settings_path())
}

pub(crate) fn save_settings(
    settings: &SettingsData,
) -> Result<(), SettingsError> {
    save_settings_to_path(&settings_path(), settings)
}

fn load_settings_from_path(path: &Path) -> Result<SettingsLoad, SettingsError> {
    let data = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(SettingsLoad::new(
                SettingsData::default(),
                SettingsLoadStatus::Missing,
            ));
        },
        Err(err) => return Err(err.into()),
    };

    let parsed = match serde_json::from_str::<serde_json::Value>(&data) {
        Ok(value) => value,
        Err(err) => {
            return Ok(SettingsLoad::new(
                SettingsData::default(),
                SettingsLoadStatus::Invalid(format!("{err}")),
            ));
        },
    };

    Ok(SettingsLoad::new(
        SettingsData::from_json(&parsed),
        SettingsLoadStatus::Loaded,
    ))
}

fn save_settings_to_path(
    path: &Path,
    settings: &SettingsData,
) -> Result<(), SettingsError> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }

    let payload = serde_json::to_string_pretty(settings)?;
    write_atomic(path, payload.as_bytes())?;

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

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{
        SettingsData, SettingsLoadStatus, load_settings_from_path,
        save_settings_to_path,
    };

    #[test]
    fn given_valid_settings_when_save_and_load_then_round_trip_matches() {
        let root = test_temp_dir("round_trip");
        let path = root.join("settings.json");
        let mut settings = SettingsData::default();
        settings.set_terminal_shell(String::from("/bin/zsh"));
        settings.set_terminal_editor(String::from("vim"));
        settings.set_theme_palette_entry(0, String::from("#112233"));

        save_settings_to_path(&path, &settings)
            .expect("settings should save successfully");
        let loaded = load_settings_from_path(&path)
            .expect("settings should load successfully");
        let (loaded_settings, loaded_status) = loaded.into_parts();

        assert!(matches!(loaded_status, SettingsLoadStatus::Loaded));
        assert_eq!(loaded_settings, settings);

        fs::remove_dir_all(&root)
            .expect("temporary directory should be removed");
    }

    #[test]
    fn given_invalid_json_when_load_then_returns_default_with_invalid_status() {
        let root = test_temp_dir("invalid_json");
        let path = root.join("settings.json");
        fs::write(&path, "{ this is not valid json")
            .expect("invalid test payload should be written");

        let loaded = load_settings_from_path(&path)
            .expect("loading invalid settings should not fail with io error");
        let (loaded_settings, loaded_status) = loaded.into_parts();

        assert_eq!(loaded_settings, SettingsData::default());
        match loaded_status {
            SettingsLoadStatus::Invalid(message) => {
                assert!(!message.is_empty());
            },
            other => panic!("expected invalid status, got {other:?}"),
        }

        fs::remove_dir_all(&root)
            .expect("temporary directory should be removed");
    }

    fn test_temp_dir(test_name: &str) -> std::path::PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be monotonic")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "otty-settings-{test_name}-{stamp}-{}",
            std::process::id()
        ));

        fs::create_dir_all(&dir)
            .expect("temporary directory should be created");
        dir
    }
}
