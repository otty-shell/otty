//! Color tokens of the OTTY design system, grouped by role.
//!
//! Each theme is one fixed [`UiPalette`]. Tokens that the design system
//! defines as an alias of a terminal color (for example `accent` is
//! `{ansi-blue}`) hold that color's value for the theme. Nothing is computed
//! at run time.

use iced::{Color, Shadow, Vector, color};

/// Every color and shadow a widget may use, named after its role.
///
/// # Example
///
/// ```
/// use iced::widget::container;
/// use otty_ui_kit::palette::UiPalette;
///
/// let palette = UiPalette::otty_dark();
/// let style = container::Style::default()
///     .background(palette.surface_panel())
///     .color(palette.text_body());
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct UiPalette {
    surface_chrome: Color,
    surface_panel: Color,
    surface_canvas: Color,
    surface_header: Color,
    surface_raised: Color,
    surface_overlay: Color,
    surface_hover: Color,
    surface_selected: Color,
    surface_error: Color,
    surface_danger: Color,
    line: Color,
    text_strong: Color,
    text_body: Color,
    text_muted: Color,
    text_faint: Color,
    text_danger: Color,
    accent: Color,
    accent_hover: Color,
    on_accent: Color,
    focus: Color,
    status_ok: Color,
    status_warn: Color,
    status_error: Color,
    target_local: Color,
    target_ssh: Color,
    target_staging: Color,
    target_prod: Color,
    diff_add: Color,
    diff_del: Color,
    scrim: Color,
    shadow_popover: Shadow,
    shadow_sticky: Shadow,
}

impl UiPalette {
    /// `surface-chrome`: title bar, widget rail, status bar.
    pub const fn surface_chrome(&self) -> Color {
        self.surface_chrome
    }

    /// `surface-panel`: left and right panels, tab strip, bottom dock header.
    pub const fn surface_panel(&self) -> Color {
        self.surface_panel
    }

    /// `surface-canvas`: main area (terminal panes, forms, tables). Equals the
    /// terminal background.
    pub const fn surface_canvas(&self) -> Color {
        self.surface_canvas
    }

    /// `surface-header`: focused pane header, sticky block header, hovered
    /// block.
    pub const fn surface_header(&self) -> Color {
        self.surface_header
    }

    /// `surface-raised`: inputs, chips, kbd, search box, row hover inside
    /// panels.
    pub const fn surface_raised(&self) -> Color {
        self.surface_raised
    }

    /// `surface-overlay`: menus, pickers, Ctrl K palette, block action bar.
    /// Always used with [`Self::shadow_popover`].
    pub const fn surface_overlay(&self) -> Color {
        self.surface_overlay
    }

    /// `surface-hover`: hovered or keyboard-active item inside an overlay;
    /// selected segment.
    pub const fn surface_hover(&self) -> Color {
        self.surface_hover
    }

    /// `surface-selected`: selected row in a list, tree or table. Text on it
    /// is [`Self::text_strong`].
    pub const fn surface_selected(&self) -> Color {
        self.surface_selected
    }

    /// `surface-error`: background of a failed command block.
    pub const fn surface_error(&self) -> Color {
        self.surface_error
    }

    /// `surface-danger`: protected-target banner. Text on it is
    /// [`Self::text_danger`].
    pub const fn surface_danger(&self) -> Color {
        self.surface_danger
    }

    /// `line`: 1px dividers between panes, blocks, table rows and under the
    /// title bar.
    pub const fn line(&self) -> Color {
        self.line
    }

    /// `text-strong`: titles, active tab, selected row, commands in block
    /// headers.
    pub const fn text_strong(&self) -> Color {
        self.text_strong
    }

    /// `text-body`: default UI and terminal text. Equals the terminal
    /// foreground.
    pub const fn text_body(&self) -> Color {
        self.text_body
    }

