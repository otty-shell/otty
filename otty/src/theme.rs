use iced::theme::Palette;
use iced::{Color, Theme};
use otty_ui_term::{ColorPalette as TerminalColorPalette, parse_hex_color};
use serde::Serialize;

/// 自适应混合色：等价 CSS `color-mix(in srgb, fg P%, bg)`。
/// 用前景色按百分比混合背景色，生成跟随主题的半透明态（hover/选中/高亮）。
pub(crate) fn mix_color(
    foreground: Color,
    background: Color,
    percent: f32,
) -> Color {
    Color::from_rgba(
        foreground.r * percent + background.r * (1.0 - percent),
        foreground.g * percent + background.g * (1.0 - percent),
        foreground.b * percent + background.b * (1.0 - percent),
        background.a,
    )
}

/// Terminal ANSI/ECMA color configuration stored in settings.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct TerminalColorConfig {
    pub(crate) foreground: String,
    pub(crate) background: String,
    pub(crate) black: String,
    pub(crate) red: String,
    pub(crate) green: String,
    pub(crate) yellow: String,
    pub(crate) blue: String,
    pub(crate) magenta: String,
    pub(crate) cyan: String,
    pub(crate) white: String,
    pub(crate) bright_black: String,
    pub(crate) bright_red: String,
    pub(crate) bright_green: String,
    pub(crate) bright_yellow: String,
    pub(crate) bright_blue: String,
    pub(crate) bright_magenta: String,
    pub(crate) bright_cyan: String,
    pub(crate) bright_white: String,
    pub(crate) bright_foreground: String,
    pub(crate) dim_black: String,
    pub(crate) dim_red: String,
    pub(crate) dim_green: String,
    pub(crate) dim_yellow: String,
    pub(crate) dim_blue: String,
    pub(crate) dim_magenta: String,
    pub(crate) dim_cyan: String,
    pub(crate) dim_white: String,
    pub(crate) dim_foreground: String,
}

impl Default for TerminalColorConfig {
    fn default() -> Self {
        Self {
            foreground: String::from("#C0C5CE"),
            background: String::from("#161822"),
            black: String::from("#161822"),
            red: String::from("#E06C75"),
            green: String::from("#98C379"),
            yellow: String::from("#E5C07B"),
            blue: String::from("#4FA6ED"),
            magenta: String::from("#C678DD"),
            cyan: String::from("#56B6C2"),
            white: String::from("#D1D5DB"),
            bright_black: String::from("#4F5666"),
            bright_red: String::from("#FF5C8D"),
            bright_green: String::from("#5AF78E"),
            bright_yellow: String::from("#F3E488"),
            bright_blue: String::from("#5FD8FF"),
            bright_magenta: String::from("#FF4081"),
            bright_cyan: String::from("#2CD4C8"),
            bright_white: String::from("#FFFFFF"),
            bright_foreground: String::from("#ECEFF4"),
            dim_foreground: String::from("#6B7280"),
            dim_black: String::from("#0F1115"),
            dim_red: String::from("#8F3F4A"),
            dim_green: String::from("#587545"),
            dim_yellow: String::from("#8A734A"),
            dim_blue: String::from("#2F638F"),
            dim_magenta: String::from("#784885"),
            dim_cyan: String::from("#326B73"),
            dim_white: String::from("#6C7385"),
        }
    }
}

/// Semantic UI color configuration stored in settings.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct UiColorConfig {
    pub(crate) foreground: String,
    pub(crate) muted_foreground: String,
    pub(crate) surface_background: String,
    pub(crate) surface_border: String,
    pub(crate) chrome_background: String,
    pub(crate) overlay: String,
    pub(crate) sidebar_background: String,
    pub(crate) activity_bar_background: String,
    pub(crate) accent: String,
    pub(crate) danger: String,
    pub(crate) info: String,
    pub(crate) success: String,
    pub(crate) warning: String,
    pub(crate) selection_background: String,
    pub(crate) emphasis_foreground: String,
}

