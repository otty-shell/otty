use iced::Font;
use otty_ui_kit::{fonts, typography};

use crate::i18n;

/// Terminal font configuration.
#[derive(Debug, Clone)]
pub(crate) struct TerminalFonts {
    pub(crate) font_type: Font,
    pub(crate) size: f32,
}

impl Default for TerminalFonts {
    fn default() -> Self {
        Self {
            font_type: fonts::MONO,
            size: typography::TERM.size(),
        }
    }
}

/// Font configuration for terminal rendering.
#[derive(Debug, Default, Clone)]
pub(crate) struct FontsConfig {
    pub(crate) terminal: TerminalFonts,
}

/// Tells the font system the language of the active locale. Call it after the
/// locale changes.
pub(crate) fn apply_text_locale() {
    fonts::set_text_locale(i18n::text_locale_tag(
        i18n::current_locale(),
        i18n::detect_system_locale(),
    ));
}

#[cfg(test)]
mod tests {
    use iced::advanced::graphics::text::{Paragraph, font_system};
    use iced::advanced::text::paragraph::Plain;
    use iced::advanced::text::{self, Paragraph as _};
    use iced::alignment::Vertical;
    use iced::widget::text::{LineHeight, Shaping, Wrapping};
    use iced::{Font, Pixels, Size};
    use otty_ui_kit::fonts;

    use super::apply_text_locale;
    use crate::i18n::{self, Locale};
    use crate::testing::alone;

    /// The text of a paragraph, as `iced` lays it out.
    fn text_of(content: &str, font: Font) -> text::Text<&str> {
        text::Text {
            content,
            bounds: Size::new(f32::INFINITY, f32::INFINITY),
            size: Pixels(16.0),
            line_height: LineHeight::Absolute(Pixels(20.0)),
            font,
            align_x: text::Alignment::Default,
            align_y: Vertical::Top,
            shaping: Shaping::Advanced,
            wrapping: Wrapping::None,
        }
    }

    /// The family of the font that draws the first letter of `paragraph`.
    fn family_of(paragraph: &Paragraph) -> String {
        let run = paragraph.buffer().layout_runs().next().expect("a line");
        let glyph = run.glyphs.first().expect("a letter");
        let mut system = font_system().write().expect("font system");
        let face = system.raw().db().face(glyph.font_id).expect("the face");

        face.families[0].0.clone()
    }

    /// Says whether this is a computer where cosmic-text asks for `family` and
    /// has it. The tests below need one and say so when skipped.
    fn asks_for_and_has(family: &str) -> bool {
        let mut system = font_system().write().expect("font system");
        let has_it =
            system.raw().db().faces().any(|face| {
                face.families.iter().any(|(name, _)| name == family)
            });

        if !cfg!(target_os = "linux") || !has_it {
            eprintln!("skipped: no {family} here");
            return false;
        }

        true
    }

    /// Without this, a font listed at another weight is passed over, and Hangul
    /// comes apart into letters on a computer that has the right font.
    #[test]
    fn hangul_stays_whole_in_each_weight_with_the_font_asked_for() {
        if !alone::is_alone(
            "hangul_stays_whole_in_each_weight_with_the_font_asked_for",
        ) {
            return;
        }
        alone::load_kit_fonts();
        if !asks_for_and_has("Noto Sans CJK KR") {
            return;
        }

        for font in
            [fonts::UI, fonts::UI_MEDIUM, fonts::UI_SEMIBOLD, fonts::MONO]
        {
            let paragraph = Paragraph::with_text(text_of("읽", font));

            assert_eq!(family_of(&paragraph), "Noto Sans CJK KR", "{font:?}");
            assert!(paragraph.min_width() < 24.0, "{font:?}");
        }
    }

    /// A name that stays the same through a change of language is laid out
    /// again by `iced`, and takes the shapes of the new language.
    #[test]
    fn han_letters_take_the_shapes_of_the_language_without_a_restart() {
        if !alone::is_alone(
            "han_letters_take_the_shapes_of_the_language_without_a_restart",
        ) {
            return;
        }
        alone::load_kit_fonts();
        if !asks_for_and_has("Noto Sans CJK JP") {
            return;
        }

        let name = "直";
        i18n::set_locale(Locale::Ja);
        apply_text_locale();
        let mut shown = Plain::<Paragraph>::default();
        shown.update(text_of(name, fonts::UI));
        assert_eq!(family_of(shown.raw()), "Noto Sans CJK JP");

        for (locale, family) in [
            (Locale::ZhTw, "Noto Sans CJK TC"),
            (Locale::ZhCn, "Noto Sans CJK SC"),
            (Locale::Ko, "Noto Sans CJK KR"),
            (Locale::Ja, "Noto Sans CJK JP"),
        ] {
            i18n::set_locale(locale);
            apply_text_locale();

            shown.update(text_of(name, fonts::UI));

            assert_eq!(family_of(shown.raw()), family, "{locale:?}");
        }
    }
}
