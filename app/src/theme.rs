use std::fmt;

use iced::theme::Palette;
use iced::{Color, Theme};
use otty_ui_kit::palette::UiPalette;
use otty_ui_term::{ColorPalette as TerminalColorPalette, parse_hex_color};

/// Raw string-based color palette used for serialization and settings.
#[derive(Debug, Clone)]
pub struct ColorPalette {
    pub foreground: String,
    pub background: String,
    pub black: String,
    pub red: String,
    pub green: String,
    pub yellow: String,
    pub blue: String,
    pub magenta: String,
    pub cyan: String,
    pub white: String,
    pub bright_black: String,
    pub bright_red: String,
    pub bright_green: String,
    pub bright_yellow: String,
    pub bright_blue: String,
    pub bright_magenta: String,
    pub bright_cyan: String,
    pub bright_white: String,
    pub bright_foreground: String,
    pub dim_black: String,
    pub dim_red: String,
    pub dim_green: String,
    pub dim_yellow: String,
    pub dim_blue: String,
    pub dim_magenta: String,
    pub dim_cyan: String,
    pub dim_white: String,
    pub dim_foreground: String,
    pub overlay: String,
}

impl ColorPalette {
    /// Terminal colors of OTTY Dark, the default theme.
    pub fn otty_dark() -> Self {
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
            overlay: String::from("#232530"),
        }
    }

    /// Terminal colors of OTTY Light. The design system fixes the
    /// background, foreground, the six base hues and `bright_black`; the
    /// other colors keep text readable on the light background, and the dim
    /// colors are 60% of the normal color mixed with 40% background.
    pub fn otty_light() -> Self {
        Self {
            foreground: String::from("#2a2f3a"),
            background: String::from("#fafbfc"),
            black: String::from("#2a2f3a"),
            red: String::from("#c23b47"),
            green: String::from("#3c7a2b"),
            yellow: String::from("#8a6200"),
            blue: String::from("#1a64b0"),
            magenta: String::from("#8e3fae"),
            cyan: String::from("#0b6e79"),
            white: String::from("#5d6575"),
            bright_black: String::from("#8a92a2"),
            bright_red: String::from("#cb3e4b"),
            bright_green: String::from("#2e7d32"),
            bright_yellow: String::from("#946800"),
            bright_blue: String::from("#1f6fc5"),
            bright_magenta: String::from("#9c47bf"),
            bright_cyan: String::from("#0a7581"),
            bright_white: String::from("#7a8292"),
            bright_foreground: String::from("#11141c"),
            dim_foreground: String::from("#7a8292"),
            dim_black: String::from("#e9ebf0"),
            dim_red: String::from("#d8888f"),
            dim_green: String::from("#88ae7f"),
            dim_yellow: String::from("#b79f65"),
            dim_blue: String::from("#74a0ce"),
            dim_magenta: String::from("#b98acd"),
            dim_cyan: String::from("#6ba6ad"),
            dim_white: String::from("#9ca1ab"),
            overlay: String::from("#ffffff"),
        }
    }
}

impl From<ColorPalette> for TerminalColorPalette {
    fn from(p: ColorPalette) -> Self {
        Self {
            foreground: p.foreground,
            background: p.background,
            black: p.black,
            red: p.red,
            green: p.green,
            yellow: p.yellow,
            blue: p.blue,
            magenta: p.magenta,
            cyan: p.cyan,
            white: p.white,
            bright_black: p.bright_black,
            bright_red: p.bright_red,
            bright_green: p.bright_green,
            bright_yellow: p.bright_yellow,
            bright_blue: p.bright_blue,
            bright_magenta: p.bright_magenta,
            bright_cyan: p.bright_cyan,
            bright_white: p.bright_white,
            bright_foreground: Some(p.bright_foreground.clone()),
            dim_foreground: p.dim_foreground,
            dim_black: p.dim_black,
            dim_red: p.dim_red,
            dim_green: p.dim_green,
            dim_yellow: p.dim_yellow,
            dim_blue: p.dim_blue,
            dim_magenta: p.dim_magenta,
            dim_cyan: p.dim_cyan,
            dim_white: p.dim_white,
            block_highlight: p.bright_foreground,
        }
    }
}

