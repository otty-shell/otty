//! Remembers which characters no installed font can draw.
//!
//! When the terminal font lacks a character, the text shaper tries every
//! installed font, one after another, and remembers nothing. A character
//! that no font has pays for that full search each time it is shaped. This
//! module answers "is this character in any font?" with a cheap glyph lookup
//! and remembers the answer, so the caller can draw the placeholder glyph
//! directly.

use std::collections::HashMap;

use iced::Font;
use iced_graphics::text::cosmic_text::fontdb;
use iced_graphics::text::{self, cosmic_text};

/// How many characters are remembered per font before starting over.
///
/// One answer takes a few bytes; the limit only keeps a program that prints
/// all of Unicode from growing this memory without end.
const MAX_REMEMBERED_CHARS: usize = 65_536;

/// Remembers, per terminal font, which characters every installed font
/// lacks.
#[derive(Clone, Default)]
pub(crate) struct FontCoverage {
    /// Answers for the fonts whose full font search already failed once.
    missing_by_font: HashMap<Font, HashMap<char, bool>>,
}

impl FontCoverage {
    /// Whether `ch` is known to be in no font the shaper would try for
    /// `font`.
    ///
    /// Answers `false` without looking until [`Self::fallback_ran_out`] was
    /// called for `font`: before that, looking a character up could load
    /// fonts the shaper would never touch. After that, `any_font_has_char`
    /// runs once per character and its answer is remembered.
    pub(crate) fn is_missing(
        &mut self,
        font: Font,
        ch: char,
        any_font_has_char: impl FnOnce() -> bool,
    ) -> bool {
        let Some(missing) = self.missing_by_font.get_mut(&font) else {
            return false;
        };
        if let Some(is_missing) = missing.get(&ch) {
            return *is_missing;
        }

        if missing.len() >= MAX_REMEMBERED_CHARS {
            missing.clear();
        }
        let is_missing = !any_font_has_char();
        missing.insert(ch, is_missing);

        is_missing
    }

    /// Note that the shaper tried every font for `font` and still drew the
    /// placeholder glyph, which means all those fonts are loaded now.
    pub(crate) fn fallback_ran_out(&mut self, font: Font) {
        self.missing_by_font.entry(font).or_default();
    }

    /// Forget everything; call when the installed fonts changed.
    pub(crate) fn clear(&mut self) {
        self.missing_by_font.clear();
    }
}

/// Whether `ch` is drawn from its own glyph or not at all.
///
/// For most of Unicode a missing glyph is not the end: the shaper may build
/// the character from a base letter and marks, hide it, or draw a space
/// instead. That needs data this crate does not have, so only the ranges
/// with none of those cases are listed here: box drawing to Braille, CJK
/// ideographs, private use, Tangut to Nushu, emoji and symbols, and the
/// planes with no assigned characters. Everything else keeps the full font
/// search.
pub(crate) fn is_drawn_only_from_own_glyph(ch: char) -> bool {
    matches!(
        u32::from(ch),
        0x2500..=0x28FF
            | 0x3400..=0x9FFF
            | 0xE000..=0xF8FF
            | 0x17000..=0x1B2FF
            | 0x1F000..=0x1FBFF
            | 0x20000..=0x2F7FF
            | 0x30000..=0xDFFFF
            | 0xF0000..=0x10FFFF
    )
}

/// Whether any font the shaper would try for `font` has a glyph for `ch`.
///
/// This looks at exactly the fonts the full font search walks through, but
/// only reads each font's character map instead of shaping the text with
/// it.
///
/// Known limit: the character map is read with a simpler reader than the
/// one the shaper uses. It understands the two common table kinds (formats
/// 4 and 12), which is what nearly every font has. A font that keeps its
/// characters only in a rarer table kind is seen as not having them, so a
/// character found only in such a font is drawn as the placeholder.
pub(crate) fn any_font_has_char(
    font_system: &mut cosmic_text::FontSystem,
    font: Font,
    ch: char,
) -> bool {
    let attrs = text::to_attributes(font);
    let terminal_face = font_system.db().query(&fontdb::Query {
        families: &[attrs.family],
        weight: attrs.weight,
        stretch: attrs.stretch,
        style: attrs.style,
    });
    let terminal_face_has_char = terminal_face
        .is_some_and(|id| face_has_char(font_system, id, attrs.weight, ch));
    if terminal_face_has_char {
        return true;
    }

    let fallback_faces = font_system
        .db()
        .faces()
        .filter(|face| attrs.matches(face))
        .map(|face| face.id)
        .collect::<Vec<_>>();

    fallback_faces
        .into_iter()
        .any(|id| face_has_char(font_system, id, attrs.weight, ch))
}

