use std::fmt;
use std::sync::OnceLock;

use otty_app_i18n::{self as i18n, Key, Locale};
use otty_app_theme::theme::BuiltinTheme;
use serde::Serialize;

const DEFAULT_EDITOR: &str = "nano";
/// The system shell the tests of this crate give as the default shell.
#[cfg(test)]
pub(crate) const TEST_DEFAULT_SHELL: &str = "/bin/sh";

/// Interface language choice stored in the settings file.
///
/// Adding a language requires no change here: the selector is derived from
/// [`Locale::ALL`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LanguageSetting {
    /// Follow the operating system locale.
    #[default]
    System,
    /// Always use one specific locale.
    Fixed(Locale),
}

impl LanguageSetting {
    /// All choices offered by the language selector, system first.
    pub fn all() -> &'static [Self] {
        static OPTIONS: OnceLock<Vec<LanguageSetting>> = OnceLock::new();

        OPTIONS.get_or_init(|| {
            let mut options = Vec::with_capacity(Locale::ALL.len() + 1);
            options.push(Self::System);
            options.extend(Locale::ALL.map(Self::Fixed));

            options
        })
    }

    /// Return the locale the user chose, or `None` to follow the system.
    /// The binary reads the system language, so it resolves `None`.
    pub fn fixed(self) -> Option<Locale> {
        match self {
            Self::Fixed(locale) => Some(locale),
            _ => None,
        }
    }

    /// Value persisted in the settings file.
    fn tag(self) -> &'static str {
        match self {
            Self::Fixed(locale) => locale.tag(),
            _ => "system",
        }
    }

    /// Parse a persisted value, falling back to [`LanguageSetting::System`].
    ///
    /// Only exact tag matches select a language, so an unrecognized value
    /// keeps following the system rather than silently picking a language.
    fn from_tag(tag: &str) -> Self {
        let normalized = tag.to_ascii_lowercase().replace('_', "-");
        Locale::ALL
            .into_iter()
            .find(|locale| locale.tag().to_ascii_lowercase() == normalized)
            .map_or(Self::System, Self::Fixed)
    }
}

impl fmt::Display for LanguageSetting {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Language names stay in their own language; only the
        // follow-the-system option is translated.
        match self {
            Self::Fixed(locale) => f.write_str(locale.native_name()),
            _ => f.write_str(i18n::t(Key::LanguageSystem)),
        }
    }
}

impl Serialize for LanguageSetting {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.tag())
    }
}

/// General settings that are not specific to the terminal or the theme.
#[derive(Debug, Clone, PartialEq, Serialize, Default)]
struct GeneralSettingsData {
    language: LanguageSetting,
}

/// Terminal-related settings.
#[derive(Debug, Clone, PartialEq, Serialize)]
struct TerminalSettingsData {
    shell: String,
    editor: String,
    equalize_panes: bool,
}

impl TerminalSettingsData {
    /// The default terminal settings, starting `default_shell`.
    fn defaults(default_shell: &str) -> Self {
        Self {
            shell: String::from(default_shell),
            editor: String::from(DEFAULT_EDITOR),
            equalize_panes: true,
        }
    }
}

/// Theme-related settings. Only the theme name is stored; the colors come
/// from the theme itself.
#[derive(Debug, Clone, PartialEq, Serialize, Default)]
struct ThemeSettingsData {
    #[serde(serialize_with = "serialize_theme_name")]
    name: BuiltinTheme,
}

/// Typed settings payload used for persistence and UI state.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SettingsData {
    general: GeneralSettingsData,
    terminal: TerminalSettingsData,
    theme: ThemeSettingsData,
}

impl SettingsData {
    /// Return the configured interface language.
    pub fn language(&self) -> LanguageSetting {
        self.general.language
    }

    /// Update the interface language.
    pub fn set_language(&mut self, value: LanguageSetting) {
        self.general.language = value;
    }

    /// Return shell command for terminal sessions.
    pub fn terminal_shell(&self) -> &str {
        &self.terminal.shell
    }

    /// Update shell command for terminal sessions.
    pub fn set_terminal_shell(&mut self, value: String) {
        self.terminal.shell = value;
    }

    /// Return editor command used by the explorer open action.
    pub fn terminal_editor(&self) -> &str {
        &self.terminal.editor
    }

    /// Update editor command used by the explorer open action.
    pub fn set_terminal_editor(&mut self, value: String) {
        self.terminal.editor = value;
    }