    /// `text-muted`: hints, meta (paths, durations), section labels, inactive
    /// tabs.
    pub const fn text_muted(&self) -> Color {
        self.text_muted
    }

    /// `text-faint`: idle rail and toolbar icons only. Not for text.
    pub const fn text_faint(&self) -> Color {
        self.text_faint
    }

    /// `text-danger`: text on [`Self::surface_danger`].
    pub const fn text_danger(&self) -> Color {
        self.text_danger
    }

    /// `accent`: primary button, active rail item, active tab top line,
    /// links. Equals the terminal blue.
    pub const fn accent(&self) -> Color {
        self.accent
    }

    /// `accent-hover`: hovered primary button and links.
    pub const fn accent_hover(&self) -> Color {
        self.accent_hover
    }

    /// `on-accent`: text and icons on [`Self::accent`].
    pub const fn on_accent(&self) -> Color {
        self.on_accent
    }

    /// `focus`: 2px keyboard focus ring, offset 1px. Equals the terminal
    /// blue.
    pub const fn focus(&self) -> Color {
        self.focus
    }

    /// `status-ok`: exit 0, running, done. Always with a word or a check
    /// mark. Equals the terminal green.
    pub const fn status_ok(&self) -> Color {
        self.status_ok
    }

    /// `status-warn`: waiting for you, pending, unsaved changes. Equals the
    /// terminal yellow.
    pub const fn status_warn(&self) -> Color {
        self.status_warn
    }

    /// `status-error`: non-zero exit, field errors. Always with a word or a
    /// cross. Equals the terminal red.
    pub const fn status_error(&self) -> Color {
        self.status_error
    }

    /// `target-local`: dot for this computer. Equals the terminal cyan.
    pub const fn target_local(&self) -> Color {
        self.target_local
    }

    /// `target-ssh`: dot for SSH hosts. Equals the terminal magenta.
    pub const fn target_ssh(&self) -> Color {
        self.target_ssh
    }

    /// `target-staging`: dot for non-protected clusters. Equals the terminal
    /// yellow.
    pub const fn target_staging(&self) -> Color {
        self.target_staging
    }

    /// `target-prod`: dot for protected targets, always with the word
    /// protected or a banner. Equals the terminal red.
    pub const fn target_prod(&self) -> Color {
        self.target_prod
    }

    /// `diff-add`: added line background in Changes.
    pub const fn diff_add(&self) -> Color {
        self.diff_add
    }

    /// `diff-del`: removed line background in Changes.
    pub const fn diff_del(&self) -> Color {
        self.diff_del
    }

    /// `scrim`: dims the window behind a modal dialog.
    pub const fn scrim(&self) -> Color {
        self.scrim
    }

    /// `shadow-popover`: menus, pickers, palette. The only drop shadow for
    /// overlays.
    pub const fn shadow_popover(&self) -> Shadow {
        self.shadow_popover
    }

    /// `shadow-sticky`: sticky block header while output scrolls under it.
    pub const fn shadow_sticky(&self) -> Shadow {
        self.shadow_sticky
    }

