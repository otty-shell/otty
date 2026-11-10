//! The language of the interface: the one the user chose or the system's,
//! and the language the kit's fonts shape Han letters for.

use otty_app_i18n::{self as i18n, Locale};
use otty_ui_kit::fonts;

/// Make `language` the language of the interface, or the system's when it
/// is `None`, and tell the fonts.
pub(crate) fn apply(language: Option<Locale>) {
    let locale = language.unwrap_or_else(i18n::system_locale);

    i18n::set_locale(locale);
    apply_text_locale();
}

/// Tell the font system the language of the active locale. Call it after the
/// locale changes.
fn apply_text_locale() {
    fonts::set_text_locale(i18n::text_locale_tag(
        i18n::current_locale(),
        i18n::system_locale(),
    ));
}

#[cfg(test)]
mod tests {
    use iced::advanced::graphics::text::{Paragraph, font_system};
    use iced::advanced::text;
    use iced::advanced::text::paragraph::Plain;
    use iced::alignment::Vertical;
    use iced::widget::text::{LineHeight, Shaping, Wrapping};
    use iced::{Font, Pixels, Size};
    use otty_app_testing::alone::is_alone;
    use otty_app_testing::fonts::load_kit_fonts;

    use super::*;

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

    /// Says whether this is a computer where cosmic-text asks for `family`
    /// and has it. The test below needs one and says so when skipped.
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

    /// A name that stays the same through a change of language is laid out
    /// again by `iced`, and takes the shapes of the new language.
    #[test]
    fn han_letters_take_the_shapes_of_the_language_without_a_restart() {
        if !is_alone(
            "han_letters_take_the_shapes_of_the_language_without_a_restart",
        ) {
            return;
        }
        load_kit_fonts();
        if !asks_for_and_has("Noto Sans CJK JP") {
            return;
        }

        let name = "直";
        apply(Some(Locale::Ja));
        let mut shown = Plain::<Paragraph>::default();
        shown.update(text_of(name, fonts::UI));
        assert_eq!(family_of(shown.raw()), "Noto Sans CJK JP");

        for (locale, family) in [
            (Locale::ZhTw, "Noto Sans CJK TC"),
            (Locale::ZhCn, "Noto Sans CJK SC"),
            (Locale::Ko, "Noto Sans CJK KR"),
            (Locale::Ja, "Noto Sans CJK JP"),
        ] {
            apply(Some(locale));

            shown.update(text_of(name, fonts::UI));

            assert_eq!(family_of(shown.raw()), family, "{locale:?}");
        }
    }
}
