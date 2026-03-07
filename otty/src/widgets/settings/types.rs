use std::fmt;

use otty_ui_tree::TreeNode;
use serde::Serialize;

use crate::theme::ColorPalette;
use crate::widgets::settings::services::is_valid_hex_color;

const DEFAULT_EDITOR: &str = "nano";
const FALLBACK_SHELL: &str = "/bin/bash";

/// Terminal-related settings.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct TerminalSettingsData {
    shell: String,
    editor: String,
}

impl Default for TerminalSettingsData {
    fn default() -> Self {
        Self {
            shell: default_shell(),
            editor: String::from(DEFAULT_EDITOR),
        }
    }
}

/// Theme-related settings.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct ThemeSettingsData {
    palette: Vec<String>,
}

impl Default for ThemeSettingsData {
    fn default() -> Self {
        Self {
            palette: default_palette(),
        }
    }
}

impl ThemeSettingsData {
    /// Convert theme settings to a runtime color palette.
    fn to_color_palette(&self) -> ColorPalette {
        let base = ColorPalette::default();
        apply_palette_overrides(&base, &self.palette)
    }
}

/// Typed settings payload used for persistence and UI state.
#[derive(Debug, Clone, PartialEq, Serialize, Default)]
pub(crate) struct SettingsData {
    terminal: TerminalSettingsData,
    theme: ThemeSettingsData,
}

impl SettingsData {
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

    /// Return palette values used by the theme form.
    pub(crate) fn theme_palette(&self) -> &[String] {
        &self.theme.palette
    }

    /// Replace the full theme palette.
    pub(crate) fn set_theme_palette(&mut self, value: Vec<String>) {
        self.theme.palette = value;
    }

    /// Update one palette color by index; returns `true` on success.
    pub(crate) fn set_theme_palette_entry(
        &mut self,
        index: usize,
        value: String,
    ) -> bool {
        let Some(entry) = self.theme.palette.get_mut(index) else {
            return false;
        };
        *entry = value;
        true
    }

    /// Convert current theme settings to a runtime terminal palette.
    pub(crate) fn to_color_palette(&self) -> ColorPalette {
        self.theme.to_color_palette()
    }

    /// Parse settings from a raw JSON value, falling back to defaults.
    pub(crate) fn from_json(value: &serde_json::Value) -> Self {
        let mut settings = SettingsData::default();

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
        }

        if let Some(theme) = value.get("theme") {
            if let Some(palette) = read_palette(theme.get("palette")) {
                settings.theme.palette = palette;
            }
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

        let palette = if is_palette_valid(&self.theme.palette) {
            self.theme.palette.clone()
        } else {
            defaults.theme.palette
        };

        Self {
            terminal: TerminalSettingsData { shell, editor },
            theme: ThemeSettingsData { palette },
        }
    }
}

/// Top-level settings pages shown in the settings tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SettingsSection {
    Terminal,
    Appearance,
}