impl Default for UiColorConfig {
    fn default() -> Self {
        Self {
            foreground: String::from("#C0C5CE"),
            muted_foreground: String::from("#6B7280"),
            surface_background: String::from("#161822"),
            surface_border: String::from("#2A2D37"),
            chrome_background: String::from("#0F1115"),
            overlay: String::from("#232530"),
            sidebar_background: String::from("#232530"),
            activity_bar_background: String::from("#1B1D23"),
            accent: String::from("#C45A6D"),
            danger: String::from("#E06C75"),
            info: String::from("#4FA6ED"),
            success: String::from("#98C379"),
            warning: String::from("#E5C07B"),
            selection_background: String::from("#2F638F"),
            emphasis_foreground: String::from("#ECEFF4"),
        }
    }
}

/// Named design token bundle shared by settings, terminal, and UI rendering.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct DesignTokens {
    pub(crate) terminal: TerminalColorConfig,
    pub(crate) ui: UiColorConfig,
}

impl DesignTokens {
    /// Parse a `theme` JSON object into named design tokens.
    pub(crate) fn from_json_theme(value: &serde_json::Value) -> Self {
        let defaults = Self::default();
        let terminal_value = value.get("terminal");
        let ui_value = value.get("ui");

        Self {
            terminal: TerminalColorConfig {
                foreground: read_color_field(
                    terminal_value,
                    "foreground",
                    &defaults.terminal.foreground,
                ),
                background: read_color_field(
                    terminal_value,
                    "background",
                    &defaults.terminal.background,
                ),
                black: read_color_field(
                    terminal_value,
                    "black",
                    &defaults.terminal.black,
                ),
                red: read_color_field(
                    terminal_value,
                    "red",
                    &defaults.terminal.red,
                ),
                green: read_color_field(
                    terminal_value,
                    "green",
                    &defaults.terminal.green,
                ),
                yellow: read_color_field(
                    terminal_value,
                    "yellow",
                    &defaults.terminal.yellow,
                ),
                blue: read_color_field(
                    terminal_value,
                    "blue",
                    &defaults.terminal.blue,
                ),
                magenta: read_color_field(
                    terminal_value,
                    "magenta",
                    &defaults.terminal.magenta,
                ),
                cyan: read_color_field(
                    terminal_value,
                    "cyan",
                    &defaults.terminal.cyan,
                ),
                white: read_color_field(
                    terminal_value,
                    "white",
                    &defaults.terminal.white,
                ),
                bright_black: read_color_field(
                    terminal_value,
                    "bright_black",
                    &defaults.terminal.bright_black,
                ),
                bright_red: read_color_field(
                    terminal_value,
                    "bright_red",
                    &defaults.terminal.bright_red,
                ),
                bright_green: read_color_field(
                    terminal_value,
                    "bright_green",
                    &defaults.terminal.bright_green,
                ),
                bright_yellow: read_color_field(
                    terminal_value,
                    "bright_yellow",
                    &defaults.terminal.bright_yellow,
                ),
                bright_blue: read_color_field(
                    terminal_value,
                    "bright_blue",
                    &defaults.terminal.bright_blue,
                ),
                bright_magenta: read_color_field(
                    terminal_value,
                    "bright_magenta",
                    &defaults.terminal.bright_magenta,
                ),
                bright_cyan: read_color_field(
                    terminal_value,
                    "bright_cyan",
                    &defaults.terminal.bright_cyan,
                ),
                bright_white: read_color_field(
                    terminal_value,
                    "bright_white",
                    &defaults.terminal.bright_white,
                ),
                bright_foreground: read_color_field(
                    terminal_value,
                    "bright_foreground",
                    &defaults.terminal.bright_foreground,
                ),
                dim_black: read_color_field(
                    terminal_value,
                    "dim_black",
                    &defaults.terminal.dim_black,
                ),
                dim_red: read_color_field(
                    terminal_value,
                    "dim_red",
                    &defaults.terminal.dim_red,
                ),
                dim_green: read_color_field(
                    terminal_value,
                    "dim_green",
                    &defaults.terminal.dim_green,
                ),
                dim_yellow: read_color_field(
                    terminal_value,
                    "dim_yellow",
                    &defaults.terminal.dim_yellow,
                ),
                dim_blue: read_color_field(
                    terminal_value,
                    "dim_blue",
                    &defaults.terminal.dim_blue,
                ),
                dim_magenta: read_color_field(
                    terminal_value,
                    "dim_magenta",
                    &defaults.terminal.dim_magenta,
                ),
                dim_cyan: read_color_field(
                    terminal_value,
                    "dim_cyan",
                    &defaults.terminal.dim_cyan,
                ),
                dim_white: read_color_field(
                    terminal_value,
                    "dim_white",
                    &defaults.terminal.dim_white,
                ),
                dim_foreground: read_color_field(
                    terminal_value,
                    "dim_foreground",
                    &defaults.terminal.dim_foreground,
                ),
            },
            ui: UiColorConfig {
                foreground: read_color_field(
                    ui_value,
                    "foreground",
                    &defaults.ui.foreground,
                ),
                muted_foreground: read_color_field(
                    ui_value,
                    "muted_foreground",
                    &defaults.ui.muted_foreground,
                ),
                surface_background: read_color_field(
                    ui_value,
                    "surface_background",
                    &defaults.ui.surface_background,
                ),
                surface_border: read_color_field(
                    ui_value,
                    "surface_border",
                    &defaults.ui.surface_border,
                ),
                chrome_background: read_color_field(
                    ui_value,
                    "chrome_background",
                    &defaults.ui.chrome_background,
                ),
                overlay: read_color_field(
                    ui_value,
                    "overlay",
                    &defaults.ui.overlay,
                ),
                sidebar_background: read_color_field(
                    ui_value,
                    "sidebar_background",
                    &defaults.ui.sidebar_background,
                ),
                activity_bar_background: read_color_field(
                    ui_value,
                    "activity_bar_background",
                    &defaults.ui.activity_bar_background,
                ),
                accent: read_color_field(
                    ui_value,
                    "accent",
                    &defaults.ui.accent,
                ),
                danger: read_color_field(
                    ui_value,
                    "danger",
                    &defaults.ui.danger,
                ),
                info: read_color_field(ui_value, "info", &defaults.ui.info),
                success: read_color_field(
                    ui_value,
                    "success",
                    &defaults.ui.success,
                ),
                warning: read_color_field(
                    ui_value,
                    "warning",
                    &defaults.ui.warning,
                ),
                selection_background: read_color_field(
                    ui_value,
                    "selection_background",
                    &defaults.ui.selection_background,
                ),
                emphasis_foreground: read_color_field(
                    ui_value,
                    "emphasis_foreground",
                    &defaults.ui.emphasis_foreground,
                ),
            },
        }
    }

