use serde::Serialize;

use crate::theme::ColorPalette;

const DEFAULT_EDITOR: &str = "nano";
const FALLBACK_SHELL: &str = "/bin/bash";

/// Typed settings payload used for persistence and UI state.
#[derive(Debug, Clone, PartialEq, Serialize, Default)]
pub(crate) struct SettingsData {
    pub(crate) terminal: TerminalSettingsData,
    pub(crate) theme: ThemeSettingsData,
}

/// Terminal-related settings.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct TerminalSettingsData {
    pub(crate) shell: String,
    pub(crate) editor: String,
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
    pub(crate) palette: Vec<String>,
}

impl Default for ThemeSettingsData {
    fn default() -> Self {
        let palette = default_palette();
        Self { palette }
    }
}

impl ThemeSettingsData {
    pub(crate) fn to_color_palette(&self) -> ColorPalette {
        let base = ColorPalette::default();
        apply_palette_overrides(&base, &self.palette)
    }
}

impl SettingsData {
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

pub(crate) fn default_palette() -> Vec<String> {
    palette_from_colors(&ColorPalette::default())
}

pub(crate) fn palette_from_colors(palette: &ColorPalette) -> Vec<String> {
    PALETTE_FIELDS
        .iter()
        .map(|field| palette_value(palette, *field).to_string())
        .collect()
}

pub(crate) fn palette_label(index: usize) -> Option<&'static str> {
    PALETTE_LABELS.get(index).copied()
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

pub(crate) fn is_valid_hex_color(value: &str) -> bool {
    let mut chars = value.chars();
    if chars.next() != Some('#') || value.len() != 7 {
        return false;
    }
    chars.all(|ch| ch.is_ascii_hexdigit())
}

pub(crate) fn is_hex_color_prefix(value: &str) -> bool {
    let mut chars = value.chars();
    if chars.next() != Some('#') || value.len() > 7 {
        return false;
    }
    chars.all(|ch| ch.is_ascii_hexdigit())
}

fn is_palette_valid(values: &[String]) -> bool {
    !values.is_empty() && values.iter().all(|value| is_valid_hex_color(value))
}

fn default_shell() -> String {
    std::env::var("SHELL").unwrap_or_else(|_| FALLBACK_SHELL.to_string())
}

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

const PALETTE_LABELS: [&str; 29] = [
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