impl SettingsSection {
    /// Human-readable section title.
    pub(crate) fn title(&self) -> &'static str {
        match self {
            Self::Terminal => "Terminal",
            Self::Appearance => "Appearance",
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

/// Return the default palette as hex strings.
fn default_palette() -> Vec<String> {
    palette_from_colors(&ColorPalette::default())
}

/// Convert a `ColorPalette` to its list of hex-string values.
fn palette_from_colors(palette: &ColorPalette) -> Vec<String> {
    PALETTE_FIELDS
        .iter()
        .map(|field| palette_value(palette, *field).to_string())
        .collect()
}

fn apply_palette_overrides(
    base: &ColorPalette,
    values: &[String],
) -> ColorPalette {
    let mut palette = base.clone();
    for (index, value) in values.iter().enumerate() {
        if let Some(field) = PALETTE_FIELDS.get(index) {
            set_palette_value(&mut palette, *field, value.clone());
        }
    }
    palette
}

fn read_string_field(value: &serde_json::Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(ToString::to_string)
}

fn read_palette(value: Option<&serde_json::Value>) -> Option<Vec<String>> {
    let palette_value = value?;
    let entries = palette_value.as_array()?;
    if entries.is_empty() {
        return None;
    }

    let mut palette = Vec::with_capacity(entries.len());
    for entry in entries {
        let value = entry.as_str()?.to_string();
        if !is_valid_hex_color(&value) {
            return None;
        }
        palette.push(value);
    }

    Some(palette)
}

fn is_non_empty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn is_palette_valid(values: &[String]) -> bool {
    !values.is_empty() && values.iter().all(|value| is_valid_hex_color(value))
}

fn default_shell() -> String {
    std::env::var("SHELL").unwrap_or_else(|_| FALLBACK_SHELL.to_string())
}

pub(super) const PALETTE_LABELS: [&str; 29] = [
    "Foreground",
    "Background",
    "Black",
    "Red",
    "Green",
    "Yellow",
    "Blue",
    "Magenta",
    "Cyan",
    "White",
    "Bright Black",
    "Bright Red",
    "Bright Green",
    "Bright Yellow",
    "Bright Blue",
    "Bright Magenta",
    "Bright Cyan",
    "Bright White",
    "Bright Foreground",
    "Dim Black",
    "Dim Red",
    "Dim Green",
    "Dim Yellow",
    "Dim Blue",
    "Dim Magenta",
    "Dim Cyan",
    "Dim White",
    "Dim Foreground",
    "Overlay",
];

#[derive(Debug, Clone, Copy)]
enum PaletteField {
    Foreground,
    Background,
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
    BrightForeground,
    DimBlack,
    DimRed,
    DimGreen,
    DimYellow,
    DimBlue,
    DimMagenta,
    DimCyan,
    DimWhite,
    DimForeground,
    Overlay,
}

const PALETTE_FIELDS: [PaletteField; 29] = [
    PaletteField::Foreground,
    PaletteField::Background,
    PaletteField::Black,
    PaletteField::Red,
    PaletteField::Green,
    PaletteField::Yellow,
    PaletteField::Blue,
    PaletteField::Magenta,
    PaletteField::Cyan,
    PaletteField::White,
    PaletteField::BrightBlack,
    PaletteField::BrightRed,
    PaletteField::BrightGreen,
    PaletteField::BrightYellow,
    PaletteField::BrightBlue,
    PaletteField::BrightMagenta,
    PaletteField::BrightCyan,
    PaletteField::BrightWhite,
    PaletteField::BrightForeground,
    PaletteField::DimBlack,
    PaletteField::DimRed,
    PaletteField::DimGreen,
    PaletteField::DimYellow,
    PaletteField::DimBlue,
    PaletteField::DimMagenta,
    PaletteField::DimCyan,
    PaletteField::DimWhite,
    PaletteField::DimForeground,
    PaletteField::Overlay,
];

fn palette_value(palette: &ColorPalette, field: PaletteField) -> &str {
    match field {
        PaletteField::Foreground => &palette.foreground,
        PaletteField::Background => &palette.background,
        PaletteField::Black => &palette.black,
        PaletteField::Red => &palette.red,
        PaletteField::Green => &palette.green,
        PaletteField::Yellow => &palette.yellow,
        PaletteField::Blue => &palette.blue,
        PaletteField::Magenta => &palette.magenta,
        PaletteField::Cyan => &palette.cyan,
        PaletteField::White => &palette.white,
        PaletteField::BrightBlack => &palette.bright_black,
        PaletteField::BrightRed => &palette.bright_red,
        PaletteField::BrightGreen => &palette.bright_green,
        PaletteField::BrightYellow => &palette.bright_yellow,
        PaletteField::BrightBlue => &palette.bright_blue,
        PaletteField::BrightMagenta => &palette.bright_magenta,
        PaletteField::BrightCyan => &palette.bright_cyan,
        PaletteField::BrightWhite => &palette.bright_white,
        PaletteField::BrightForeground => &palette.bright_foreground,
        PaletteField::DimBlack => &palette.dim_black,
        PaletteField::DimRed => &palette.dim_red,
        PaletteField::DimGreen => &palette.dim_green,
        PaletteField::DimYellow => &palette.dim_yellow,
        PaletteField::DimBlue => &palette.dim_blue,
        PaletteField::DimMagenta => &palette.dim_magenta,
        PaletteField::DimCyan => &palette.dim_cyan,
        PaletteField::DimWhite => &palette.dim_white,
        PaletteField::DimForeground => &palette.dim_foreground,
        PaletteField::Overlay => &palette.overlay,
    }
}

fn set_palette_value(
    palette: &mut ColorPalette,
    field: PaletteField,
    value: String,
) {
    match field {
        PaletteField::Foreground => palette.foreground = value,
        PaletteField::Background => palette.background = value,
        PaletteField::Black => palette.black = value,
        PaletteField::Red => palette.red = value,
        PaletteField::Green => palette.green = value,
        PaletteField::Yellow => palette.yellow = value,
        PaletteField::Blue => palette.blue = value,
        PaletteField::Magenta => palette.magenta = value,
        PaletteField::Cyan => palette.cyan = value,
        PaletteField::White => palette.white = value,
        PaletteField::BrightBlack => palette.bright_black = value,
        PaletteField::BrightRed => palette.bright_red = value,
        PaletteField::BrightGreen => palette.bright_green = value,
        PaletteField::BrightYellow => palette.bright_yellow = value,
        PaletteField::BrightBlue => palette.bright_blue = value,
        PaletteField::BrightMagenta => palette.bright_magenta = value,
        PaletteField::BrightCyan => palette.bright_cyan = value,
        PaletteField::BrightWhite => palette.bright_white = value,
        PaletteField::BrightForeground => palette.bright_foreground = value,
        PaletteField::DimBlack => palette.dim_black = value,
        PaletteField::DimRed => palette.dim_red = value,
        PaletteField::DimGreen => palette.dim_green = value,
        PaletteField::DimYellow => palette.dim_yellow = value,
        PaletteField::DimBlue => palette.dim_blue = value,
        PaletteField::DimMagenta => palette.dim_magenta = value,
        PaletteField::DimCyan => palette.dim_cyan = value,
        PaletteField::DimWhite => palette.dim_white = value,
        PaletteField::DimForeground => palette.dim_foreground = value,
        PaletteField::Overlay => palette.overlay = value,
    }
}

/// Palette presets available in the theme editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SettingsPreset {
    /// Default OTTY dark theme.
    OttyDark,
    /// One Dark-inspired palette.
    OneDark,
    /// Solarized Dark-inspired palette.
    SolarizedDark,
    /// Dracula-inspired palette.
    Dracula,
}

impl SettingsPreset {
    /// All built-in theme presets shown in the selector.
    pub(crate) const ALL: [Self; 4] = [
        Self::OttyDark,
        Self::OneDark,
        Self::SolarizedDark,
        Self::Dracula,
    ];