/// Parsed color palette ready for iced rendering.
#[derive(Debug, Clone)]
pub struct IcedColorPalette {
    pub foreground: Color,
    pub background: Color,
    pub black: Color,
    pub red: Color,
    pub green: Color,
    pub yellow: Color,
    pub blue: Color,
    pub magenta: Color,
    pub cyan: Color,
    pub white: Color,
    pub bright_black: Color,
    pub bright_red: Color,
    pub bright_green: Color,
    pub bright_yellow: Color,
    pub bright_blue: Color,
    pub bright_magenta: Color,
    pub bright_cyan: Color,
    pub bright_white: Color,
    pub bright_foreground: Color,
    pub dim_black: Color,
    pub dim_red: Color,
    pub dim_green: Color,
    pub dim_yellow: Color,
    pub dim_blue: Color,
    pub dim_magenta: Color,
    pub dim_cyan: Color,
    pub dim_white: Color,
    pub dim_foreground: Color,
    pub overlay: Color,
}

impl From<&ColorPalette> for IcedColorPalette {
    fn from(p: &ColorPalette) -> Self {
        Self {
            foreground: parse_hex_color(&p.foreground),
            background: parse_hex_color(&p.background),
            black: parse_hex_color(&p.black),
            red: parse_hex_color(&p.red),
            green: parse_hex_color(&p.green),
            yellow: parse_hex_color(&p.yellow),
            blue: parse_hex_color(&p.blue),
            magenta: parse_hex_color(&p.magenta),
            cyan: parse_hex_color(&p.cyan),
            white: parse_hex_color(&p.white),
            bright_black: parse_hex_color(&p.bright_black),
            bright_red: parse_hex_color(&p.bright_red),
            bright_green: parse_hex_color(&p.bright_green),
            bright_yellow: parse_hex_color(&p.bright_yellow),
            bright_blue: parse_hex_color(&p.bright_blue),
            bright_magenta: parse_hex_color(&p.bright_magenta),
            bright_cyan: parse_hex_color(&p.bright_cyan),
            bright_white: parse_hex_color(&p.bright_white),
            bright_foreground: parse_hex_color(&p.bright_foreground),
            dim_black: parse_hex_color(&p.dim_black),
            dim_red: parse_hex_color(&p.dim_red),
            dim_green: parse_hex_color(&p.dim_green),
            dim_yellow: parse_hex_color(&p.dim_yellow),
            dim_blue: parse_hex_color(&p.dim_blue),
            dim_magenta: parse_hex_color(&p.dim_magenta),
            dim_cyan: parse_hex_color(&p.dim_cyan),
            dim_white: parse_hex_color(&p.dim_white),
            dim_foreground: parse_hex_color(&p.dim_foreground),
            overlay: parse_hex_color(&p.overlay),
        }
    }
}

/// One of the two fixed themes of the app. Each theme is a complete set of
/// terminal colors and UI color tokens; nothing is computed at run time.
///
/// # Example
///
/// ```ignore
/// let theme = BuiltinTheme::from_name("otty-light").unwrap_or_default();
/// let app_theme = AppTheme::new(theme);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BuiltinTheme {
    /// OTTY Dark, the default theme.
    #[default]
    OttyDark,
    /// OTTY Light.
    OttyLight,
}

impl BuiltinTheme {
    /// Every theme, in the order shown by the theme selector.
    pub const ALL: [Self; 2] = [Self::OttyDark, Self::OttyLight];