    /// Return a copy with invalid colors replaced by defaults.
    pub(crate) fn normalized(&self) -> Self {
        let defaults = Self::default();

        Self {
            terminal: normalize_terminal(&self.terminal, &defaults.terminal),
            ui: normalize_ui(&self.ui, &defaults.ui),
        }
    }

    /// Project named tokens back into the legacy 32-entry palette order.
    pub(crate) fn legacy_palette(&self) -> Vec<String> {
        vec![
            self.terminal.foreground.clone(),
            self.terminal.background.clone(),
            self.terminal.black.clone(),
            self.terminal.red.clone(),
            self.terminal.green.clone(),
            self.terminal.yellow.clone(),
            self.terminal.blue.clone(),
            self.terminal.magenta.clone(),
            self.terminal.cyan.clone(),
            self.terminal.white.clone(),
            self.terminal.bright_black.clone(),
            self.terminal.bright_red.clone(),
            self.terminal.bright_green.clone(),
            self.terminal.bright_yellow.clone(),
            self.terminal.bright_blue.clone(),
            self.terminal.bright_magenta.clone(),
            self.terminal.bright_cyan.clone(),
            self.terminal.bright_white.clone(),
            self.terminal.bright_foreground.clone(),
            self.terminal.dim_black.clone(),
            self.terminal.dim_red.clone(),
            self.terminal.dim_green.clone(),
            self.terminal.dim_yellow.clone(),
            self.terminal.dim_blue.clone(),
            self.terminal.dim_magenta.clone(),
            self.terminal.dim_cyan.clone(),
            self.terminal.dim_white.clone(),
            self.terminal.dim_foreground.clone(),
            self.ui.overlay.clone(),
            self.ui.sidebar_background.clone(),
            self.ui.activity_bar_background.clone(),
            self.ui.accent.clone(),
        ]
    }