fn face_has_char(
    font_system: &mut cosmic_text::FontSystem,
    id: fontdb::ID,
    weight: fontdb::Weight,
    ch: char,
) -> bool {
    // A face that does not load cannot be used by the shaper either.
    font_system
        .get_font(id, weight)
        .is_some_and(|face| face.as_swash().charmap().map(ch) != 0)
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use iced::font::Weight as FontWeight;

    use super::*;

    const BOLD: Font = Font {
        weight: FontWeight::Bold,
        ..Font::MONOSPACE
    };

    /// Ask about `ch`, counting how often the fonts are searched.
    fn is_missing(
        coverage: &mut FontCoverage,
        font: Font,
        ch: char,
        any_font_has_char: bool,
        searches: &Cell<usize>,
    ) -> bool {
        coverage.is_missing(font, ch, || {
            searches.set(searches.get() + 1);
            any_font_has_char
        })
    }

    #[test]
    fn answers_nothing_before_the_shaper_ran_out_of_fonts() {
        let mut coverage = FontCoverage::default();
        let searches = Cell::new(0);

        let missing =
            is_missing(&mut coverage, Font::MONOSPACE, '中', false, &searches);

        assert!(!missing);
        assert_eq!(searches.get(), 0);
    }

    #[test]
    fn remembers_both_answers_and_searches_each_char_once() {
        let mut coverage = FontCoverage::default();
        let searches = Cell::new(0);
        coverage.fallback_ran_out(Font::MONOSPACE);

        for _ in 0..3 {
            assert!(is_missing(
                &mut coverage,
                Font::MONOSPACE,
                '\u{10FFFD}',
                false,
                &searches
            ));
            assert!(!is_missing(
                &mut coverage,
                Font::MONOSPACE,
                '中',
                true,
                &searches
            ));
        }

        assert_eq!(searches.get(), 2);
    }

    #[test]
    fn keeps_each_font_apart() {
        let mut coverage = FontCoverage::default();
        let searches = Cell::new(0);
        coverage.fallback_ran_out(Font::MONOSPACE);

        assert!(is_missing(
            &mut coverage,
            Font::MONOSPACE,
            '中',
            false,
            &searches
        ));
        assert!(!is_missing(&mut coverage, BOLD, '中', false, &searches));
        assert_eq!(searches.get(), 1);

        coverage.fallback_ran_out(BOLD);
        assert!(!is_missing(&mut coverage, BOLD, '中', true, &searches));
        assert_eq!(searches.get(), 2);
    }

    #[test]
    fn running_out_again_keeps_the_remembered_answers() {
        let mut coverage = FontCoverage::default();
        let searches = Cell::new(0);
        coverage.fallback_ran_out(Font::MONOSPACE);
        is_missing(&mut coverage, Font::MONOSPACE, '中', false, &searches);

        coverage.fallback_ran_out(Font::MONOSPACE);

        assert!(is_missing(
            &mut coverage,
            Font::MONOSPACE,
            '中',
            true,
            &searches
        ));
        assert_eq!(searches.get(), 1);
    }

    #[test]
    fn clear_forgets_answers_and_fonts() {
        let mut coverage = FontCoverage::default();
        let searches = Cell::new(0);
        coverage.fallback_ran_out(Font::MONOSPACE);
        is_missing(&mut coverage, Font::MONOSPACE, '中', false, &searches);

        coverage.clear();

        assert!(!is_missing(
            &mut coverage,
            Font::MONOSPACE,
            '中',
            false,
            &searches
        ));
        assert_eq!(searches.get(), 1);
    }

    #[test]
    fn starts_over_when_too_many_chars_are_remembered() {
        let mut coverage = FontCoverage::default();
        let searches = Cell::new(0);
        coverage.fallback_ran_out(Font::MONOSPACE);
        let chars = ('\u{20000}'..)
            .take(MAX_REMEMBERED_CHARS + 1)
            .collect::<Vec<_>>();

        for ch in &chars {
            is_missing(&mut coverage, Font::MONOSPACE, *ch, false, &searches);
        }
        assert_eq!(searches.get(), MAX_REMEMBERED_CHARS + 1);

        // The first char was dropped with the rest, the last one was kept.
        is_missing(&mut coverage, Font::MONOSPACE, chars[0], false, &searches);
        is_missing(
            &mut coverage,
            Font::MONOSPACE,
            chars[MAX_REMEMBERED_CHARS],
            false,
            &searches,
        );
        assert_eq!(searches.get(), MAX_REMEMBERED_CHARS + 2);
    }

    #[test]
    fn plain_glyph_chars_are_ideographs_symbols_and_private_use() {
        for ch in [
            '─',
            '⣿',
            '中',
            '\u{E0B0}',
            '\u{18000}',
            '😀',
            '\u{1FB00}',
            '\u{20000}',
            '\u{3FFFD}',
            '\u{10FFFD}',
        ] {
            assert!(is_drawn_only_from_own_glyph(ch), "{ch:?}");
        }
    }

    #[test]
    fn chars_the_shaper_may_draw_another_way_are_not_plain_glyph_chars() {
        for ch in [
            'a',
            'é',
            '\u{00A0}',
            '\u{200B}',
            '\u{2011}',
            '\u{3000}',
            'が',
            '가',
            '\u{F900}',
            '\u{FEFF}',
            '\u{1D15E}',
            '\u{2F800}',
            '\u{E0001}',
        ] {
            assert!(!is_drawn_only_from_own_glyph(ch), "{ch:?}");
        }
    }

    #[test]
    fn lookup_agrees_with_the_full_font_search_for_common_chars() {
        let mut font_system =
            text::font_system().write().expect("Write font system");
        let font_system = font_system.raw();

        // No char is assumed to be installed: whatever fonts this machine
        // has, the cheap lookup and the full search must give one answer.
        for font in [Font::MONOSPACE, BOLD] {
            for ch in ['a', '─', '中'] {
                let some_font_has_it = any_font_has_char(font_system, font, ch);
                let search_ends_with_placeholder =
                    full_search_draws_placeholder(font_system, font, ch);

                assert_eq!(
                    some_font_has_it, !search_ends_with_placeholder,
                    "{ch:?} with {font:?}"
                );
            }
        }
    }

    #[test]
    fn lookup_finds_no_font_for_a_char_the_full_search_gives_up_on() {
        let mut font_system =
            text::font_system().write().expect("Write font system");
        let font_system = font_system.raw();

        // The other direction is not checked for these rare chars: a font
        // with an unusual character table may have one of them while the
        // lookup cannot see it (see `any_font_has_char`).
        for font in [Font::MONOSPACE, BOLD] {
            for ch in ['\u{E0B0}', '\u{31350}', '\u{10FFFD}'] {
                if full_search_draws_placeholder(font_system, font, ch) {
                    assert!(
                        !any_font_has_char(font_system, font, ch),
                        "{ch:?} with {font:?}"
                    );
                }
            }
        }
    }

    /// Whether shaping `ch` with the full font search ends as the
    /// placeholder glyph.
    fn full_search_draws_placeholder(
        font_system: &mut cosmic_text::FontSystem,
        font: Font,
        ch: char,
    ) -> bool {
        let mut buffer = cosmic_text::Buffer::new(
            font_system,
            cosmic_text::Metrics::new(14.0, 16.0),
        );
        buffer.set_text(
            font_system,
            &ch.to_string(),
            &text::to_attributes(font),
            cosmic_text::Shaping::Advanced,
            None,
        );

        buffer
            .layout_runs()
            .flat_map(|layout_run| layout_run.glyphs.iter())
            .any(|glyph| glyph.glyph_id == 0)
    }
}
