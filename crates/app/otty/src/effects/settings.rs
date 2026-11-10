//! The settings file, `~/.config/otty/settings.json`: read once at start
//! before the first frame, read again when the Settings dialog opens, and
//! written on Save. Turning its text into settings is the settings feature's
//! work ([`SettingsLoad::parse`]); only the file reads and writes are here.

#![allow(
    clippy::disallowed_methods,
    reason = "effects/ is where the binary reads and writes files"
)]

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use iced::Task;
use otty_app_settings::{
    SettingsData, SettingsIntent, SettingsLoad, SettingsLoadStatus,
    SettingsProblem,
};
use thiserror::Error;

use crate::message::AppEvent;

/// Errors of reading or writing the settings file.
#[derive(Debug, Error)]
enum SettingsError {
    /// The file could not be read or written.
    #[error("settings IO failed: {0}")]
    Io(#[from] std::io::Error),
    /// The settings could not be turned into JSON.
    #[error("settings JSON failed: {0}")]
    Json(#[from] serde_json::Error),
}

/// Read the saved settings at start. A missing file is the first start; a
/// file that can not be read or is broken gives the defaults too, and the
/// problem the user is told about. The default shell is `default_shell`.
pub(crate) fn read_at_start(
    default_shell: &str,
) -> (SettingsData, Option<SettingsProblem>) {
    read_at_start_from(&settings_path(), default_shell)
}

/// Read the settings file in the background, for the open dialog, with
/// `default_shell` as the default shell; the answer comes back as
/// [`SettingsIntent::ReloadLoaded`] or [`SettingsIntent::ReloadFailed`].
pub(crate) fn read(default_shell: String) -> Task<AppEvent> {
    Task::perform(
        async move {
            read_from(&settings_path(), &default_shell)
                .map_err(|err| format!("{err}"))
        },
        read_finished,
    )
}

/// Write `settings` in the background; the answer comes back as
/// [`SettingsIntent::SaveCompleted`] with what was written, or
/// [`SettingsIntent::SaveFailed`].
pub(crate) fn write(settings: SettingsData) -> Task<AppEvent> {
    Task::perform(
        async move {
            match write_to(&settings_path(), &settings) {
                Ok(()) => Ok(settings),
                Err(err) => Err(format!("{err}")),
            }
        },
        write_finished,
    )
}

fn read_at_start_from(
    path: &Path,
    default_shell: &str,
) -> (SettingsData, Option<SettingsProblem>) {
    match read_from(path, default_shell) {
        Ok(load) => match load.into_parts() {
            (data, SettingsLoadStatus::Invalid(message)) => {
                log::warn!("settings file invalid: {message}");
                (data, Some(SettingsProblem::FileInvalid(message)))
            },
            (data, _) => (data, None),
        },
        Err(err) => {
            log::warn!("settings read failed: {err}");
            let problem = SettingsProblem::ReadFailed(format!("{err}"));
            (SettingsData::defaults(default_shell), Some(problem))
        },
    }
}

fn read_finished(result: Result<SettingsLoad, String>) -> AppEvent {
    let intent = match result {
        Ok(load) => SettingsIntent::ReloadLoaded(load),
        Err(message) => SettingsIntent::ReloadFailed(message),
    };

    AppEvent::Settings(intent)
}

fn write_finished(result: Result<SettingsData, String>) -> AppEvent {
    let intent = match result {
        Ok(settings) => SettingsIntent::SaveCompleted(settings),
        Err(message) => SettingsIntent::SaveFailed(message),
    };

    AppEvent::Settings(intent)
}

fn read_from(
    path: &Path,
    default_shell: &str,
) -> Result<SettingsLoad, SettingsError> {
    match fs::read_to_string(path) {
        Ok(contents) => Ok(SettingsLoad::parse(&contents, default_shell)),
        Err(err) if err.kind() == ErrorKind::NotFound => {
            Ok(SettingsLoad::missing(default_shell))
        },
        Err(err) => Err(err.into()),
    }
}

fn write_to(path: &Path, settings: &SettingsData) -> Result<(), SettingsError> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }

    let payload = serde_json::to_string_pretty(settings)?;
    write_atomic(path, payload.as_bytes())?;

    Ok(())
}

/// Write the file next to its place and move it there, so a failed write
/// never leaves half a file.
fn write_atomic(path: &Path, payload: &[u8]) -> Result<(), std::io::Error> {
    let tmp_path = path.with_extension("json.tmp");

    fs::write(&tmp_path, payload)?;
    fs::rename(tmp_path, path)?;

    Ok(())
}