    /// Return the selector label for this preset.
    pub(crate) fn title(&self) -> &'static str {
        match self {
            Self::OttyDark => "OTTY Dark",
            Self::OneDark => "One Dark",
            Self::SolarizedDark => "Solarized Dark",
            Self::Dracula => "Dracula",
        }
    }

    /// Return the palette hex values for this preset.
    pub(crate) fn palette(&self) -> Vec<String> {
        match self {
            Self::OttyDark => default_palette(),
            Self::OneDark => palette_from_hexes([
                "#ABB2BF", "#282C34", "#282C34", "#E06C75", "#98C379",
                "#E5C07B", "#61AFEF", "#C678DD", "#56B6C2", "#ABB2BF",
                "#5C6370", "#FF7A90", "#B5E890", "#FFD98E", "#7DCFFF",
                "#DFA6FF", "#7FE4EA", "#FFFFFF", "#E6EDF7", "#1C2027",
                "#8B434A", "#5E7748", "#8A744A", "#3A6A8F", "#784885",
                "#326B73", "#7B8496", "#7B8496", "#323842",
            ]),
            Self::SolarizedDark => palette_from_hexes([
                "#839496", "#002B36", "#073642", "#DC322F", "#859900",
                "#B58900", "#268BD2", "#D33682", "#2AA198", "#EEE8D5",
                "#002B36", "#CB4B16", "#586E75", "#657B83", "#839496",
                "#6C71C4", "#93A1A1", "#FDF6E3", "#FDF6E3", "#001F27",
                "#8A231F", "#536100", "#715900", "#175785", "#7E1F4D",
                "#1A625D", "#A6A093", "#657B83", "#073642",
            ]),
            Self::Dracula => palette_from_hexes([
                "#F8F8F2", "#282A36", "#21222C", "#FF5555", "#50FA7B",
                "#F1FA8C", "#BD93F9", "#FF79C6", "#8BE9FD", "#F8F8F2",
                "#6272A4", "#FF6E6E", "#69FF94", "#FFFFA5", "#D6ACFF",
                "#FF92DF", "#A4FFFF", "#FFFFFF", "#FFFFFF", "#191A21",
                "#9B3333", "#318F49", "#8F9554", "#715892", "#99508A",
                "#5395A1", "#B7B7B2", "#B7B7B2", "#303341",
            ]),
        }
    }