    /// Return whether sibling panes are evened out on split and close.
    pub fn equalize_panes(&self) -> bool {
        self.terminal.equalize_panes
    }

    /// Update whether sibling panes are evened out on split and close.
    pub fn set_equalize_panes(&mut self, value: bool) {
        self.terminal.equalize_panes = value;
    }

    /// Return the selected theme.
    pub fn theme(&self) -> BuiltinTheme {
        self.theme.name
    }

    /// Select the theme.
    pub fn set_theme(&mut self, value: BuiltinTheme) {
        self.theme.name = value;
    }

    /// Build the default settings. New terminals start `default_shell`:
    /// the system shell, which the binary reads at start.
    ///
    /// # Example
    ///
    /// ```
    /// use otty_app_settings::SettingsData;
    ///
    /// let settings = SettingsData::defaults("/bin/zsh");
    ///
    /// assert_eq!(settings.terminal_shell(), "/bin/zsh");
    /// ```
    pub fn defaults(default_shell: &str) -> Self {
        Self {
            general: GeneralSettingsData::default(),
            terminal: TerminalSettingsData::defaults(default_shell),
            theme: ThemeSettingsData::default(),
        }
    }

    /// Parse settings from a raw JSON value, falling back to the defaults
    /// with `default_shell`.
    pub fn from_json(value: &serde_json::Value, default_shell: &str) -> Self {
        let mut settings = SettingsData::defaults(default_shell);

        if let Some(general) = value.get("general")
            && let Some(language) = read_string_field(general, "language")
        {
            settings.general.language = LanguageSetting::from_tag(&language);
        }

        if let Some(terminal) = value.get("terminal") {
            if let Some(shell) = read_string_field(terminal, "shell")
                .filter(|value| is_non_empty(value))
            {
                settings.terminal.shell = shell;
            }

            if let Some(editor) = read_string_field(terminal, "editor")
                .filter(|value| is_non_empty(value))
            {
                settings.terminal.editor = editor;
            }

            if let Some(equalize_panes) = terminal
                .get("equalize_panes")
                .and_then(serde_json::Value::as_bool)
            {
                settings.terminal.equalize_panes = equalize_panes;
            }
        }

        // Old files keep the whole palette under `theme.palette`; it is
        // ignored, so they load as the default theme.
        if let Some(theme) = value.get("theme")
            && let Some(name) = read_string_field(theme, "name")
            && let Some(selected) = BuiltinTheme::from_name(&name)
        {
            settings.theme.name = selected;
        }

        settings
    }

    /// Return a copy with invalid or empty fields replaced by the defaults
    /// with `default_shell`.
    pub fn normalized(&self, default_shell: &str) -> Self {
        let defaults = SettingsData::defaults(default_shell);

        let shell = if is_non_empty(&self.terminal.shell) {
            self.terminal.shell.clone()
        } else {
            defaults.terminal.shell
        };

        let editor = if is_non_empty(&self.terminal.editor) {
            self.terminal.editor.clone()
        } else {
            defaults.terminal.editor
        };

        Self {
            general: self.general.clone(),
            terminal: TerminalSettingsData {
                shell,
                editor,
                equalize_panes: self.terminal.equalize_panes,
            },
            theme: self.theme.clone(),
        }
    }
}

/// The test defaults: the shell is [`TEST_DEFAULT_SHELL`].
#[cfg(test)]
impl Default for SettingsData {
    fn default() -> Self {
        Self::defaults(TEST_DEFAULT_SHELL)
    }
}

/// Status describing how settings were loaded from disk.
#[derive(Debug, Clone)]
pub enum SettingsLoadStatus {
    /// File existed and was parsed successfully.
    Loaded,
    /// File did not exist; defaults were used.
    Missing,
    /// File existed but contained invalid data.
    Invalid(String),
}

/// Result of loading settings from disk.
#[derive(Debug, Clone)]
pub struct SettingsLoad {
    settings: SettingsData,
    status: SettingsLoadStatus,
}

impl SettingsLoad {
    /// Build a settings load result from explicit parts.
    pub fn new(settings: SettingsData, status: SettingsLoadStatus) -> Self {
        Self { settings, status }
    }