fn settings_path() -> PathBuf {
    // Applying read settings sets the process-wide language, so tests that
    // run a read must not depend on the settings of the user running them.
    if cfg!(test) {
        return std::env::temp_dir()
            .join("otty-app-tests")
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

#[cfg(test)]
mod tests {
    use otty_app_i18n::Locale;
    use otty_app_settings::LanguageSetting;
    use otty_app_testing::temp_dir::TempDir;
    use otty_app_theme::theme::BuiltinTheme;
    use serde_json::json;

    use super::*;

    /// The system shell the binary would read from `SHELL`.
    const DEFAULT_SHELL: &str = "/usr/bin/otty-test-default-shell";

    /// A missing file is the first start: the defaults, and nothing to
    /// tell.
    #[test]
    fn at_start_a_missing_file_gives_the_defaults_and_tells_nothing() {
        let folder = TempDir::new("settings-start-missing");

        let (saved, problem) = read_at_start_from(
            &folder.path().join("settings.json"),
            DEFAULT_SHELL,
        );

        assert_eq!(saved, SettingsData::defaults(DEFAULT_SHELL));
        assert_eq!(problem, None);
    }

    /// A broken file gives the defaults, as before, and the user is told:
    /// the next save writes over the file.
    #[test]
    fn at_start_a_broken_file_gives_the_defaults_and_a_problem() {
        let folder = TempDir::new("settings-start-broken");
        let path = folder.path().join("settings.json");
        fs::write(&path, "{ this is not valid json").expect("write the file");

        let (saved, problem) = read_at_start_from(&path, DEFAULT_SHELL);

        let error = serde_json::from_str::<serde_json::Value>(
            "{ this is not valid json",
        )
        .expect_err("the payload should be invalid JSON");
        assert_eq!(saved, SettingsData::defaults(DEFAULT_SHELL));
        assert_eq!(
            problem,
            Some(SettingsProblem::FileInvalid(format!("{error}")))
        );
    }

    /// A file that can not be read, here a folder in its place, gives the
    /// defaults and a problem.
    #[test]
    fn at_start_an_unreadable_file_gives_the_defaults_and_a_problem() {
        let folder = TempDir::new("settings-start-unreadable");
        let path = folder.path().join("settings.json");
        fs::create_dir_all(&path).expect("create a folder");

        let (saved, problem) = read_at_start_from(&path, DEFAULT_SHELL);

        let error = fs::read_to_string(&path).expect_err("a folder is no file");
        assert_eq!(saved, SettingsData::defaults(DEFAULT_SHELL));
        assert_eq!(
            problem,
            Some(SettingsProblem::ReadFailed(format!(
                "settings IO failed: {error}"
            )))
        );
    }

    #[test]
    fn at_start_a_saved_file_gives_its_settings() {
        let folder = TempDir::new("settings-start-saved");
        let path = folder.path().join("settings.json");
        let contents = json!({
            "general": { "language": "de" },
            "terminal": { "shell": "/bin/zsh", "editor": "vim" },
            "theme": { "name": "otty-light" },
        });
        fs::write(&path, contents.to_string()).expect("write the file");

        let (saved, problem) = read_at_start_from(&path, DEFAULT_SHELL);

        assert_eq!(problem, None);
        assert_eq!(saved.language(), LanguageSetting::Fixed(Locale::De));
        assert_eq!(saved.terminal_shell(), "/bin/zsh");
        assert_eq!(saved.terminal_editor(), "vim");
        assert_eq!(saved.theme(), BuiltinTheme::OttyLight);
    }

    #[test]
    fn tests_never_read_the_settings_of_the_user() {
        let Ok(home) = std::env::var("HOME") else {
            return;
        };

        assert!(!settings_path().starts_with(home));
    }

    #[test]
    fn written_settings_are_read_back_the_same() {
        let folder = TempDir::new("settings-round-trip");
        let path = folder.path().join("otty").join("settings.json");
        let mut settings = SettingsData::defaults(DEFAULT_SHELL);
        settings.set_terminal_shell(String::from("/bin/zsh"));
        settings.set_terminal_editor(String::from("vim"));
        settings.set_theme(BuiltinTheme::OttyLight);
        settings.set_language(LanguageSetting::Fixed(Locale::ZhCn));

        write_to(&path, &settings).expect("settings should be written");
        let (read, status) = read_from(&path, DEFAULT_SHELL)
            .expect("settings should be read")
            .into_parts();

        assert!(matches!(status, SettingsLoadStatus::Loaded));
        assert_eq!(read, settings);
        assert!(!path.with_extension("json.tmp").exists());
    }

    #[test]
    fn a_missing_file_reads_as_missing() {
        let folder = TempDir::new("settings-missing-file");

        let (read, status) =
            read_from(&folder.path().join("settings.json"), DEFAULT_SHELL)
                .expect("a missing file is no error")
                .into_parts();

        assert_eq!(read, SettingsData::defaults(DEFAULT_SHELL));
        assert!(matches!(status, SettingsLoadStatus::Missing));
    }

    /// A file can not be written where a folder stands in its place.
    #[test]
    fn a_blocked_write_fails_with_its_reason() {
        let folder = TempDir::new("settings-blocked");
        let path = folder.path().join("settings.json");
        fs::create_dir_all(path.with_extension("json.tmp"))
            .expect("create a folder");

        let error = write_to(&path, &SettingsData::defaults(DEFAULT_SHELL))
            .expect_err("the write should fail");

        assert!(format!("{error}").starts_with("settings IO failed"));
    }

    #[test]
    fn a_finished_read_reaches_the_feature() {
        assert!(matches!(
            read_finished(Ok(SettingsLoad::missing(DEFAULT_SHELL))),
            AppEvent::Settings(SettingsIntent::ReloadLoaded(_))
        ));
    }

    #[test]
    fn a_failed_read_reaches_the_feature_with_its_reason() {
        assert!(matches!(
            read_finished(Err(String::from("no access"))),
            AppEvent::Settings(SettingsIntent::ReloadFailed(message))
                if message == "no access"
        ));
    }

    #[test]
    fn a_finished_write_reaches_the_feature_with_what_was_written() {
        let written = SettingsData::defaults(DEFAULT_SHELL);

        assert!(matches!(
            write_finished(Ok(written.clone())),
            AppEvent::Settings(SettingsIntent::SaveCompleted(settings))
                if settings == written
        ));
    }

    #[test]
    fn a_failed_write_reaches_the_feature_with_its_reason() {
        assert!(matches!(
            write_finished(Err(String::from("disk full"))),
            AppEvent::Settings(SettingsIntent::SaveFailed(message))
                if message == "disk full"
        ));
    }
}