    /// OTTY Dark, the default theme.
    pub const fn otty_dark() -> Self {
        Self {
            surface_chrome: color!(0x0f1115),
            surface_panel: color!(0x111319),
            surface_canvas: color!(0x161822),
            surface_header: color!(0x1a1d29),
            surface_raised: color!(0x1c1f2b),
            surface_overlay: color!(0x232530),
            surface_hover: color!(0x2e3140),
            surface_selected: color!(0x1f2940),
            surface_error: color!(0x1e1a24),
            surface_danger: color!(0x2a1e26),
            line: color!(0x232530),
            text_strong: color!(0xeceff4),
            text_body: color!(0xc0c5ce),
            text_muted: color!(0x939ba8),
            text_faint: color!(0x6b7280),
            text_danger: color!(0xf1c3c7),
            accent: color!(0x4fa6ed),
            accent_hover: color!(0x7dbdf2),
            on_accent: color!(0x0f1115),
            focus: color!(0x4fa6ed),
            status_ok: color!(0x98c379),
            status_warn: color!(0xe5c07b),
            status_error: color!(0xe06c75),
            target_local: color!(0x56b6c2),
            target_ssh: color!(0xc678dd),
            target_staging: color!(0xe5c07b),
            target_prod: color!(0xe06c75),
            diff_add: color!(0x1f2c26),
            diff_del: color!(0x2e1f27),
            scrim: color!(0x000000, 0.5),
            // 0 14px 34px rgba(0,0,0,0.5)
            shadow_popover: Shadow {
                color: color!(0x000000, 0.5),
                offset: Vector::new(0.0, 14.0),
                blur_radius: 34.0,
            },
            // 0 6px 12px rgba(0,0,0,0.25)
            shadow_sticky: Shadow {
                color: color!(0x000000, 0.25),
                offset: Vector::new(0.0, 6.0),
                blur_radius: 12.0,
            },
        }
    }

