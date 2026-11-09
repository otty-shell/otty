use std::fs;
use std::path::{Path, PathBuf};

use super::errors::SettingsError;
use super::state::SettingsState;
use super::types::{SettingsData, SettingsLoad, SettingsLoadStatus};
use crate::i18n::{self, Key};

/// Load settings from disk.
pub(crate) fn load_settings() -> Result<SettingsLoad, SettingsError> {
    load_settings_from_path(&settings_path())
}

/// Save settings to disk.
pub(crate) fn save_settings(
    settings: &SettingsData,
) -> Result<(), SettingsError> {
    save_settings_to_path(&settings_path(), settings)
}

/// Load the settings state at start from `~/.config/otty/settings.json`,
/// with the text to tell the user when the file could not be used.
pub(super) fn load_initial_settings_state() -> (SettingsState, Option<String>) {
    initial_settings_from(&settings_path())
}

/// Load the settings state at start from `path`. A file that cannot be read
/// or is broken gives the defaults and the text that tells the user; a
/// missing file is the first start and tells nothing.
fn initial_settings_from(path: &Path) -> (SettingsState, Option<String>) {
    let (data, problem) = match load_settings_from_path(path) {
        Ok(load) => match load.into_parts() {
            (data, SettingsLoadStatus::Invalid(message)) => {
                log::warn!("settings file invalid: {message}");
                (data, Some((Key::TplSettingsFileInvalid, message)))
            },
            (data, _) => (data, None),
        },
        Err(err) => {
            log::warn!("settings read failed: {err}");
            let problem = (Key::TplSettingsReadFailed, format!("{err}"));
            (SettingsData::default(), Some(problem))
        },
    };

    // The locale must be active before the state builds its tree, whose paths
    // are made of localized section titles, and before the problem is told
    // in it.
    i18n::set_locale(data.language().resolve());

    let problem = problem.map(|(key, error)| i18n::with_error(key, &error));

    (SettingsState::from_settings(data), problem)
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
    // Loading settings sets the process-wide language, so tests that build the
    // widget must not depend on the language of the user running them.
    if cfg!(test) {
        return std::env::temp_dir()
            .join("otty-tests")
            .join("settings.json");
    }

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

    use super::{
        SettingsData, SettingsLoadStatus, initial_settings_from,
        load_settings_from_path, save_settings_to_path, settings_path,
    };
    use crate::i18n::{self, Key, Locale};
    use crate::testing::temp_dir::TempDir;
    use crate::theme::BuiltinTheme;
    use crate::widgets::settings::errors::SettingsError;
    use crate::widgets::settings::types::LanguageSetting;

    /// A missing file is the first start: the defaults, nothing to tell.
    #[test]
    fn at_start_a_missing_file_gives_the_defaults_and_tells_nothing() {
        let root = TempDir::new("settings-start-missing");

        let (state, problem) =
            initial_settings_from(&root.path().join("settings.json"));

        assert_eq!(state.baseline(), &SettingsData::default());
        assert!(problem.is_none());
    }

    /// A broken file gives the defaults, as before, and the user is told:
    /// the next save writes over the file.
    #[test]
    fn at_start_a_broken_file_gives_the_defaults_and_a_problem() {
        let root = TempDir::new("settings-start-broken");
        let path = root.path().join("settings.json");
        fs::write(&path, "{ this is not valid json")
            .expect("invalid test payload should be written");

        let (state, problem) = initial_settings_from(&path);

        let error = serde_json::from_str::<serde_json::Value>(
            "{ this is not valid json",
        )
        .expect_err("the payload should be invalid JSON");
        let expected =
            i18n::with_error(Key::TplSettingsFileInvalid, &format!("{error}"));
        assert_eq!(state.baseline(), &SettingsData::default());
        assert_eq!(problem.as_deref(), Some(expected.as_str()));
    }

    /// A file that cannot be read, here a folder in its place, gives the
    /// defaults and a problem.
    #[test]
    fn at_start_an_unreadable_file_gives_the_defaults_and_a_problem() {
        let root = TempDir::new("settings-start-unreadable");
        let path = root.path().join("settings.json");
        fs::create_dir_all(&path).expect("folder should be created");

        let (state, problem) = initial_settings_from(&path);

        let error = SettingsError::from(
            fs::read_to_string(&path).expect_err("a folder is no text file"),
        );
        let expected =
            i18n::with_error(Key::TplSettingsReadFailed, &format!("{error}"));
        assert_eq!(state.baseline(), &SettingsData::default());
        assert_eq!(problem.as_deref(), Some(expected.as_str()));
    }

    #[test]
    fn tests_never_read_the_settings_of_the_user() {
        let Ok(home) = std::env::var("HOME") else {
            return;
        };

        assert!(!settings_path().starts_with(home));
    }

    #[test]
    fn given_valid_settings_when_save_and_load_then_round_trip_matches() {
        let root = TempDir::new("settings-round-trip");
        let path = root.path().join("settings.json");
        let mut settings = SettingsData::default();
        settings.set_terminal_shell(String::from("/bin/zsh"));
        settings.set_terminal_editor(String::from("vim"));
        settings.set_theme(BuiltinTheme::OttyLight);
        settings.set_language(LanguageSetting::Fixed(Locale::ZhCn));

        save_settings_to_path(&path, &settings)
            .expect("settings should save successfully");
        let loaded = load_settings_from_path(&path)
            .expect("settings should load successfully");
        let (loaded_settings, loaded_status) = loaded.into_parts();

        assert!(matches!(loaded_status, SettingsLoadStatus::Loaded));
        assert_eq!(loaded_settings, settings);
    }

    #[test]
    fn given_invalid_json_when_load_then_returns_default_with_invalid_status() {
        let root = TempDir::new("settings-invalid-json");
        let path = root.path().join("settings.json");
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
    }

    #[test]
    fn given_missing_file_when_load_then_returns_default_with_missing_status() {
        let root = TempDir::new("settings-missing-file");
        let path = root.path().join("nonexistent.json");

        let loaded = load_settings_from_path(&path)
            .expect("loading missing settings should not fail");
        let (loaded_settings, loaded_status) = loaded.into_parts();

        assert_eq!(loaded_settings, SettingsData::default());
        assert!(matches!(loaded_status, SettingsLoadStatus::Missing));
    }

    #[test]
    fn given_old_palette_file_when_loaded_and_saved_then_theme_name_replaces_palette()
     {
        let root = TempDir::new("settings-old-palette");
        let path = root.path().join("settings.json");
        let old_payload = serde_json::json!({
            "terminal": { "shell": "/bin/zsh" },
            "theme": { "palette": ["#282A36", "#F8F8F2"] }
        });
        fs::write(&path, old_payload.to_string())
            .expect("old settings payload should be written");

        let (loaded_settings, _) = load_settings_from_path(&path)
            .expect("old settings should load")
            .into_parts();
        save_settings_to_path(&path, &loaded_settings)
            .expect("settings should save successfully");
        let saved: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(&path).expect("saved settings should be read"),
        )
        .expect("saved settings should be valid JSON");

        assert_eq!(loaded_settings.theme(), BuiltinTheme::OttyDark);
        assert_eq!(loaded_settings.terminal_shell(), "/bin/zsh");
        assert_eq!(saved["theme"], serde_json::json!({ "name": "otty-dark" }));
    }
}
