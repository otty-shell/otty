use gpui::{FontStyle, FontWeight};
use otty_libterm::escape::{Color, Rgb, StdColor};
use otty_libterm::surface::Colors;
use otty_ui_term_gpui::{
    BellPolicy, BlockUiMode, ConfigError, ContextMenuPolicy, LinkPolicy,
    TerminalAppearance, TerminalBehavior, TerminalBindings, TerminalColor,
    TerminalConfig, TerminalFont, TerminalTheme,
};

#[test]
fn rejects_invalid_font_metrics() {
    assert!(matches!(
        TerminalFont::try_new(
            "monospace",
            std::iter::empty::<&str>(),
            0.0,
            1.2,
        ),
        Err(ConfigError::InvalidFontSize)
    ));
    assert!(matches!(
        TerminalFont::try_new(
            "monospace",
            std::iter::empty::<&str>(),
            14.0,
            f32::NAN,
        ),
        Err(ConfigError::InvalidLineHeight)
    ));
}

#[test]
fn rejects_invalid_appearance_and_scroll_values() {
    assert!(matches!(
        TerminalAppearance::try_new(-1.0, 1.0, 8.0, 2.0),
        Err(ConfigError::InvalidPadding)
    ));
    assert!(matches!(
        TerminalBehavior::try_new(0.0),
        Err(ConfigError::InvalidScrollMultiplier)
    ));
}

#[test]
fn parses_hex_without_panicking() {
    let color = TerminalColor::from_hex("#12abef").expect("valid color");
    assert_eq!(color.rgb(), (0x12, 0xab, 0xef));
    assert!(matches!(
        TerminalColor::from_hex("not-a-color"),
        Err(ConfigError::InvalidColor(_))
    ));
}

#[test]
fn default_config_is_valid_and_embeddable() {
    let config = TerminalConfig::default();

    assert_eq!(config.font().family(), "monospace");
    assert!(config.appearance().padding() >= 8.0);
    assert!(config.behavior().focus_on_click());
}

#[test]
fn theme_and_behavior_support_runtime_variants() {
    let accent = TerminalColor::from_rgb(0x66, 0xcc, 0xff);
    let palette = otty_ui_term_gpui::ColorPalette::default()
        .with_background(TerminalColor::from_rgb(0x10, 0x12, 0x18))
        .with_foreground(TerminalColor::from_rgb(0xee, 0xee, 0xee));
    let theme = otty_ui_term_gpui::TerminalTheme::new(palette)
        .with_border(accent, accent)
        .with_selection(accent, TerminalColor::from_rgb(0, 0, 0));
    let behavior = TerminalBehavior::default()
        .with_scroll_multiplier(2.0)
        .expect("valid multiplier");

    assert_eq!(theme.focused_border(), accent);
    assert_eq!(behavior.scroll_multiplier(), 2.0);
}

#[test]
fn config_builder_replaces_only_requested_parts() {
    let theme = TerminalTheme::new(
        otty_ui_term_gpui::ColorPalette::default()
            .with_background(TerminalColor::from_rgb(1, 2, 3)),
    );
    let font = TerminalFont::monospace(16.0).expect("valid font");
    let appearance =
        TerminalAppearance::try_new(5.0, 1.0, 4.0, 2.0).expect("valid frame");
    let behavior = TerminalBehavior::try_new(1.5).expect("valid behavior");
    let bindings = TerminalBindings::new();

    let config = TerminalConfig::builder()
        .theme(theme.clone())
        .font(font.clone())
        .appearance(appearance)
        .behavior(behavior.clone())
        .bindings(bindings.clone())
        .build()
        .expect("validated parts build a config");

    assert_eq!(config.theme(), &theme);
    assert_eq!(config.font(), &font);
    assert_eq!(config.appearance(), &appearance);
    assert_eq!(config.behavior(), &behavior);
    assert_eq!(config.bindings(), &bindings);
}

#[test]
fn terminal_font_disables_contextual_ligatures_for_grid_alignment() {
    let font = TerminalFont::default().gpui_font();

    assert_eq!(font.features.is_calt_enabled(), Some(false));
}

