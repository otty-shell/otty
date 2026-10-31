use std::fmt;

use otty_ui_tree::TreeNode;
use serde::Serialize;

use crate::i18n::{self, Key, Locale};
use crate::theme::BuiltinTheme;

const DEFAULT_EDITOR: &str = "nano";
const FALLBACK_SHELL: &str = "/bin/bash";

/// Interface language choice stored in the settings file.
///
/// Adding a language requires no change here: the selector is derived from
/// [`Locale::ALL`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum LanguageSetting {
    /// Follow the operating system locale.
    #[default]
    System,
    /// Always use one specific locale.
    Fixed(Locale),
}

impl LanguageSetting {
    /// All choices offered by the language selector, system first.
    pub(crate) fn all() -> Vec<Self> {
        let mut options = Vec::with_capacity(Locale::ALL.len() + 1);
        options.push(Self::System);
        options.extend(Locale::ALL.map(Self::Fixed));

        options
    }

    /// Resolve the setting to the locale that should be rendered.
    pub(crate) fn resolve(self) -> Locale {
        match self {
            Self::Fixed(locale) => locale,
            _ => i18n::detect_system_locale(),
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
pub(crate) struct GeneralSettingsData {
    language: LanguageSetting,
}

/// Terminal-related settings.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct TerminalSettingsData {
    shell: String,
    editor: String,
    equalize_panes: bool,
}

impl Default for TerminalSettingsData {
    fn default() -> Self {
        Self {
            shell: default_shell(),
            editor: String::from(DEFAULT_EDITOR),
            equalize_panes: true,
        }
    }
}

/// Theme-related settings. Only the theme name is stored; the colors come
/// from the theme itself.
#[derive(Debug, Clone, PartialEq, Serialize, Default)]
pub(crate) struct ThemeSettingsData {
    #[serde(serialize_with = "serialize_theme_name")]
    name: BuiltinTheme,
}

/// Typed settings payload used for persistence and UI state.
#[derive(Debug, Clone, PartialEq, Serialize, Default)]
pub(crate) struct SettingsData {
    general: GeneralSettingsData,
    terminal: TerminalSettingsData,
    theme: ThemeSettingsData,
}

impl SettingsData {
    /// Return the configured interface language.
    pub(crate) fn language(&self) -> LanguageSetting {
        self.general.language
    }

    /// Update the interface language.
    pub(crate) fn set_language(&mut self, value: LanguageSetting) {
        self.general.language = value;
    }

    /// Return shell command for terminal sessions.
    pub(crate) fn terminal_shell(&self) -> &str {
        &self.terminal.shell
    }

    /// Update shell command for terminal sessions.
    pub(crate) fn set_terminal_shell(&mut self, value: String) {
        self.terminal.shell = value;
    }

    /// Return editor command used by the explorer open action.
    pub(crate) fn terminal_editor(&self) -> &str {
        &self.terminal.editor
    }

    /// Update editor command used by the explorer open action.
    pub(crate) fn set_terminal_editor(&mut self, value: String) {
        self.terminal.editor = value;
    }

    /// Return whether sibling panes are evened out on split and close.
    pub(crate) fn equalize_panes(&self) -> bool {
        self.terminal.equalize_panes
    }

    /// Update whether sibling panes are evened out on split and close.
    pub(crate) fn set_equalize_panes(&mut self, value: bool) {
        self.terminal.equalize_panes = value;
    }

    /// Return the selected theme.
    pub(crate) fn theme(&self) -> BuiltinTheme {
        self.theme.name
    }

    /// Select the theme.
    pub(crate) fn set_theme(&mut self, value: BuiltinTheme) {
        self.theme.name = value;
    }

    /// Parse settings from a raw JSON value, falling back to defaults.
    pub(crate) fn from_json(value: &serde_json::Value) -> Self {
        let mut settings = SettingsData::default();

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

    /// Return a copy with invalid or empty fields replaced by defaults.
    pub(crate) fn normalized(&self) -> Self {
        let defaults = SettingsData::default();

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

/// Top-level settings pages shown in the settings tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SettingsSection {
    General,
    Appearance,
    Terminal,
}

impl SettingsSection {
    /// All sections, in the order they appear in the navigation tree.
    pub(crate) const ALL: [Self; 3] =
        [Self::General, Self::Appearance, Self::Terminal];

    /// Human-readable section title in the active locale.
    ///
    /// The title doubles as the tree path segment, so callers must rebuild
    /// paths whenever the locale changes.
    pub(crate) fn title(&self) -> &'static str {
        match self {
            Self::General => i18n::t(Key::SectionGeneral),
            Self::Appearance => i18n::t(Key::SectionAppearance),
            _ => i18n::t(Key::SectionTerminal),
        }
    }
}

/// Node kinds for the settings navigation tree.
#[derive(Debug, Clone)]
pub(crate) enum SettingsNodeKind {
    Folder,
    Section(SettingsSection),
}

/// Navigation tree node for the settings sidebar.
#[derive(Debug, Clone)]
pub(crate) struct SettingsNode {
    title: String,
    expanded: bool,
    kind: SettingsNodeKind,
    children: Vec<SettingsNode>,
}

impl TreeNode for SettingsNode {
    fn title(&self) -> &str {
        SettingsNode::title(self)
    }

    fn children(&self) -> Option<&[Self]> {
        if SettingsNode::is_folder(self) {
            Some(SettingsNode::children(self))
        } else {
            None
        }
    }

    fn expanded(&self) -> bool {
        self.is_expanded()
    }

    fn is_folder(&self) -> bool {
        SettingsNode::is_folder(self)
    }
}

impl SettingsNode {
    /// Create a folder node with children.
    pub(crate) fn folder(
        title: impl Into<String>,
        children: Vec<SettingsNode>,
    ) -> Self {
        Self {
            title: title.into(),
            expanded: true,
            kind: SettingsNodeKind::Folder,
            children,
        }
    }

    /// Create a leaf section node.
    pub(crate) fn section(section: SettingsSection) -> Self {
        Self {
            title: section.title().to_string(),
            expanded: false,
            kind: SettingsNodeKind::Section(section),
            children: Vec::new(),
        }
    }

    /// Return whether this node is a folder.
    pub(crate) fn is_folder(&self) -> bool {
        matches!(self.kind, SettingsNodeKind::Folder)
    }

    /// Return the display title.
    pub(crate) fn title(&self) -> &str {
        &self.title
    }

    /// Return child nodes.
    pub(crate) fn children(&self) -> &[SettingsNode] {
        &self.children
    }

    /// Return whether this folder is expanded.
    pub(crate) fn is_expanded(&self) -> bool {
        self.expanded
    }

    /// Return the section kind if this is a section node.
    pub(crate) fn section_kind(&self) -> Option<SettingsSection> {
        match self.kind {
            SettingsNodeKind::Section(section) => Some(section),
            _ => None,
        }
    }

    /// Toggle folder expansion (used by the reducer).
    pub(super) fn toggle_expanded(&mut self) {
        self.expanded = !self.expanded;
    }

    /// Return mutable child nodes (used by state tree traversal).
    pub(super) fn children_mut(&mut self) -> &mut Vec<SettingsNode> {
        &mut self.children
    }
}

/// Status describing how settings were loaded from disk.
#[derive(Debug, Clone)]
pub(crate) enum SettingsLoadStatus {
    /// File existed and was parsed successfully.
    Loaded,
    /// File did not exist; defaults were used.
    Missing,
    /// File existed but contained invalid data.
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

fn read_string_field(value: &serde_json::Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(ToString::to_string)
}

fn is_non_empty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn default_shell() -> String {
    std::env::var("SHELL").unwrap_or_else(|_| FALLBACK_SHELL.to_string())
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

    use super::{BuiltinTheme, LanguageSetting, Locale, SettingsData};

    #[test]
    fn given_light_theme_name_when_from_json_then_otty_light_is_loaded() {
        let value = json!({ "theme": { "name": "otty-light" } });

        let settings = SettingsData::from_json(&value);

        assert_eq!(settings.theme(), BuiltinTheme::OttyLight);
    }

    #[test]
    fn given_missing_theme_name_when_from_json_then_otty_dark_is_loaded() {
        let without_theme = json!({ "terminal": { "shell": "/bin/sh" } });
        let without_name = json!({ "theme": {} });

        assert_eq!(
            SettingsData::from_json(&without_theme).theme(),
            BuiltinTheme::OttyDark
        );
        assert_eq!(
            SettingsData::from_json(&without_name).theme(),
            BuiltinTheme::OttyDark
        );
    }

    #[test]
    fn given_unknown_theme_name_when_from_json_then_otty_dark_is_loaded() {
        let value = json!({ "theme": { "name": "dracula" } });

        let settings = SettingsData::from_json(&value);

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

        let settings = SettingsData::from_json(&value);

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

        let settings = SettingsData::from_json(&value);

        assert!(!settings.equalize_panes());
    }

    #[test]
    fn given_equalize_panes_disabled_when_normalized_then_value_is_kept() {
        let mut settings = SettingsData::default();
        settings.set_equalize_panes(false);

        assert!(!settings.normalized().equalize_panes());
    }

    #[test]
    fn given_invalid_fields_when_normalized_then_defaults_are_applied() {
        let defaults = SettingsData::default();
        let mut settings = SettingsData::default();
        settings.terminal.shell = String::from("   ");
        settings.terminal.editor = String::new();

        let normalized = settings.normalized();

        assert_eq!(normalized.terminal.shell, defaults.terminal.shell);
        assert_eq!(normalized.terminal.editor, defaults.terminal.editor);
    }

    #[test]
    fn given_light_theme_when_normalized_then_theme_is_kept() {
        let mut settings = SettingsData::default();
        settings.set_theme(BuiltinTheme::OttyLight);

        assert_eq!(settings.normalized().theme(), BuiltinTheme::OttyLight);
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
            assert_eq!(LanguageSetting::from_tag(language.tag()), language);
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
    fn given_explicit_language_when_resolved_then_locale_matches() {
        for locale in Locale::ALL {
            assert_eq!(LanguageSetting::Fixed(locale).resolve(), locale);
        }
    }

    #[test]
    fn given_language_in_json_when_parsed_then_settings_carry_it() {
        let value = serde_json::json!({ "general": { "language": "zh-CN" } });

        let settings = SettingsData::from_json(&value);

        assert_eq!(settings.language(), LanguageSetting::Fixed(Locale::ZhCn));
    }

    #[test]
    fn given_json_without_general_when_parsed_then_language_defaults_to_system()
    {
        let value = serde_json::json!({ "terminal": { "shell": "/bin/sh" } });

        let settings = SettingsData::from_json(&value);

        assert_eq!(settings.language(), LanguageSetting::System);
    }

    #[test]
    fn given_language_when_normalized_then_selection_is_preserved() {
        let mut settings = SettingsData::default();
        settings.set_language(LanguageSetting::Fixed(Locale::ZhTw));

        let normalized = settings.normalized();

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
}