    /// Build the result of reading the settings file whose text is
    /// `contents`. Text that is no valid JSON gives the defaults with the
    /// parse error; a field that is missing, empty or unknown keeps its
    /// default. The default shell is `default_shell`.
    pub fn parse(contents: &str, default_shell: &str) -> Self {
        match serde_json::from_str::<serde_json::Value>(contents) {
            Ok(value) => Self::new(
                SettingsData::from_json(&value, default_shell),
                SettingsLoadStatus::Loaded,
            ),
            Err(err) => Self::new(
                SettingsData::defaults(default_shell),
                SettingsLoadStatus::Invalid(format!("{err}")),
            ),
        }
    }

    /// Build the result of a settings file that does not exist yet, as on
    /// the first start: the defaults with `default_shell`.
    pub fn missing(default_shell: &str) -> Self {
        Self::new(
            SettingsData::defaults(default_shell),
            SettingsLoadStatus::Missing,
        )
    }

    /// Consume the value and return both payload and status.
    pub fn into_parts(self) -> (SettingsData, SettingsLoadStatus) {
        (self.settings, self.status)
    }
}

fn read_string_field(value: &serde_json::Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(ToString::to_string)
}

fn is_non_empty(value: &str) -> bool {
    !value.trim().is_empty()
}

/// Write the theme as its stored name, for example `"otty-dark"`.
fn serialize_theme_name<S: serde::Serializer>(
    theme: &BuiltinTheme,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(theme.name())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        BuiltinTheme, LanguageSetting, Locale, SettingsData, SettingsLoad,
        SettingsLoadStatus, TEST_DEFAULT_SHELL,
    };

    #[test]
    fn given_light_theme_name_when_from_json_then_otty_light_is_loaded() {
        let value = json!({ "theme": { "name": "otty-light" } });

        let settings = SettingsData::from_json(&value, TEST_DEFAULT_SHELL);

        assert_eq!(settings.theme(), BuiltinTheme::OttyLight);
    }

    #[test]
    fn given_missing_theme_name_when_from_json_then_otty_dark_is_loaded() {
        let without_theme = json!({ "terminal": { "shell": "/bin/sh" } });
        let without_name = json!({ "theme": {} });

        assert_eq!(
            SettingsData::from_json(&without_theme, TEST_DEFAULT_SHELL).theme(),
            BuiltinTheme::OttyDark
        );
        assert_eq!(
            SettingsData::from_json(&without_name, TEST_DEFAULT_SHELL).theme(),
            BuiltinTheme::OttyDark
        );
    }

    #[test]
    fn given_unknown_theme_name_when_from_json_then_otty_dark_is_loaded() {
        let value = json!({ "theme": { "name": "dracula" } });

        let settings = SettingsData::from_json(&value, TEST_DEFAULT_SHELL);

        assert_eq!(settings.theme(), BuiltinTheme::OttyDark);
    }

    #[test]
    fn given_old_file_with_only_palette_when_from_json_then_otty_dark_is_loaded()
     {
        let value = json!({
            "theme": {
                "palette": ["#F8F8F2", "#282A36", "#21222C", "#FF5555"]
            }
        });

        let settings = SettingsData::from_json(&value, TEST_DEFAULT_SHELL);

        assert_eq!(settings.theme(), BuiltinTheme::OttyDark);
    }

    #[test]
    fn given_theme_when_serialized_then_only_theme_name_is_written() {
        let mut settings = SettingsData::default();
        settings.set_theme(BuiltinTheme::OttyLight);

        let value =
            serde_json::to_value(&settings).expect("settings should serialize");

        assert_eq!(value["theme"], json!({ "name": "otty-light" }));
    }

    #[test]
    fn given_equalize_panes_flag_when_from_json_then_value_is_loaded() {
        assert!(SettingsData::default().equalize_panes());

        let value = json!({ "terminal": { "equalize_panes": false } });

        let settings = SettingsData::from_json(&value, TEST_DEFAULT_SHELL);

        assert!(!settings.equalize_panes());
    }

    #[test]
    fn given_equalize_panes_disabled_when_normalized_then_value_is_kept() {
        let mut settings = SettingsData::default();
        settings.set_equalize_panes(false);

        assert!(!settings.normalized(TEST_DEFAULT_SHELL).equalize_panes());
    }

    #[test]
    fn given_invalid_fields_when_normalized_then_defaults_are_applied() {
        let defaults = SettingsData::default();
        let mut settings = SettingsData::default();
        settings.terminal.shell = String::from("   ");
        settings.terminal.editor = String::new();

        let normalized = settings.normalized(TEST_DEFAULT_SHELL);

        assert_eq!(normalized.terminal.shell, defaults.terminal.shell);
        assert_eq!(normalized.terminal.editor, defaults.terminal.editor);
    }

    #[test]
    fn given_light_theme_when_normalized_then_theme_is_kept() {
        let mut settings = SettingsData::default();
        settings.set_theme(BuiltinTheme::OttyLight);

        assert_eq!(
            settings.normalized(TEST_DEFAULT_SHELL).theme(),
            BuiltinTheme::OttyLight
        );
    }

    #[test]
    fn given_language_tags_when_parsed_then_known_values_are_recognized() {
        assert_eq!(
            LanguageSetting::from_tag("en"),
            LanguageSetting::Fixed(Locale::En)
        );
        assert_eq!(
            LanguageSetting::from_tag("zh-CN"),
            LanguageSetting::Fixed(Locale::ZhCn)
        );
        assert_eq!(
            LanguageSetting::from_tag("zh_tw"),
            LanguageSetting::Fixed(Locale::ZhTw)
        );
        assert_eq!(
            LanguageSetting::from_tag("pt-br"),
            LanguageSetting::Fixed(Locale::PtBr)
        );
        assert_eq!(
            LanguageSetting::from_tag("system"),
            LanguageSetting::System
        );
        assert_eq!(
            LanguageSetting::from_tag("klingon"),
            LanguageSetting::System
        );
    }

    #[test]
    fn given_partial_language_tag_when_parsed_then_system_is_kept() {
        // Unlike locale detection, a stored setting must match exactly:
        // a bare "zh" is not a choice the selector can produce.
        assert_eq!(LanguageSetting::from_tag("zh"), LanguageSetting::System);
        assert_eq!(LanguageSetting::from_tag("en-GB"), LanguageSetting::System);
    }

    #[test]
    fn given_language_setting_when_round_tripped_then_value_is_preserved() {
        for language in LanguageSetting::all() {
            assert_eq!(LanguageSetting::from_tag(language.tag()), *language);
        }
    }

    #[test]
    fn given_selector_options_when_listed_then_system_leads_every_locale() {
        let options = LanguageSetting::all();

        assert_eq!(options.len(), Locale::ALL.len() + 1);
        assert_eq!(options.first(), Some(&LanguageSetting::System));
        for locale in Locale::ALL {
            assert!(options.contains(&LanguageSetting::Fixed(locale)));
        }
    }

    #[test]
    fn given_explicit_language_when_fixed_then_locale_matches() {
        for locale in Locale::ALL {
            assert_eq!(LanguageSetting::Fixed(locale).fixed(), Some(locale));
        }
    }

    /// The binary reads the system language, so following the system
    /// names no locale here.
    #[test]
    fn given_system_language_when_fixed_then_no_locale_is_named() {
        assert_eq!(LanguageSetting::System.fixed(), None);
    }

    #[test]
    fn given_a_saved_file_when_parsed_then_its_settings_are_loaded() {
        let mut settings = SettingsData::default();
        settings.set_terminal_shell(String::from("/bin/zsh"));
        settings.set_terminal_editor(String::from("vim"));
        settings.set_theme(BuiltinTheme::OttyLight);
        settings.set_language(LanguageSetting::Fixed(Locale::ZhCn));
        let contents = serde_json::to_string_pretty(&settings)
            .expect("settings should serialize");

        let (loaded, status) =
            SettingsLoad::parse(&contents, TEST_DEFAULT_SHELL).into_parts();

        assert!(matches!(status, SettingsLoadStatus::Loaded));
        assert_eq!(loaded, settings);
    }

    #[test]
    fn given_invalid_json_when_parsed_then_defaults_with_invalid_status() {
        let (loaded, status) =
            SettingsLoad::parse("{ this is not valid json", TEST_DEFAULT_SHELL)
                .into_parts();

        let error = serde_json::from_str::<serde_json::Value>(
            "{ this is not valid json",
        )
        .expect_err("the payload should be invalid JSON");
        assert_eq!(loaded, SettingsData::default());
        assert!(matches!(
            status,
            SettingsLoadStatus::Invalid(message) if message == format!("{error}")
        ));
    }

    #[test]
    fn a_missing_file_gives_the_defaults_with_missing_status() {
        let (loaded, status) =
            SettingsLoad::missing(TEST_DEFAULT_SHELL).into_parts();

        assert_eq!(loaded, SettingsData::default());
        assert!(matches!(status, SettingsLoadStatus::Missing));
    }

    #[test]
    fn given_old_palette_file_when_parsed_and_saved_then_theme_name_replaces_palette()
     {
        let old_payload = json!({
            "terminal": { "shell": "/bin/zsh" },
            "theme": { "palette": ["#282A36", "#F8F8F2"] }
        });

        let (loaded, _) =
            SettingsLoad::parse(&old_payload.to_string(), TEST_DEFAULT_SHELL)
                .into_parts();
        let saved =
            serde_json::to_value(&loaded).expect("settings should serialize");

        assert_eq!(loaded.theme(), BuiltinTheme::OttyDark);
        assert_eq!(loaded.terminal_shell(), "/bin/zsh");
        assert_eq!(saved["theme"], json!({ "name": "otty-dark" }));
    }

    #[test]
    fn given_language_in_json_when_parsed_then_settings_carry_it() {
        let value = serde_json::json!({ "general": { "language": "zh-CN" } });

        let settings = SettingsData::from_json(&value, TEST_DEFAULT_SHELL);

        assert_eq!(settings.language(), LanguageSetting::Fixed(Locale::ZhCn));
    }

    #[test]
    fn given_json_without_general_when_parsed_then_language_defaults_to_system()
    {
        let value = serde_json::json!({ "terminal": { "shell": "/bin/sh" } });

        let settings = SettingsData::from_json(&value, TEST_DEFAULT_SHELL);

        assert_eq!(settings.language(), LanguageSetting::System);
    }

    #[test]
    fn given_language_when_normalized_then_selection_is_preserved() {
        let mut settings = SettingsData::default();
        settings.set_language(LanguageSetting::Fixed(Locale::ZhTw));

        let normalized = settings.normalized(TEST_DEFAULT_SHELL);

        assert_eq!(normalized.language(), LanguageSetting::Fixed(Locale::ZhTw));
    }

    #[test]
    fn given_language_when_serialized_then_tag_is_written() {
        let mut settings = SettingsData::default();
        settings.set_language(LanguageSetting::Fixed(Locale::PtBr));

        let value =
            serde_json::to_value(&settings).expect("settings should serialize");

        assert_eq!(value["general"]["language"], "pt-BR");
    }

    #[test]
    fn given_system_language_when_serialized_then_system_tag_is_written() {
        let settings = SettingsData::default();

        let value =
            serde_json::to_value(&settings).expect("settings should serialize");

        assert_eq!(value["general"]["language"], "system");
    }

    /// The system shell the binary read at start.
    const SYSTEM_SHELL: &str = "/usr/bin/otty-system-shell";

    #[test]
    fn the_defaults_start_the_given_default_shell() {
        let settings = SettingsData::defaults(SYSTEM_SHELL);

        assert_eq!(settings.terminal_shell(), SYSTEM_SHELL);
    }

    #[test]
    fn a_missing_file_gives_the_given_default_shell() {
        let (loaded, _) = SettingsLoad::missing(SYSTEM_SHELL).into_parts();

        assert_eq!(loaded.terminal_shell(), SYSTEM_SHELL);
    }

    #[test]
    fn a_broken_file_gives_the_given_default_shell() {
        let (loaded, _) =
            SettingsLoad::parse("{ not json", SYSTEM_SHELL).into_parts();

        assert_eq!(loaded.terminal_shell(), SYSTEM_SHELL);
    }

    #[test]
    fn a_file_without_a_shell_gives_the_given_default_shell() {
        let without_shell = json!({ "terminal": { "editor": "vim" } });
        let empty_shell = json!({ "terminal": { "shell": "  " } });

        let (without, _) =
            SettingsLoad::parse(&without_shell.to_string(), SYSTEM_SHELL)
                .into_parts();
        let (empty, _) =
            SettingsLoad::parse(&empty_shell.to_string(), SYSTEM_SHELL)
                .into_parts();

        assert_eq!(without.terminal_shell(), SYSTEM_SHELL);
        assert_eq!(empty.terminal_shell(), SYSTEM_SHELL);
    }

    #[test]
    fn an_empty_shell_is_normalized_to_the_given_default_shell() {
        let mut settings = SettingsData::defaults("/bin/zsh");
        settings.set_terminal_shell(String::from(" "));

        let normalized = settings.normalized(SYSTEM_SHELL);

        assert_eq!(normalized.terminal_shell(), SYSTEM_SHELL);
    }
}