    /// Apply a legacy palette array over the current tokens.
    pub(crate) fn apply_legacy_palette(&mut self, values: &[String]) {
        for (index, value) in values.iter().enumerate() {
            self.apply_legacy_palette_entry(index, value);
        }
    }

    /// Apply one legacy palette index to the matching named field.
    pub(crate) fn apply_legacy_palette_entry(
        &mut self,
        index: usize,
        value: &str,
    ) {
        match index {
            0 => self.terminal.foreground = value.to_string(),
            1 => self.terminal.background = value.to_string(),
            2 => self.terminal.black = value.to_string(),
            3 => self.terminal.red = value.to_string(),
            4 => self.terminal.green = value.to_string(),
            5 => self.terminal.yellow = value.to_string(),
            6 => self.terminal.blue = value.to_string(),
            7 => self.terminal.magenta = value.to_string(),
            8 => self.terminal.cyan = value.to_string(),
            9 => self.terminal.white = value.to_string(),
            10 => self.terminal.bright_black = value.to_string(),
            11 => self.terminal.bright_red = value.to_string(),
            12 => self.terminal.bright_green = value.to_string(),
            13 => self.terminal.bright_yellow = value.to_string(),
            14 => self.terminal.bright_blue = value.to_string(),
            15 => self.terminal.bright_magenta = value.to_string(),
            16 => self.terminal.bright_cyan = value.to_string(),
            17 => self.terminal.bright_white = value.to_string(),
            18 => self.terminal.bright_foreground = value.to_string(),
            19 => self.terminal.dim_black = value.to_string(),
            20 => self.terminal.dim_red = value.to_string(),
            21 => self.terminal.dim_green = value.to_string(),
            22 => self.terminal.dim_yellow = value.to_string(),
            23 => self.terminal.dim_blue = value.to_string(),
            24 => self.terminal.dim_magenta = value.to_string(),
            25 => self.terminal.dim_cyan = value.to_string(),
            26 => self.terminal.dim_white = value.to_string(),
            27 => self.terminal.dim_foreground = value.to_string(),
            28 => self.ui.overlay = value.to_string(),
            29 => self.ui.sidebar_background = value.to_string(),
            30 => self.ui.activity_bar_background = value.to_string(),
            31 => self.ui.accent = value.to_string(),
            _ => {},
        }

        self.sync_ui_from_legacy();
    }

    fn sync_ui_from_legacy(&mut self) {
        self.ui.foreground = self.terminal.foreground.clone();
        self.ui.muted_foreground = self.terminal.dim_foreground.clone();
        self.ui.surface_background = self.terminal.background.clone();
        if is_valid_hex_color(&self.terminal.foreground)
            && is_valid_hex_color(&self.terminal.background)
        {
            self.ui.surface_border = mix_hex_color(
                &self.terminal.foreground,
                &self.terminal.background,
                0.12,
            );
        }
        self.ui.chrome_background = self.terminal.dim_black.clone();
        self.ui.danger = self.terminal.red.clone();
        self.ui.info = self.terminal.blue.clone();
        self.ui.success = self.terminal.green.clone();
        self.ui.warning = self.terminal.yellow.clone();
        self.ui.selection_background = self.terminal.dim_blue.clone();
        self.ui.emphasis_foreground = self.terminal.bright_foreground.clone();
    }
}

