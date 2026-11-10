//! What the frame reads from the settings file at start: the language, the
//! shell new terminals start and the theme.
//!
//! The settings feature (issue 09) brings the whole settings data, its
//! popup and its saving; its storage then replaces this reader here.

#![allow(
    clippy::disallowed_methods,
    reason = "effects/ is where the binary reads files"
)]

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use otty_app_i18n::{self as i18n, Key, Locale};
use otty_app_theme::theme::BuiltinTheme;

/// The shell when the settings name none and `SHELL` is not set.
const FALLBACK_SHELL: &str = "/bin/bash";

/// The saved settings the frame starts with.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SavedSettings {
    language: Option<Locale>,
    shell: String,
    theme: BuiltinTheme,
}

impl SavedSettings {
    /// The language the user chose; `None` follows the system.
    pub(crate) fn language(&self) -> Option<Locale> {
        self.language
    }

    /// The shell new terminal tabs start.
    pub(crate) fn shell(&self) -> &str {
        &self.shell
    }

    /// The theme of the window and new terminals.
    pub(crate) fn theme(&self) -> BuiltinTheme {
        self.theme
    }

    /// Take the frame's settings from the settings JSON. A field that is
    /// missing, empty or unknown keeps its default.
    fn from_json(value: &serde_json::Value) -> Self {
        let mut saved = Self::default();

        if let Some(tag) = value
            .get("general")
            .and_then(|general| read_string_field(general, "language"))
        {
            saved.language = fixed_language(tag);
        }

        if let Some(shell) = value
            .get("terminal")
            .and_then(|terminal| read_string_field(terminal, "shell"))
            .filter(|shell| !shell.trim().is_empty())
        {
            saved.shell = shell.to_string();
        }

        // Old files keep the whole palette under `theme.palette`; it is
        // ignored, so they load as the default theme.
        if let Some(theme) = value
            .get("theme")
            .and_then(|theme| read_string_field(theme, "name"))
            .and_then(BuiltinTheme::from_name)
        {
            saved.theme = theme;
        }

        saved
    }
}

impl Default for SavedSettings {
    /// Follow the system language, start the user's shell, use the default
    /// theme.
    fn default() -> Self {
        Self {
            language: None,
            shell: default_shell(),
            theme: BuiltinTheme::default(),
        }
    }
}

/// Why the settings file could not be used at start, so the frame started
/// with the defaults. The next save of the settings writes over the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SettingsProblem {
    /// The file is there but could not be read; carries the error.
    Unreadable(String),
    /// The file is no valid JSON; carries the error.
    Invalid(String),
}

impl SettingsProblem {
    /// The text that tells the user, in the active language. Build it after
    /// the language from the file is set.
    pub(crate) fn text(&self) -> String {
        match self {
            Self::Unreadable(error) => {
                i18n::with_error(Key::TplSettingsReadFailed, error)
            },
            Self::Invalid(error) => {
                i18n::with_error(Key::TplSettingsFileInvalid, error)
            },
        }
    }
}

/// Read the saved settings from `~/.config/otty/settings.json`. A missing
/// file is the first start; a file that can not be read or is broken gives
/// the defaults too, and the problem the user is told about.
pub(crate) fn read_saved() -> (SavedSettings, Option<SettingsProblem>) {
    read_saved_from(&settings_path())
}

fn read_saved_from(path: &Path) -> (SavedSettings, Option<SettingsProblem>) {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == ErrorKind::NotFound => {
            return (SavedSettings::default(), None);
        },
        Err(err) => {
            log::warn!("settings read failed: {err}");
            let problem = SettingsProblem::Unreadable(format!(
                "settings IO failed: {err}"
            ));
            return (SavedSettings::default(), Some(problem));
        },
    };

    match serde_json::from_str::<serde_json::Value>(&contents) {
        Ok(value) => (SavedSettings::from_json(&value), None),
        Err(err) => {
            log::warn!("settings file invalid: {err}");
            let problem = SettingsProblem::Invalid(format!("{err}"));
            (SavedSettings::default(), Some(problem))
        },
    }
}

/// The language a stored tag picks. Only an exact tag picks one, so an
/// unknown value, such as `system`, keeps following the system rather than
/// picking a language silently.
fn fixed_language(tag: &str) -> Option<Locale> {
    let normalized = tag.to_ascii_lowercase().replace('_', "-");

    Locale::ALL
        .into_iter()
        .find(|locale| locale.tag().to_ascii_lowercase() == normalized)
}

fn read_string_field<'a>(
    value: &'a serde_json::Value,
    key: &str,
) -> Option<&'a str> {
    value.get(key).and_then(serde_json::Value::as_str)
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