#[test]
fn font_and_behavior_builders_preserve_all_runtime_settings() {
    let font = TerminalFont::try_new(
        "Iosevka",
        ["Symbols Nerd Font", "Noto Color Emoji"],
        15.0,
        1.4,
    )
    .expect("valid font")
    .with_weight(FontWeight::BOLD)
    .with_style(FontStyle::Italic);
    let behavior = TerminalBehavior::default()
        .with_focus_on_click(false)
        .with_copy_on_select(true)
        .with_middle_click_paste(false)
        .with_link_policy(LinkPolicy::OpenAndEmit)
        .with_bell_policy(BellPolicy::SystemAndEmit)
        .with_context_menu_policy(ContextMenuPolicy::Disabled)
        .with_block_ui_mode(BlockUiMode::ExternalOverlay);

    assert_eq!(font.family(), "Iosevka");
    assert_eq!(font.fallbacks().len(), 2);
    assert_eq!(font.size(), 15.0);
    assert_eq!(font.line_height(), 1.4);
    assert_eq!(font.weight(), FontWeight::BOLD);
    assert_eq!(font.style(), FontStyle::Italic);
    assert!(font.gpui_font().fallbacks.is_some());
    assert!(!behavior.focus_on_click());
    assert!(behavior.copy_on_select());
    assert!(!behavior.middle_click_paste());
    assert_eq!(behavior.link_policy(), LinkPolicy::OpenAndEmit);
    assert_eq!(behavior.bell_policy(), BellPolicy::SystemAndEmit);
    assert_eq!(behavior.context_menu_policy(), ContextMenuPolicy::Disabled);
    assert_eq!(behavior.block_ui_mode(), BlockUiMode::ExternalOverlay);
}

#[test]
fn palette_resolves_standard_indexed_truecolor_and_dynamic_colors() {
    let palette = otty_ui_term_gpui::ColorPalette::default();
    let mut dynamic = Colors::default();
    dynamic[StdColor::Foreground] = Some(Rgb { r: 1, g: 2, b: 3 });

    assert_eq!(
        palette.resolve(Color::TrueColor(Rgb { r: 4, g: 5, b: 6 }), &dynamic,),
        TerminalColor::from_rgb(4, 5, 6)
    );
    assert_eq!(
        palette.resolve(Color::Std(StdColor::Foreground), &dynamic),
        TerminalColor::from_rgb(1, 2, 3)
    );
    assert_eq!(
        palette.resolve(Color::Indexed(16), &dynamic),
        TerminalColor::from_rgb(0, 0, 0)
    );
    assert_eq!(
        palette.resolve(Color::Indexed(231), &dynamic),
        TerminalColor::from_rgb(255, 255, 255)
    );
    assert_eq!(
        palette.resolve(Color::Indexed(232), &dynamic),
        TerminalColor::from_rgb(8, 8, 8)
    );
    assert_eq!(
        palette.resolve(Color::Indexed(255), &dynamic),
        TerminalColor::from_rgb(238, 238, 238)
    );

    for color in [
        StdColor::Background,
        StdColor::Cursor,
        StdColor::Black,
        StdColor::Red,
        StdColor::Green,
        StdColor::Yellow,
        StdColor::Blue,
        StdColor::Magenta,
        StdColor::Cyan,
        StdColor::White,
        StdColor::BrightBlack,
        StdColor::BrightRed,
        StdColor::BrightGreen,
        StdColor::BrightYellow,
        StdColor::BrightBlue,
        StdColor::BrightMagenta,
        StdColor::BrightCyan,
        StdColor::BrightWhite,
        StdColor::BrightForeground,
        StdColor::DimForeground,
        StdColor::DimBlack,
        StdColor::DimRed,
        StdColor::DimGreen,
        StdColor::DimYellow,
        StdColor::DimBlue,
        StdColor::DimMagenta,
        StdColor::DimCyan,
        StdColor::DimWhite,
    ] {
        let resolved = palette.resolve(Color::Std(color), &dynamic);
        assert_eq!(resolved.hsla().a, 1.0);
    }
}

#[test]
fn palette_and_theme_setters_replace_each_paint_role() {
    let foreground = TerminalColor::from_rgb(1, 2, 3);
    let background = TerminalColor::from_rgb(4, 5, 6);
    let cursor = TerminalColor::from_rgb(7, 8, 9);
    let highlight = TerminalColor::from_rgb(10, 11, 12);
    let palette = otty_ui_term_gpui::ColorPalette::default()
        .with_foreground(foreground)
        .with_background(background)
        .with_cursor(cursor);
    let theme = TerminalTheme::new(palette)
        .with_selection(highlight, foreground)
        .with_border(background, cursor)
        .with_block_highlight(highlight);

    assert_eq!(theme.palette().foreground(), foreground);
    assert_eq!(theme.palette().background(), background);
    assert_eq!(theme.palette().cursor(), cursor);
    assert_eq!(theme.selection_background(), highlight);
    assert_eq!(theme.selection_foreground(), foreground);
    assert_eq!(theme.border(), background);
    assert_eq!(theme.focused_border(), cursor);
    assert_eq!(theme.block_highlight(), highlight);
}