fn normalize_terminal(
    value: &TerminalColorConfig,
    defaults: &TerminalColorConfig,
) -> TerminalColorConfig {
    TerminalColorConfig {
        foreground: normalize_color(&value.foreground, &defaults.foreground),
        background: normalize_color(&value.background, &defaults.background),
        black: normalize_color(&value.black, &defaults.black),
        red: normalize_color(&value.red, &defaults.red),
        green: normalize_color(&value.green, &defaults.green),
        yellow: normalize_color(&value.yellow, &defaults.yellow),
        blue: normalize_color(&value.blue, &defaults.blue),
        magenta: normalize_color(&value.magenta, &defaults.magenta),
        cyan: normalize_color(&value.cyan, &defaults.cyan),
        white: normalize_color(&value.white, &defaults.white),
        bright_black: normalize_color(
            &value.bright_black,
            &defaults.bright_black,
        ),
        bright_red: normalize_color(&value.bright_red, &defaults.bright_red),
        bright_green: normalize_color(
            &value.bright_green,
            &defaults.bright_green,
        ),
        bright_yellow: normalize_color(
            &value.bright_yellow,
            &defaults.bright_yellow,
        ),
        bright_blue: normalize_color(&value.bright_blue, &defaults.bright_blue),
        bright_magenta: normalize_color(
            &value.bright_magenta,
            &defaults.bright_magenta,
        ),
        bright_cyan: normalize_color(&value.bright_cyan, &defaults.bright_cyan),
        bright_white: normalize_color(
            &value.bright_white,
            &defaults.bright_white,
        ),
        bright_foreground: normalize_color(
            &value.bright_foreground,
            &defaults.bright_foreground,
        ),
        dim_black: normalize_color(&value.dim_black, &defaults.dim_black),
        dim_red: normalize_color(&value.dim_red, &defaults.dim_red),
        dim_green: normalize_color(&value.dim_green, &defaults.dim_green),
        dim_yellow: normalize_color(&value.dim_yellow, &defaults.dim_yellow),
        dim_blue: normalize_color(&value.dim_blue, &defaults.dim_blue),
        dim_magenta: normalize_color(&value.dim_magenta, &defaults.dim_magenta),
        dim_cyan: normalize_color(&value.dim_cyan, &defaults.dim_cyan),
        dim_white: normalize_color(&value.dim_white, &defaults.dim_white),
        dim_foreground: normalize_color(
            &value.dim_foreground,
            &defaults.dim_foreground,
        ),
    }
}

fn normalize_ui(
    value: &UiColorConfig,
    defaults: &UiColorConfig,
) -> UiColorConfig {
    UiColorConfig {
        foreground: normalize_color(&value.foreground, &defaults.foreground),
        muted_foreground: normalize_color(
            &value.muted_foreground,
            &defaults.muted_foreground,
        ),
        surface_background: normalize_color(
            &value.surface_background,
            &defaults.surface_background,
        ),
        surface_border: normalize_color(
            &value.surface_border,
            &defaults.surface_border,
        ),
        chrome_background: normalize_color(
            &value.chrome_background,
            &defaults.chrome_background,
        ),
        overlay: normalize_color(&value.overlay, &defaults.overlay),
        sidebar_background: normalize_color(
            &value.sidebar_background,
            &defaults.sidebar_background,
        ),
        activity_bar_background: normalize_color(
            &value.activity_bar_background,
            &defaults.activity_bar_background,
        ),
        accent: normalize_color(&value.accent, &defaults.accent),
        danger: normalize_color(&value.danger, &defaults.danger),
        info: normalize_color(&value.info, &defaults.info),
        success: normalize_color(&value.success, &defaults.success),
        warning: normalize_color(&value.warning, &defaults.warning),
        selection_background: normalize_color(
            &value.selection_background,
            &defaults.selection_background,
        ),
        emphasis_foreground: normalize_color(
            &value.emphasis_foreground,
            &defaults.emphasis_foreground,
        ),
    }
}