    /// Name stored in the settings file.
    pub fn name(self) -> &'static str {
        match self {
            Self::OttyLight => "otty-light",
            _ => "otty-dark",
        }
    }

    /// Product name shown in the theme selector. Theme names are not
    /// translated.
    pub fn title(self) -> &'static str {
        match self {
            Self::OttyLight => "OTTY Light",
            _ => "OTTY Dark",
        }
    }

    /// Terminal colors of the theme.
    pub fn terminal_palette(self) -> ColorPalette {
        match self {
            Self::OttyLight => ColorPalette::otty_light(),
            _ => ColorPalette::otty_dark(),
        }
    }

    /// UI color tokens of the theme.
    pub fn ui_palette(self) -> UiPalette {
        match self {
            Self::OttyLight => UiPalette::otty_light(),
            _ => UiPalette::otty_dark(),
        }
    }

    /// Find the theme with the given stored name; `None` for any other
    /// value.
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|theme| theme.name() == name)
    }
}

impl fmt::Display for BuiltinTheme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.title())
    }
}

/// Optional overrides for widget/component styling.
#[derive(Debug, Default, Clone, Copy)]
pub struct StyleOverrides {
    pub background: Option<Color>,
    pub foreground: Option<Color>,
    pub border_radius: Option<f32>,
}

/// Global application theme shared between UI and terminal.
#[derive(Debug, Clone)]
pub struct AppTheme {
    theme: BuiltinTheme,
    raw_palette: ColorPalette,
    iced_palette: IcedColorPalette,
    ui: UiPalette,
}

impl AppTheme {
    /// Build terminal-compatible palette from the raw colors.
    pub fn terminal_palette(&self) -> TerminalColorPalette {
        TerminalColorPalette::from(self.raw_palette.clone())
    }

    /// Return the parsed iced color palette.
    pub fn iced_palette(&self) -> &IcedColorPalette {
        &self.iced_palette
    }

    /// Return the UI color tokens of the theme.
    pub fn ui(&self) -> &UiPalette {
        &self.ui
    }

    /// Build the application theme for one of the built-in themes.
    pub fn new(theme: BuiltinTheme) -> Self {
        let raw_palette = theme.terminal_palette();
        let iced_palette = IcedColorPalette::from(&raw_palette);

        Self {
            theme,
            raw_palette,
            iced_palette,
            ui: theme.ui_palette(),
        }
    }
}

impl Default for AppTheme {
    fn default() -> Self {
        Self::new(BuiltinTheme::default())
    }
}

impl From<&AppTheme> for Theme {
    fn from(value: &AppTheme) -> Self {
        let palette = &value.iced_palette;
        let palette = Palette {
            background: palette.background,
            text: palette.foreground,
            primary: palette.blue,
            success: palette.green,
            danger: palette.red,
            warning: palette.yellow,
        };

        Theme::custom(value.theme.title().to_string(), palette)
    }
}

/// Theme props passed through the view tree for consistent styling.
#[derive(Debug, Clone, Copy)]
pub struct ThemeProps<'a> {
    pub theme: &'a AppTheme,
    pub overrides: Option<StyleOverrides>,
}

impl<'a> ThemeProps<'a> {
    /// Create theme props with no overrides.
    pub fn new(theme: &'a AppTheme) -> Self {
        Self {
            theme,
            overrides: None,
        }
    }
}

/// Holds the theme the whole window is drawn with.
#[derive(Debug, Clone)]
pub struct ThemeManager {
    current: AppTheme,
}

impl ThemeManager {
    /// Return the current application theme.
    pub fn current(&self) -> &AppTheme {
        &self.current
    }

    /// Create a theme manager with the default theme.
    pub fn new() -> Self {
        Self {
            current: AppTheme::default(),
        }
    }

    /// Build an iced theme from the current palette.
    pub fn iced_theme(&self) -> Theme {
        Theme::from(&self.current)
    }

    /// Switch the window and terminal colors to `theme`.
    pub fn set_theme(&mut self, theme: BuiltinTheme) {
        self.current = AppTheme::new(theme);
    }
}

#[cfg(test)]
mod tests {
    use iced::{Color, Theme};
    use otty_ui_kit::contrast::contrast_ratio;
    use otty_ui_kit::palette::UiPalette;
    use otty_ui_term::parse_hex_color;

    use super::{AppTheme, BuiltinTheme, ColorPalette, ThemeManager};