    /// Detect a built-in preset that matches the given palette values.
    pub(crate) fn from_palette(values: &[String]) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|preset| preset.palette().as_slice() == values)
    }
}

impl fmt::Display for SettingsPreset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.title())
    }
}

fn palette_from_hexes(values: [&str; 29]) -> Vec<String> {
    values.into_iter().map(String::from).collect()
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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{SettingsData, SettingsPreset, is_valid_hex_color};

    #[test]
    fn given_valid_palette_when_from_json_then_palette_is_loaded() {
        let value = json!({
            "theme": {
                "palette": ["#112233", "#223344"]
            }
        });

        let settings = SettingsData::from_json(&value);

        assert_eq!(settings.theme.palette, vec!["#112233", "#223344"]);
    }

    #[test]
    fn given_invalid_palette_when_from_json_then_defaults_are_used() {
        let defaults = SettingsData::default();
        let value = json!({
            "theme": {
                "palette": ["#112233", "not-a-color"]
            }
        });

        let settings = SettingsData::from_json(&value);

        assert_eq!(settings.theme.palette, defaults.theme.palette);
    }

    #[test]
    fn given_invalid_fields_when_normalized_then_defaults_are_applied() {
        let defaults = SettingsData::default();
        let mut settings = SettingsData::default();
        settings.terminal.shell = String::from("   ");
        settings.terminal.editor = String::new();
        settings.theme.palette = vec![String::from("bad-value")];

        let normalized = settings.normalized();

        assert_eq!(normalized.terminal.shell, defaults.terminal.shell);
        assert_eq!(normalized.terminal.editor, defaults.terminal.editor);
        assert_eq!(normalized.theme.palette, defaults.theme.palette);
    }

    #[test]
    fn given_hex_color_value_when_validated_then_result_matches_format() {
        assert!(is_valid_hex_color("#aBc123"));
        assert!(!is_valid_hex_color("#12345"));
        assert!(!is_valid_hex_color("123456"));
    }

    #[test]
    fn given_known_preset_palette_when_detected_then_matching_preset_is_found()
    {
        let palette = SettingsPreset::Dracula.palette();

        let preset = SettingsPreset::from_palette(&palette);

        assert_eq!(preset, Some(SettingsPreset::Dracula));
    }
}