fn normalize_color(value: &str, default: &str) -> String {
    if is_valid_hex_color(value) {
        value.to_string()
    } else {
        default.to_string()
    }
}

fn mix_hex_color(foreground: &str, background: &str, percent: f32) -> String {
    let mixed = mix_color(
        parse_hex_color(foreground),
        parse_hex_color(background),
        percent,
    );
    format!(
        "#{:02X}{:02X}{:02X}",
        (mixed.r * 255.0).round() as u8,
        (mixed.g * 255.0).round() as u8,
        (mixed.b * 255.0).round() as u8,
    )
}

fn read_color_field(
    value: Option<&serde_json::Value>,
    key: &str,
    default: &str,
) -> String {
    let Some(value) = value
        .and_then(|value| value.get(key))
        .and_then(serde_json::Value::as_str)
    else {
        return default.to_string();
    };

    normalize_color(value, default)
}

fn is_valid_hex_color(value: &str) -> bool {
    let mut chars = value.chars();
    if chars.next() != Some('#') || value.len() != 7 {
        return false;
    }
    chars.all(|ch| ch.is_ascii_hexdigit())
}

/// Parsed semantic UI color palette ready for iced rendering.
#[derive(Debug, Clone)]
pub(crate) struct UiColorPalette {
    pub foreground: Color,
    pub muted_foreground: Color,
    pub surface_background: Color,
    pub surface_foreground: Color,
    pub surface_border: Color,
    pub chrome_background: Color,
    pub chrome_foreground: Color,
    pub overlay: Color,
    pub sidebar_background: Color,
    pub sidebar_foreground: Color,
    pub activity_bar_background: Color,
    pub activity_bar_foreground: Color,
    pub accent: Color,
    pub danger: Color,
    pub info: Color,
    pub success: Color,
    pub warning: Color,
    pub selection_background: Color,
    pub selection_foreground: Color,
    pub emphasis_foreground: Color,
    pub separator: Color,
}

impl From<&UiColorConfig> for UiColorPalette {
    fn from(config: &UiColorConfig) -> Self {
        let foreground = parse_hex_color(&config.foreground);
        let chrome_background = parse_hex_color(&config.chrome_background);

        Self {
            foreground,
            muted_foreground: parse_hex_color(&config.muted_foreground),
            surface_background: parse_hex_color(&config.surface_background),
            surface_foreground: foreground,
            surface_border: parse_hex_color(&config.surface_border),
            chrome_background,
            chrome_foreground: foreground,
            overlay: parse_hex_color(&config.overlay),
            sidebar_background: parse_hex_color(&config.sidebar_background),
            sidebar_foreground: foreground,
            activity_bar_background: parse_hex_color(
                &config.activity_bar_background,
            ),
            activity_bar_foreground: foreground,
            accent: parse_hex_color(&config.accent),
            danger: parse_hex_color(&config.danger),
            info: parse_hex_color(&config.info),
            success: parse_hex_color(&config.success),
            warning: parse_hex_color(&config.warning),
            selection_background: parse_hex_color(&config.selection_background),
            selection_foreground: chrome_background,
            emphasis_foreground: parse_hex_color(&config.emphasis_foreground),
            separator: parse_hex_color(&config.muted_foreground),
        }
    }
}