    /// WCAG 2.1 AA minimum for normal text.
    const TEXT_MIN: f32 = 4.5;
    /// WCAG 2.1 AA minimum for non-text marks such as ghost text.
    const NON_TEXT_MIN: f32 = 3.0;

    #[test]
    fn given_builtin_themes_when_listed_then_only_otty_dark_and_light_exist() {
        assert_eq!(
            BuiltinTheme::ALL,
            [BuiltinTheme::OttyDark, BuiltinTheme::OttyLight]
        );
        assert_eq!(BuiltinTheme::default(), BuiltinTheme::OttyDark);
    }

    #[test]
    fn given_builtin_themes_when_named_then_stored_names_and_titles_match() {
        assert_eq!(BuiltinTheme::OttyDark.name(), "otty-dark");
        assert_eq!(BuiltinTheme::OttyLight.name(), "otty-light");
        assert_eq!(BuiltinTheme::OttyDark.title(), "OTTY Dark");
        assert_eq!(BuiltinTheme::OttyLight.title(), "OTTY Light");
    }

    #[test]
    fn given_stored_names_when_parsed_then_only_known_names_match() {
        for theme in BuiltinTheme::ALL {
            assert_eq!(BuiltinTheme::from_name(theme.name()), Some(theme));
        }
        assert_eq!(BuiltinTheme::from_name("dracula"), None);
        assert_eq!(BuiltinTheme::from_name(""), None);
    }

    #[test]
    fn given_each_theme_when_ui_aliases_read_then_they_equal_terminal_colors() {
        for theme in BuiltinTheme::ALL {
            let terminal = theme.terminal_palette();
            let ui = theme.ui_palette();

            let aliases = [
                ("surface-canvas", ui.surface_canvas(), &terminal.background),
                ("text-body", ui.text_body(), &terminal.foreground),
                ("accent", ui.accent(), &terminal.blue),
                ("focus", ui.focus(), &terminal.blue),
                ("status-ok", ui.status_ok(), &terminal.green),
                ("status-warn", ui.status_warn(), &terminal.yellow),
                ("status-error", ui.status_error(), &terminal.red),
                ("target-local", ui.target_local(), &terminal.cyan),
                ("target-ssh", ui.target_ssh(), &terminal.magenta),
                ("target-staging", ui.target_staging(), &terminal.yellow),
                ("target-prod", ui.target_prod(), &terminal.red),
            ];

            for (token, ui_color, terminal_hex) in aliases {
                assert_eq!(
                    ui_color,
                    parse_hex_color(terminal_hex),
                    "{} {token} differs from the terminal color",
                    theme.title()
                );
            }
        }
    }

    #[test]
    fn given_otty_light_when_text_colors_checked_then_they_reach_text_contrast()
    {
        let palette = ColorPalette::otty_light();
        let background = parse_hex_color(&palette.background);

        let text_colors = [
            ("foreground", &palette.foreground),
            ("bright_foreground", &palette.bright_foreground),
            ("black", &palette.black),
            ("red", &palette.red),
            ("green", &palette.green),
            ("yellow", &palette.yellow),
            ("blue", &palette.blue),
            ("magenta", &palette.magenta),
            ("cyan", &palette.cyan),
            ("white", &palette.white),
            ("bright_red", &palette.bright_red),
            ("bright_green", &palette.bright_green),
            ("bright_yellow", &palette.bright_yellow),
            ("bright_blue", &palette.bright_blue),
            ("bright_magenta", &palette.bright_magenta),
            ("bright_cyan", &palette.bright_cyan),
        ];

        for (field, hex) in text_colors {
            let ratio = contrast_ratio(parse_hex_color(hex), background);
            assert!(ratio >= TEXT_MIN, "{field} contrast was {ratio}");
        }
    }

    #[test]
    fn given_otty_light_when_grey_brights_checked_then_they_stay_readable() {
        let palette = ColorPalette::otty_light();
        let background = parse_hex_color(&palette.background);

        for (field, hex) in [
            ("bright_black", &palette.bright_black),
            ("bright_white", &palette.bright_white),
        ] {
            let ratio = contrast_ratio(parse_hex_color(hex), background);
            assert!(ratio >= NON_TEXT_MIN, "{field} contrast was {ratio}");
        }
    }