fn default_shell() -> String {
    std::env::var("SHELL").unwrap_or_else(|_| FALLBACK_SHELL.to_string())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::effects::temp_dir::TempDir;

    #[test]
    fn given_light_theme_name_when_read_then_otty_light_is_used() {
        let saved = SavedSettings::from_json(
            &json!({ "theme": { "name": "otty-light" } }),
        );

        assert_eq!(saved.theme(), BuiltinTheme::OttyLight);
    }

    #[test]
    fn given_missing_or_unknown_theme_name_when_read_then_otty_dark_is_used() {
        for value in [
            json!({ "terminal": { "shell": "/bin/sh" } }),
            json!({ "theme": {} }),
            json!({ "theme": { "name": "solarized" } }),
            json!({ "theme": { "palette": { "background": "#000000" } } }),
        ] {
            assert_eq!(
                SavedSettings::from_json(&value).theme(),
                BuiltinTheme::OttyDark,
                "{value}"
            );
        }
    }

    #[test]
    fn given_a_language_tag_when_read_then_that_language_is_used() {
        for (tag, locale) in [
            ("ja", Locale::Ja),
            ("pt-BR", Locale::PtBr),
            ("zh_tw", Locale::ZhTw),
            ("EN", Locale::En),
        ] {
            let saved = SavedSettings::from_json(
                &json!({ "general": { "language": tag } }),
            );

            assert_eq!(saved.language(), Some(locale), "{tag}");
        }
    }

    /// Only an exact tag picks a language, so an unknown value keeps
    /// following the system rather than picking one silently.
    #[test]
    fn given_system_or_an_unknown_language_when_read_then_the_system_is_followed()
     {
        for tag in ["system", "pt", "klingon", ""] {
            let saved = SavedSettings::from_json(
                &json!({ "general": { "language": tag } }),
            );

            assert_eq!(saved.language(), None, "{tag}");
        }
        assert_eq!(SavedSettings::from_json(&json!({})).language(), None);
    }

    #[test]
    fn given_a_shell_when_read_then_new_terminals_start_it() {
        let saved = SavedSettings::from_json(
            &json!({ "terminal": { "shell": "/usr/bin/fish" } }),
        );

        assert_eq!(saved.shell(), "/usr/bin/fish");
    }

    #[test]
    fn given_an_empty_or_missing_shell_when_read_then_the_default_shell_is_kept()
     {
        for value in [
            json!({ "terminal": { "shell": "  " } }),
            json!({ "terminal": {} }),
            json!({}),
        ] {
            assert_eq!(
                SavedSettings::from_json(&value).shell(),
                default_shell(),
                "{value}"
            );
        }
    }

    /// A missing file is the first start: the defaults, and nothing to
    /// tell.
    #[test]
    fn a_missing_file_gives_the_defaults_and_no_problem() {
        let folder = TempDir::new("settings-missing");

        let (saved, problem) =
            read_saved_from(&folder.path().join("settings.json"));

        assert_eq!(saved, SavedSettings::default());
        assert_eq!(problem, None);
    }

    /// The user is told why the defaults are used, as the old app did.
    #[test]
    fn a_broken_file_gives_the_defaults_and_tells_why() {
        let folder = TempDir::new("settings-broken");
        let broken = folder.path().join("broken.json");
        fs::write(&broken, "{ \"theme\": ").expect("write the file");

        let (saved, problem) = read_saved_from(&broken);

        let error = serde_json::from_str::<serde_json::Value>("{ \"theme\": ")
            .expect_err("the file is broken");
        let expected =
            i18n::with_error(Key::TplSettingsFileInvalid, &format!("{error}"));
        assert_eq!(saved, SavedSettings::default());
        assert_eq!(problem.map(|problem| problem.text()), Some(expected));
    }

    /// A file that can not be read, here a folder in its place, gives the
    /// defaults and the text that tells the user.
    #[test]
    fn an_unreadable_file_gives_the_defaults_and_tells_why() {
        let folder = TempDir::new("settings-unreadable");
        let unreadable = folder.path().join("folder.json");
        fs::create_dir_all(&unreadable).expect("create a folder");

        let (saved, problem) = read_saved_from(&unreadable);

        let error = fs::read_to_string(&unreadable)
            .expect_err("a folder is no text file");
        let expected = i18n::with_error(
            Key::TplSettingsReadFailed,
            &format!("settings IO failed: {error}"),
        );
        assert_eq!(saved, SavedSettings::default());
        assert_eq!(problem.map(|problem| problem.text()), Some(expected));
    }

    #[test]
    fn a_saved_file_gives_its_theme_language_and_shell() {
        let folder = TempDir::new("settings-saved");
        let path = folder.path().join("settings.json");
        let contents = json!({
            "general": { "language": "de" },
            "terminal": { "shell": "/bin/zsh", "editor": "vim" },
            "theme": { "name": "otty-light" },
        });
        fs::write(&path, contents.to_string()).expect("write the file");

        let (saved, problem) = read_saved_from(&path);

        assert_eq!(problem, None);
        assert_eq!(saved.language(), Some(Locale::De));
        assert_eq!(saved.shell(), "/bin/zsh");
        assert_eq!(saved.theme(), BuiltinTheme::OttyLight);
    }
}