impl From<&TerminalColorConfig> for TerminalColorPalette {
    fn from(config: &TerminalColorConfig) -> Self {
        Self {
            foreground: config.foreground.clone(),
            background: config.background.clone(),
            black: config.black.clone(),
            red: config.red.clone(),
            green: config.green.clone(),
            yellow: config.yellow.clone(),
            blue: config.blue.clone(),
            magenta: config.magenta.clone(),
            cyan: config.cyan.clone(),
            white: config.white.clone(),
            bright_black: config.bright_black.clone(),
            bright_red: config.bright_red.clone(),
            bright_green: config.bright_green.clone(),
            bright_yellow: config.bright_yellow.clone(),
            bright_blue: config.bright_blue.clone(),
            bright_magenta: config.bright_magenta.clone(),
            bright_cyan: config.bright_cyan.clone(),
            bright_white: config.bright_white.clone(),
            bright_foreground: Some(config.bright_foreground.clone()),
            dim_foreground: config.dim_foreground.clone(),
            dim_black: config.dim_black.clone(),
            dim_red: config.dim_red.clone(),
            dim_green: config.dim_green.clone(),
            dim_yellow: config.dim_yellow.clone(),
            dim_blue: config.dim_blue.clone(),
            dim_magenta: config.dim_magenta.clone(),
            dim_cyan: config.dim_cyan.clone(),
            dim_white: config.dim_white.clone(),
            block_highlight: config.bright_foreground.clone(),
        }
    }
}

/// Global application theme shared between UI and terminal.
#[derive(Debug, Clone)]
pub(crate) struct AppTheme {
    id: String,
    design_tokens: DesignTokens,
    ui_palette: UiColorPalette,
}

impl Default for AppTheme {
    fn default() -> Self {
        let design_tokens = DesignTokens::default();
        Self::from_design_tokens(String::from("default"), design_tokens)
    }
}

impl From<&AppTheme> for Theme {
    fn from(value: &AppTheme) -> Self {
        let palette = value.ui_palette();
        let palette = Palette {
            background: palette.surface_background,
            text: palette.foreground,
            primary: palette.accent,
            success: palette.success,
            danger: palette.danger,
            warning: palette.warning,
        };

        Theme::custom(value.id.clone(), palette)
    }
}

impl AppTheme {
    /// Build an application theme from named design tokens.
    pub fn from_design_tokens(id: String, design_tokens: DesignTokens) -> Self {
        let ui_palette = UiColorPalette::from(&design_tokens.ui);

        Self {
            id,
            design_tokens,
            ui_palette,
        }
    }

    /// Return the theme identifier.
    pub fn id(&self) -> &String {
        &self.id
    }

    /// Build terminal-compatible palette from the terminal tokens.
    pub fn terminal_palette(&self) -> TerminalColorPalette {
        TerminalColorPalette::from(&self.design_tokens.terminal)
    }

    /// Return the parsed semantic UI color palette.
    pub fn ui_palette(&self) -> &UiColorPalette {
        &self.ui_palette
    }

    /// Return the named design tokens used to build this theme.
    pub fn design_tokens(&self) -> &DesignTokens {
        &self.design_tokens
    }
}

/// Theme props passed through the view tree for consistent styling.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ThemeProps<'a> {
    pub theme: &'a AppTheme,
}

impl<'a> ThemeProps<'a> {
    /// Create theme props.
    pub fn new(theme: &'a AppTheme) -> Self {
        Self { theme }
    }
}

/// Manages the current global theme.
#[derive(Debug, Clone)]
pub(crate) struct ThemeManager {
    current: AppTheme,
}

impl ThemeManager {
    /// Create a theme manager with the default palette.
    pub fn new() -> Self {
        Self {
            current: AppTheme::default(),
        }
    }

    /// Return the current application theme.
    pub fn current(&self) -> &AppTheme {
        &self.current
    }

    /// Build an iced theme from the current palette.
    pub fn iced_theme(&self) -> Theme {
        Theme::from(&self.current)
    }