    #[test]
    fn given_otty_light_when_dim_colors_checked_then_they_mix_normal_and_background()
     {
        let palette = ColorPalette::otty_light();

        let dim_pairs = [
            ("dim_red", &palette.dim_red, &palette.red),
            ("dim_green", &palette.dim_green, &palette.green),
            ("dim_yellow", &palette.dim_yellow, &palette.yellow),
            ("dim_blue", &palette.dim_blue, &palette.blue),
            ("dim_magenta", &palette.dim_magenta, &palette.magenta),
            ("dim_cyan", &palette.dim_cyan, &palette.cyan),
            ("dim_white", &palette.dim_white, &palette.white),
        ];

        for (field, dim, normal) in dim_pairs {
            assert_eq!(
                rgb(dim),
                mix_with_background(rgb(normal), rgb(&palette.background)),
                "{field} is not 60% of the normal color and 40% background"
            );
        }
    }

    #[test]
    fn given_otty_light_when_ui_facing_fields_read_then_they_mirror_ui_roles() {
        let palette = ColorPalette::otty_light();
        let ui = UiPalette::otty_light();

        assert_eq!(
            parse_hex_color(&palette.bright_foreground),
            ui.text_strong()
        );
        assert_eq!(parse_hex_color(&palette.dim_foreground), ui.text_faint());
        assert_eq!(parse_hex_color(&palette.dim_black), ui.surface_chrome());
        assert_eq!(parse_hex_color(&palette.overlay), ui.surface_overlay());
    }

    #[test]
    fn given_default_app_theme_when_ui_palette_read_then_it_is_otty_dark() {
        let app_theme = AppTheme::default();

        assert_eq!(app_theme.ui(), &UiPalette::otty_dark());
        assert_eq!(
            app_theme.terminal_palette().background,
            ColorPalette::otty_dark().background
        );
    }

    #[test]
    fn given_light_theme_when_app_theme_built_then_both_palettes_are_light() {
        let app_theme = AppTheme::new(BuiltinTheme::OttyLight);

        assert_eq!(app_theme.ui(), &UiPalette::otty_light());
        assert_eq!(app_theme.terminal_palette().background, "#fafbfc");
        assert_eq!(
            app_theme.iced_palette().background,
            Color::from_rgb8(0xfa, 0xfb, 0xfc)
        );
    }

    #[test]
    fn given_theme_manager_when_theme_set_then_current_theme_switches() {
        let mut manager = ThemeManager::new();

        manager.set_theme(BuiltinTheme::OttyLight);

        assert_eq!(manager.current().ui(), &UiPalette::otty_light());

        manager.set_theme(BuiltinTheme::OttyDark);

        assert_eq!(manager.current().ui(), &UiPalette::otty_dark());
    }

    #[test]
    fn given_app_theme_when_converted_to_iced_then_primary_uses_accent_blue() {
        let app_theme = AppTheme::new(BuiltinTheme::OttyLight);

        let theme = Theme::from(&app_theme);

        assert_eq!(theme.palette().primary, app_theme.iced_palette().blue);
    }

    /// Read `#rrggbb` into its three channels.
    fn rgb(hex: &str) -> [u8; 3] {
        let channel = |range: std::ops::Range<usize>| {
            u8::from_str_radix(&hex[range], 16)
                .unwrap_or_else(|_| panic!("invalid hex color {hex}"))
        };

        [channel(1..3), channel(3..5), channel(5..7)]
    }

    /// 60% of `color` plus 40% of `background`, rounded per channel.
    fn mix_with_background(color: [u8; 3], background: [u8; 3]) -> [u8; 3] {
        let mix = |index: usize| {
            (0.6 * f32::from(color[index]) + 0.4 * f32::from(background[index]))
                .round() as u8
        };

        [mix(0), mix(1), mix(2)]
    }
}