    /// OTTY Light.
    pub const fn otty_light() -> Self {
        Self {
            surface_chrome: color!(0xe9ebf0),
            surface_panel: color!(0xeff1f5),
            surface_canvas: color!(0xfafbfc),
            surface_header: color!(0xf1f3f7),
            surface_raised: color!(0xe4e7ee),
            surface_overlay: color!(0xffffff),
            surface_hover: color!(0xe0e4eb),
            surface_selected: color!(0xd6e4f5),
            surface_error: color!(0xfbedee),
            surface_danger: color!(0xfbe7e9),
            line: color!(0xd5d9e2),
            text_strong: color!(0x11141c),
            text_body: color!(0x2a2f3a),
            text_muted: color!(0x5d6575),
            text_faint: color!(0x7a8292),
            text_danger: color!(0x8f1f2b),
            accent: color!(0x1a64b0),
            accent_hover: color!(0x154f8c),
            on_accent: color!(0xffffff),
            focus: color!(0x1a64b0),
            status_ok: color!(0x377028),
            status_warn: color!(0x835d00),
            status_error: color!(0xb23641),
            target_local: color!(0x0b6e79),
            target_ssh: color!(0x8e3fae),
            target_staging: color!(0x835d00),
            target_prod: color!(0xb23641),
            diff_add: color!(0xe3f1dd),
            diff_del: color!(0xf8e1e3),
            scrim: color!(0x11141c, 0.16),
            // 0 12px 30px rgba(17,20,28,0.16)
            shadow_popover: Shadow {
                color: color!(0x11141c, 0.16),
                offset: Vector::new(0.0, 12.0),
                blur_radius: 30.0,
            },
            // 0 4px 10px rgba(17,20,28,0.08)
            shadow_sticky: Shadow {
                color: color!(0x11141c, 0.08),
                offset: Vector::new(0.0, 4.0),
                blur_radius: 10.0,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use iced::{Color, Shadow, Vector};

    use super::UiPalette;
    use crate::contrast::contrast_ratio;

    /// WCAG 2.1 AA minimum for normal text.
    const TEXT_MIN: f32 = 4.5;
    /// WCAG 2.1 AA minimum for focus rings and other non-text marks.
    const NON_TEXT_MIN: f32 = 3.0;

    /// Hex values of `tokens.json` for OTTY Dark, aliases resolved.
    const OTTY_DARK_HEX: [(&str, &str); 29] = [
        ("surface-chrome", "#0f1115"),
        ("surface-panel", "#111319"),
        ("surface-canvas", "#161822"),
        ("surface-header", "#1a1d29"),
        ("surface-raised", "#1c1f2b"),
        ("surface-overlay", "#232530"),
        ("surface-hover", "#2e3140"),
        ("surface-selected", "#1f2940"),
        ("surface-error", "#1e1a24"),
        ("surface-danger", "#2a1e26"),
        ("line", "#232530"),
        ("text-strong", "#eceff4"),
        ("text-body", "#c0c5ce"),
        ("text-muted", "#939ba8"),
        ("text-faint", "#6b7280"),
        ("text-danger", "#f1c3c7"),
        ("accent", "#4fa6ed"),
        ("accent-hover", "#7dbdf2"),
        ("on-accent", "#0f1115"),
        ("focus", "#4fa6ed"),
        ("status-ok", "#98c379"),
        ("status-warn", "#e5c07b"),
        ("status-error", "#e06c75"),
        ("target-local", "#56b6c2"),
        ("target-ssh", "#c678dd"),
        ("target-staging", "#e5c07b"),
        ("target-prod", "#e06c75"),
        ("diff-add", "#1f2c26"),
        ("diff-del", "#2e1f27"),
    ];

    /// Hex values of `tokens.json` for OTTY Light, aliases resolved.
    const OTTY_LIGHT_HEX: [(&str, &str); 29] = [
        ("surface-chrome", "#e9ebf0"),
        ("surface-panel", "#eff1f5"),
        ("surface-canvas", "#fafbfc"),
        ("surface-header", "#f1f3f7"),
        ("surface-raised", "#e4e7ee"),
        ("surface-overlay", "#ffffff"),
        ("surface-hover", "#e0e4eb"),
        ("surface-selected", "#d6e4f5"),
        ("surface-error", "#fbedee"),
        ("surface-danger", "#fbe7e9"),
        ("line", "#d5d9e2"),
        ("text-strong", "#11141c"),
        ("text-body", "#2a2f3a"),
        ("text-muted", "#5d6575"),
        ("text-faint", "#7a8292"),
        ("text-danger", "#8f1f2b"),
        ("accent", "#1a64b0"),
        ("accent-hover", "#154f8c"),
        ("on-accent", "#ffffff"),
        ("focus", "#1a64b0"),
        ("status-ok", "#377028"),
        ("status-warn", "#835d00"),
        ("status-error", "#b23641"),
        ("target-local", "#0b6e79"),
        ("target-ssh", "#8e3fae"),
        ("target-staging", "#835d00"),
        ("target-prod", "#b23641"),
        ("diff-add", "#e3f1dd"),
        ("diff-del", "#f8e1e3"),
    ];

    /// Every color token of the palette, named as in `tokens.json`.
    fn color_tokens(palette: &UiPalette) -> [(&'static str, Color); 29] {
        [
            ("surface-chrome", palette.surface_chrome()),
            ("surface-panel", palette.surface_panel()),
            ("surface-canvas", palette.surface_canvas()),
            ("surface-header", palette.surface_header()),
            ("surface-raised", palette.surface_raised()),
            ("surface-overlay", palette.surface_overlay()),
            ("surface-hover", palette.surface_hover()),
            ("surface-selected", palette.surface_selected()),
            ("surface-error", palette.surface_error()),
            ("surface-danger", palette.surface_danger()),
            ("line", palette.line()),
            ("text-strong", palette.text_strong()),
            ("text-body", palette.text_body()),
            ("text-muted", palette.text_muted()),
            ("text-faint", palette.text_faint()),
            ("text-danger", palette.text_danger()),
            ("accent", palette.accent()),
            ("accent-hover", palette.accent_hover()),
            ("on-accent", palette.on_accent()),
            ("focus", palette.focus()),
            ("status-ok", palette.status_ok()),
            ("status-warn", palette.status_warn()),
            ("status-error", palette.status_error()),
            ("target-local", palette.target_local()),
            ("target-ssh", palette.target_ssh()),
            ("target-staging", palette.target_staging()),
            ("target-prod", palette.target_prod()),
            ("diff-add", palette.diff_add()),
            ("diff-del", palette.diff_del()),
        ]
    }

    /// Every `surface-*` token of the palette.
    fn surfaces(palette: &UiPalette) -> [(&'static str, Color); 10] {
        [
            ("surface-chrome", palette.surface_chrome()),
            ("surface-panel", palette.surface_panel()),
            ("surface-canvas", palette.surface_canvas()),
            ("surface-header", palette.surface_header()),
            ("surface-raised", palette.surface_raised()),
            ("surface-overlay", palette.surface_overlay()),
            ("surface-hover", palette.surface_hover()),
            ("surface-selected", palette.surface_selected()),
            ("surface-error", palette.surface_error()),
            ("surface-danger", palette.surface_danger()),
        ]
    }

    fn themes() -> [(&'static str, UiPalette); 2] {
        [
            ("OTTY Dark", UiPalette::otty_dark()),
            ("OTTY Light", UiPalette::otty_light()),
        ]
    }

    fn assert_tokens_match(
        theme: &str,
        palette: &UiPalette,
        expected: &[(&str, &str)],
    ) {
        let actual = color_tokens(palette);
        assert_eq!(actual.len(), expected.len(), "{theme}: token count");

        for ((name, color), (expected_name, hex)) in actual.iter().zip(expected)
        {
            let expected_color: Color =
                hex.parse().expect("token table holds valid hex colors");

            assert_eq!(name, expected_name, "{theme}: token order");
            assert_eq!(
                color.into_rgba8(),
                expected_color.into_rgba8(),
                "{theme}: {name} must be {hex}"
            );
        }
    }

    fn assert_contrast_at_least(
        theme: &str,
        (fg_name, fg): (&str, Color),
        (bg_name, bg): (&str, Color),
        min: f32,
    ) {
        let ratio = contrast_ratio(fg, bg);

        assert!(
            ratio >= min,
            "{theme}: {fg_name} on {bg_name} is {ratio:.2}:1, needs {min}:1"
        );
    }

    #[test]
    fn given_otty_dark_when_tokens_read_then_each_equals_tokens_json() {
        assert_tokens_match(
            "OTTY Dark",
            &UiPalette::otty_dark(),
            &OTTY_DARK_HEX,
        );
    }

    #[test]
    fn given_otty_light_when_tokens_read_then_each_equals_tokens_json() {
        assert_tokens_match(
            "OTTY Light",
            &UiPalette::otty_light(),
            &OTTY_LIGHT_HEX,
        );
    }

    #[test]
    fn given_otty_dark_when_shadows_read_then_they_match_tokens_json() {
        let palette = UiPalette::otty_dark();

        assert_eq!(
            palette.shadow_popover(),
            Shadow {
                color: Color::from_rgba8(0, 0, 0, 0.5),
                offset: Vector::new(0.0, 14.0),
                blur_radius: 34.0,
            }
        );
        assert_eq!(
            palette.shadow_sticky(),
            Shadow {
                color: Color::from_rgba8(0, 0, 0, 0.25),
                offset: Vector::new(0.0, 6.0),
                blur_radius: 12.0,
            }
        );
    }

    #[test]
    fn given_otty_light_when_shadows_read_then_they_match_tokens_json() {
        let palette = UiPalette::otty_light();

        assert_eq!(
            palette.shadow_popover(),
            Shadow {
                color: Color::from_rgba8(17, 20, 28, 0.16),
                offset: Vector::new(0.0, 12.0),
                blur_radius: 30.0,
            }
        );
        assert_eq!(
            palette.shadow_sticky(),
            Shadow {
                color: Color::from_rgba8(17, 20, 28, 0.08),
                offset: Vector::new(0.0, 4.0),
                blur_radius: 10.0,
            }
        );
    }

    #[test]
    fn given_each_theme_when_scrim_read_then_it_matches_tokens_json() {
        assert_eq!(
            UiPalette::otty_dark().scrim(),
            Color::from_rgba8(0, 0, 0, 0.5)
        );
        assert_eq!(
            UiPalette::otty_light().scrim(),
            Color::from_rgba8(17, 20, 28, 0.16)
        );
    }

    #[test]
    fn given_each_theme_when_text_on_main_surfaces_then_reaches_4_5() {
        for (theme, palette) in themes() {
            let texts = [
                ("text-strong", palette.text_strong()),
                ("text-body", palette.text_body()),
                ("text-muted", palette.text_muted()),
            ];
            let main_surfaces = [
                ("surface-chrome", palette.surface_chrome()),
                ("surface-panel", palette.surface_panel()),
                ("surface-canvas", palette.surface_canvas()),
                ("surface-overlay", palette.surface_overlay()),
            ];

            for text in texts {
                for surface in main_surfaces {
                    assert_contrast_at_least(theme, text, surface, TEXT_MIN);
                }
            }
        }
    }

    #[test]
    fn given_each_theme_when_focus_on_every_surface_then_reaches_3() {
        for (theme, palette) in themes() {
            let focus = ("focus", palette.focus());

            for surface in surfaces(&palette) {
                assert_contrast_at_least(theme, focus, surface, NON_TEXT_MIN);
            }
        }
    }

    #[test]
    fn given_each_theme_when_on_accent_on_accent_then_reaches_4_5() {
        for (theme, palette) in themes() {
            assert_contrast_at_least(
                theme,
                ("on-accent", palette.on_accent()),
                ("accent", palette.accent()),
                TEXT_MIN,
            );
        }
    }

    #[test]
    fn given_each_theme_when_muted_text_on_selected_row_then_reaches_4_5() {
        for (theme, palette) in themes() {
            assert_contrast_at_least(
                theme,
                ("text-muted", palette.text_muted()),
                ("surface-selected", palette.surface_selected()),
                TEXT_MIN,
            );
        }
    }

    /// Status colors are text-sized marks. They must reach 4.5:1 on chrome,
    /// raised and selected surfaces. They are never placed on
    /// `surface-hover` (only the hovered menu item and the selected segment
    /// use it), so that surface is not checked.
    #[test]
    fn given_each_theme_when_status_on_chrome_raised_and_selected_then_reaches_4_5()
     {
        for (theme, palette) in themes() {
            for status in [
                ("status-ok", palette.status_ok()),
                ("status-warn", palette.status_warn()),
                ("status-error", palette.status_error()),
            ] {
                for surface in [
                    ("surface-chrome", palette.surface_chrome()),
                    ("surface-raised", palette.surface_raised()),
                    ("surface-selected", palette.surface_selected()),
                ] {
                    assert_contrast_at_least(theme, status, surface, TEXT_MIN);
                }
            }
        }
    }

    /// Target dots are non-text graphics (WCAG 1.4.11), so 3:1 is enough.
    #[test]
    fn given_each_theme_when_target_dot_on_every_surface_then_reaches_3() {
        for (theme, palette) in themes() {
            for target in [
                ("target-local", palette.target_local()),
                ("target-ssh", palette.target_ssh()),
                ("target-staging", palette.target_staging()),
                ("target-prod", palette.target_prod()),
            ] {
                for surface in [
                    ("surface-chrome", palette.surface_chrome()),
                    ("surface-panel", palette.surface_panel()),
                    ("surface-canvas", palette.surface_canvas()),
                    ("surface-raised", palette.surface_raised()),
                    ("surface-overlay", palette.surface_overlay()),
                    ("surface-hover", palette.surface_hover()),
                    ("surface-selected", palette.surface_selected()),
                ] {
                    assert_contrast_at_least(
                        theme,
                        target,
                        surface,
                        NON_TEXT_MIN,
                    );
                }
            }
        }
    }

    #[test]
    fn given_each_theme_when_muted_text_on_hover_row_then_reaches_4_5() {
        for (theme, palette) in themes() {
            assert_contrast_at_least(
                theme,
                ("text-muted", palette.text_muted()),
                ("surface-hover", palette.surface_hover()),
                TEXT_MIN,
            );
        }
    }
}