    /// Replace the current theme with named design tokens.
    pub fn set_design_tokens(&mut self, design_tokens: DesignTokens) {
        self.current =
            AppTheme::from_design_tokens(String::from("custom"), design_tokens);
    }
}

#[cfg(test)]
mod tests {
    use iced::Theme;

    use super::{AppTheme, DesignTokens, mix_hex_color};

    #[test]
    fn given_app_theme_when_converted_to_iced_then_primary_uses_accent() {
        let app_theme = AppTheme::from_design_tokens(
            String::from("custom"),
            DesignTokens::default(),
        );

        let theme = Theme::from(&app_theme);

        assert_eq!(theme.palette().primary, app_theme.ui_palette().accent);
    }

    #[test]
    fn given_legacy_palette_when_applied_then_named_fields_are_updated() {
        let mut tokens = DesignTokens::default();

        tokens.apply_legacy_palette(&[
            String::from("#112233"),
            String::from("#223344"),
            String::from("#334455"),
            String::from("#445566"),
            String::from("#556677"),
            String::from("#667788"),
            String::from("#778899"),
            String::from("#8899AA"),
            String::from("#99AABB"),
            String::from("#AABBCC"),
            String::from("#BBCCDD"),
            String::from("#CCDDEE"),
            String::from("#DDEEFF"),
            String::from("#EEFF00"),
            String::from("#FF0011"),
            String::from("#001122"),
            String::from("#112233"),
            String::from("#223344"),
            String::from("#334455"),
            String::from("#445566"),
            String::from("#556677"),
            String::from("#667788"),
            String::from("#778899"),
            String::from("#8899AA"),
            String::from("#99AABB"),
            String::from("#AABBCC"),
            String::from("#BBCCDD"),
            String::from("#CCDDEE"),
            String::from("#DDEEFF"),
            String::from("#EEFF00"),
            String::from("#FF0011"),
            String::from("#001122"),
        ]);

        assert_eq!(tokens.terminal.foreground, "#112233");
        assert_eq!(tokens.terminal.background, "#223344");
        assert_eq!(tokens.ui.overlay, "#DDEEFF");
        assert_eq!(tokens.ui.sidebar_background, "#EEFF00");
        assert_eq!(tokens.ui.activity_bar_background, "#FF0011");
        assert_eq!(tokens.ui.accent, "#001122");
    }

    #[test]
    fn given_named_tokens_when_projected_to_legacy_then_round_trips() {
        let tokens = DesignTokens::default();
        let projection = tokens.legacy_palette();

        let mut restored = DesignTokens::default();
        restored.apply_legacy_palette(&projection);

        assert_eq!(restored, tokens);
    }

    #[test]
    fn given_legacy_palette_when_applied_then_ui_surface_uses_legacy_background()
     {
        let mut tokens = DesignTokens::default();

        tokens.apply_legacy_palette(&[
            String::from("#3A3132"),
            String::from("#FFF6F8"),
        ]);

        assert_eq!(tokens.ui.foreground, "#3A3132");
        assert_eq!(tokens.ui.surface_background, "#FFF6F8");
        assert_eq!(
            tokens.ui.surface_border,
            mix_hex_color("#3A3132", "#FFF6F8", 0.12)
        );
    }

    #[test]
    fn given_legacy_entry_when_applied_then_ui_semantics_follow_terminal_slot()
    {
        let mut tokens = DesignTokens::default();

        tokens.apply_legacy_palette_entry(3, "#123456");

        assert_eq!(tokens.ui.danger, "#123456");
    }

    #[test]
    fn given_invalid_legacy_colors_when_applied_then_surface_border_stays_default()
     {
        let mut tokens = DesignTokens::default();

        tokens.apply_legacy_palette(&[
            String::from("not-a-color"),
            String::from("#FFF6F8"),
        ]);

        assert_eq!(tokens.ui.foreground, "not-a-color");
        assert_eq!(tokens.ui.surface_border, "#2A2D37");
    }
}
