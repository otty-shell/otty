//! Bundled fonts of the design system: Noto Sans for the interface and
//! JetBrainsMono Nerd Font Mono for terminal text. Both are licensed under
//! the SIL Open Font License 1.1 (see `assets/fonts`).
//!
//! Load every file from [`all_bytes`] into the `iced` application before
//! using the [`Font`] values below, and call [`register_variable_cjk_faces`]
//! before the first text is shaped.

use std::borrow::Cow;
use std::mem;

use iced::Font;
use iced::advanced::graphics::text::cosmic_text::skrifa::{
    self, MetadataProvider as _, Tag,
};
use iced::advanced::graphics::text::cosmic_text::{self, fontdb};
use iced::advanced::graphics::text::font_system;
use iced::font::{Family, Weight};

/// Family name of the interface font as stored in the font file.
const UI_FAMILY: &str = "Noto Sans";

/// Family name of the terminal font as stored in the font file.
const MONO_FAMILY: &str = "JetBrainsMono Nerd Font Mono";

/// The weights the interface and the terminal draw with.
const DRAWN_WEIGHTS: [u16; 4] = [400, 500, 600, 700];

/// One letter each of Hangul, kana and Han: a font with any of them is a CJK
/// font, whatever it is called.
const CJK_LETTERS: [char; 3] = ['한', 'あ', '漢'];

/// A font collection with no fonts in it: loading it adds no face, but tells
/// `iced` that text has to be shaped again.
const EMPTY_FONT_COLLECTION: &[u8] = b"ttcf\x00\x01\x00\x00\x00\x00\x00\x00";

/// Noto Sans Regular (400) font file.
pub const NOTO_SANS_REGULAR_BYTES: &[u8] =
    include_bytes!("../../../../assets/fonts/NotoSans/NotoSans-Regular.ttf");

/// Noto Sans Medium (500) font file.
pub const NOTO_SANS_MEDIUM_BYTES: &[u8] =
    include_bytes!("../../../../assets/fonts/NotoSans/NotoSans-Medium.ttf");

/// Noto Sans SemiBold (600) font file.
pub const NOTO_SANS_SEMIBOLD_BYTES: &[u8] =
    include_bytes!("../../../../assets/fonts/NotoSans/NotoSans-SemiBold.ttf");

/// JetBrainsMono Nerd Font Mono Regular (400) font file.
pub const JETBRAINS_MONO_REGULAR_BYTES: &[u8] = include_bytes!(
    "../../../../assets/fonts/JetBrains/JetBrainsMonoNerdFontMono-Regular.ttf"
);

/// JetBrainsMono Nerd Font Mono Bold (700) font file.
pub const JETBRAINS_MONO_BOLD_BYTES: &[u8] = include_bytes!(
    "../../../../assets/fonts/JetBrains/JetBrainsMonoNerdFontMono-Bold.ttf"
);

/// Interface font, Noto Sans Regular: default text of rows, buttons and menus.
pub const UI: Font = Font {
    family: Family::Name(UI_FAMILY),
    weight: Weight::Normal,
    ..Font::DEFAULT
};

/// Interface font, Noto Sans Medium: session titles in pane headers.
pub const UI_MEDIUM: Font = Font {
    family: Family::Name(UI_FAMILY),
    weight: Weight::Medium,
    ..Font::DEFAULT
};

/// Interface font, Noto Sans SemiBold: titles and headings.
pub const UI_SEMIBOLD: Font = Font {
    family: Family::Name(UI_FAMILY),
    weight: Weight::Semibold,
    ..Font::DEFAULT
};

/// Terminal and code font, JetBrainsMono Nerd Font Mono Regular.
pub const MONO: Font = Font {
    family: Family::Name(MONO_FAMILY),
    weight: Weight::Normal,
    ..Font::DEFAULT
};

/// Terminal and code font, JetBrainsMono Nerd Font Mono Bold: bold terminal
/// text.
pub const MONO_BOLD: Font = Font {
    family: Family::Name(MONO_FAMILY),
    weight: Weight::Bold,
    ..Font::DEFAULT
};

/// Returns the bytes of every bundled font file the app must load at startup.
pub fn all_bytes() -> [&'static [u8]; 5] {
    [
        NOTO_SANS_REGULAR_BYTES,
        NOTO_SANS_MEDIUM_BYTES,
        NOTO_SANS_SEMIBOLD_BYTES,
        JETBRAINS_MONO_REGULAR_BYTES,
        JETBRAINS_MONO_BOLD_BYTES,
    ]
}

/// Lists each variable CJK font of the computer at the weights the app draws
/// with, so Chinese, Japanese and Korean text uses it at all. Call it once, where
/// the app loads its fonts and before the first text is shaped.
///
/// cosmic-text 0.15 takes a fallback font only if the weight the font database
/// lists is exactly the weight asked for (`font_weight_diff == 0` in
/// `font/fallback/mod.rs`). The database lists a variable font once, at the
/// weight its file states, although it can draw many: Fedora's
/// `NotoSansCJK-VF.ttc` is listed as 100 and so never matches 400, 500 or 600.
/// The search then falls through to a font with next to no Hangul, and Korean
/// syllables come apart into their letters, each one as wide as a syllable.
///
/// Drop this once cosmic-text accepts a variable font for any weight on its
/// axis. A font that is not variable, or has no CJK, is not touched, so a
/// computer without such a font gains no entry, whatever its system.
///
/// # Example
///
/// Not run: it changes the font system that the whole process shares.
///
/// ```no_run
/// otty_ui_kit::fonts::register_variable_cjk_faces();
/// ```
pub fn register_variable_cjk_faces() {
    let mut system = font_system().write().expect("Write font system");

    add_variable_cjk_faces(system.raw().db_mut());
}

/// Tells the font system the language of the text, as a tag such as `ja`, `ko`
/// or `zh-TW`. Han letters have regional shapes, and cosmic-text 0.15 picks the
/// regional font by that exact tag, so `ja-JP` is not taken for Japanese.
///
/// cosmic-text fixes its language when it is created, so this builds a new one
/// around the same font database. All text is shaped again on the next layout,
/// the terminal's too, so a change of language needs no restart.
///
/// # Example
///
/// Not run: it changes the font system that the whole process shares.
///
/// ```no_run
/// otty_ui_kit::fonts::set_text_locale("ko");
/// ```
pub fn set_text_locale(locale: &str) {
    let mut system = font_system().write().expect("Write font system");
    let raw = system.raw();

    if raw.locale() == locale {
        return;
    }

    let nothing = cosmic_text::FontSystem::new_with_locale_and_db(
        String::new(),
        fontdb::Database::new(),
    );
    let (_, db) = mem::replace(raw, nothing).into_locale_and_db();
    *raw =
        cosmic_text::FontSystem::new_with_locale_and_db(locale.to_owned(), db);

    system.load_font(Cow::Owned(EMPTY_FONT_COLLECTION.to_vec()));
}

/// Adds one entry per missing weight for each variable CJK face of `db`. An
/// entry points at the same face of the same file; cosmic-text sets the weight
/// axis of a face to the weight it was asked for.
fn add_variable_cjk_faces(db: &mut fontdb::Database) {
    let mut entries = Vec::new();

    for face in db.faces() {
        let Some(Some((smallest, largest))) =
            db.with_face_data(face.id, variable_cjk_weights)
        else {
            continue;
        };

        for weight in DRAWN_WEIGHTS {
            let is_on_axis = (smallest..=largest).contains(&f32::from(weight));
            let is_listed = db.faces().any(|other| {
                other.weight.0 == weight
                    && other.families == face.families
                    && other.style == face.style
                    && other.stretch == face.stretch
            });

            if is_on_axis && !is_listed {
                entries.push(fontdb::FaceInfo {
                    weight: fontdb::Weight(weight),
                    ..face.clone()
                });
            }
        }
    }

    for entry in entries {
        db.push_face_info(entry);
    }
}

/// Returns the smallest and largest value of the weight axis of a font with CJK
/// letters, or `None` for any other font.
fn variable_cjk_weights(data: &[u8], index: u32) -> Option<(f32, f32)> {
    let font = skrifa::FontRef::from_index(data, index).ok()?;
    let axis = font.axes().get_by_tag(Tag::new(b"wght"))?;
    let charmap = font.charmap();

    CJK_LETTERS
        .iter()
        .any(|letter| charmap.map(*letter).is_some())
        .then(|| (axis.min_value(), axis.max_value()))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    /// A font's `wght` axis, smallest and largest value.
    type WeightAxis = (i16, i16);

    /// Builds a font file with no outlines, enough for the font database to
    /// list it: no font of this computer is needed. `weight` is the weight
    /// the file states and `character` the letter it covers.
    fn font_file(
        family: &str,
        weight: u16,
        axis: Option<WeightAxis>,
        character: char,
    ) -> Vec<u8> {
        let mut tables: Vec<(&[u8; 4], Vec<u8>)> = vec![
            (b"OS/2", os2_table(weight)),
            (b"cmap", cmap_table(character)),
            (b"head", head_table()),
            (b"hhea", hhea_table()),
            (b"maxp", vec![0, 0, 0x50, 0, 0, 2]),
            (b"name", name_table(family)),
        ];
        if let Some(axis) = axis {
            tables.insert(4, (b"fvar", fvar_table(axis)));
        }

        let mut directory: Vec<u8> = vec![0, 1, 0, 0];
        directory.extend((tables.len() as u16).to_be_bytes());
        directory.extend([0; 6]);
        let mut offset = directory.len() + 16 * tables.len();
        let mut body: Vec<u8> = Vec::new();
        for (tag, table) in &tables {
            directory.extend(*tag);
            directory.extend([0; 4]);
            directory.extend((offset as u32).to_be_bytes());
            directory.extend((table.len() as u32).to_be_bytes());
            offset += table.len();
            body.extend(table);
        }
        directory.extend(body);

        directory
    }

    fn os2_table(weight: u16) -> Vec<u8> {
        let mut table = vec![0; 78];
        table[4..6].copy_from_slice(&weight.to_be_bytes());
        table[6..8].copy_from_slice(&5_u16.to_be_bytes());
        table
    }

    fn cmap_table(character: char) -> Vec<u8> {
        let code = u32::from(character).to_be_bytes();
        let mut table = vec![0, 0, 0, 1, 0, 3, 0, 10, 0, 0, 0, 12];
        table.extend([0, 12, 0, 0, 0, 0, 0, 28, 0, 0, 0, 0, 0, 0, 0, 1]);
        table.extend(code);
        table.extend(code);
        table.extend([0, 0, 0, 1]);
        table
    }

    fn head_table() -> Vec<u8> {
        let mut table = vec![0; 54];
        table[0..4].copy_from_slice(&[0, 1, 0, 0]);
        table[12..16].copy_from_slice(&[0x5F, 0x0F, 0x3C, 0xF5]);
        table[18..20].copy_from_slice(&1000_u16.to_be_bytes());
        table
    }

    fn hhea_table() -> Vec<u8> {
        let mut table = vec![0; 36];
        table[0..4].copy_from_slice(&[0, 1, 0, 0]);
        table[34..36].copy_from_slice(&1_u16.to_be_bytes());
        table
    }

    /// A name table with the family and the PostScript name, which the font
    /// database asks for.
    fn name_table(family: &str) -> Vec<u8> {
        let text: Vec<u8> =
            family.encode_utf16().flat_map(u16::to_be_bytes).collect();
        let length = (text.len() as u16).to_be_bytes();

        let mut table = vec![0, 0, 0, 2, 0, 30];
        for name_id in [1, 6] {
            table.extend([0, 3, 0, 1, 4, 9, 0, name_id]);
            table.extend(length);
            table.extend([0, 0]);
        }
        table.extend(text);
        table
    }

    fn fvar_table((smallest, largest): WeightAxis) -> Vec<u8> {
        let mut table = vec![0, 1, 0, 0, 0, 16, 0, 2, 0, 1, 0, 20, 0, 0, 0, 8];
        table.extend(b"wght");
        for value in [smallest, smallest, largest] {
            table.extend(value.to_be_bytes());
            table.extend([0, 0]);
        }
        table.extend([0, 0, 1, 0]);
        table
    }

    fn database(files: Vec<Vec<u8>>) -> fontdb::Database {
        let mut db = fontdb::Database::new();
        for file in files {
            db.load_font_source(fontdb::Source::Binary(Arc::new(file)));
        }
        db
    }

    /// The weights the database lists for `family`, smallest first.
    fn weights_of(db: &fontdb::Database, family: &str) -> Vec<u16> {
        let mut weights: Vec<u16> = db
            .faces()
            .filter(|face| face.families.iter().any(|(name, _)| name == family))
            .map(|face| face.weight.0)
            .collect();
        weights.sort_unstable();
        weights
    }

    const FAMILY: &str = "Some Sans";
    const ALL_WEIGHTS: Option<WeightAxis> = Some((100, 900));

    #[test]
    fn a_font_file_of_these_tests_is_listed_at_the_weight_it_states() {
        let db = database(vec![font_file(FAMILY, 100, ALL_WEIGHTS, '한')]);

        assert_eq!(weights_of(&db, FAMILY), [100]);
    }

    #[test]
    fn a_variable_face_with_hangul_is_listed_at_each_weight_the_app_draws() {
        let mut db = database(vec![font_file(FAMILY, 100, ALL_WEIGHTS, '한')]);

        add_variable_cjk_faces(&mut db);

        assert_eq!(weights_of(&db, FAMILY), [100, 400, 500, 600, 700]);
    }

    #[test]
    fn a_variable_face_with_kana_or_han_is_listed_too() {
        for character in ['あ', '漢'] {
            let mut db =
                database(vec![font_file(FAMILY, 100, ALL_WEIGHTS, character)]);

            add_variable_cjk_faces(&mut db);

            assert_eq!(
                weights_of(&db, FAMILY),
                [100, 400, 500, 600, 700],
                "{character}"
            );
        }
    }

    #[test]
    fn a_new_entry_keeps_the_file_and_the_face_of_the_first() {
        let mut db = database(vec![font_file(FAMILY, 100, ALL_WEIGHTS, '한')]);

        add_variable_cjk_faces(&mut db);

        let first = db.faces().next().expect("the face").clone();
        for face in db.faces() {
            assert_eq!(face.index, first.index);
            assert_eq!(face.families, first.families);
            assert_eq!(face.post_script_name, first.post_script_name);
            assert_eq!(face.style, first.style);
            assert_eq!(face.stretch, first.stretch);
            assert!(db.with_face_data(face.id, |_, _| ()).is_some());
        }
    }

    #[test]
    fn a_weight_the_axis_does_not_reach_is_not_listed() {
        let mut db =
            database(vec![font_file(FAMILY, 200, Some((200, 500)), '한')]);

        add_variable_cjk_faces(&mut db);

        assert_eq!(weights_of(&db, FAMILY), [200, 400, 500]);
    }

    #[test]
    fn a_weight_the_family_already_has_is_not_listed_twice() {
        let mut db = database(vec![
            font_file(FAMILY, 400, ALL_WEIGHTS, '한'),
            font_file(FAMILY, 600, None, '한'),
        ]);

        add_variable_cjk_faces(&mut db);

        assert_eq!(weights_of(&db, FAMILY), [400, 500, 600, 700]);
    }

    #[test]
    fn registering_twice_lists_nothing_new() {
        let mut db = database(vec![font_file(FAMILY, 100, ALL_WEIGHTS, '한')]);

        add_variable_cjk_faces(&mut db);
        add_variable_cjk_faces(&mut db);

        assert_eq!(weights_of(&db, FAMILY), [100, 400, 500, 600, 700]);
    }

    #[test]
    fn a_static_cjk_face_is_left_alone() {
        let mut db = database(vec![font_file(FAMILY, 400, None, '한')]);

        add_variable_cjk_faces(&mut db);

        assert_eq!(weights_of(&db, FAMILY), [400]);
    }

    #[test]
    fn a_variable_face_without_cjk_is_left_alone() {
        let mut db = database(vec![font_file(FAMILY, 100, ALL_WEIGHTS, 'A')]);

        add_variable_cjk_faces(&mut db);

        assert_eq!(weights_of(&db, FAMILY), [100]);
    }

    #[test]
    fn the_fonts_of_the_kit_are_left_alone() {
        let mut db = database(
            all_bytes()
                .into_iter()
                .map(<[u8]>::to_vec)
                .collect::<Vec<_>>(),
        );
        let faces = db.len();

        add_variable_cjk_faces(&mut db);

        assert_eq!(db.len(), faces);
    }

    #[test]
    fn a_new_language_keeps_the_fonts_and_has_all_text_shaped_again() {
        let (locale_before, faces, version) = {
            let mut system = font_system().write().expect("font system");
            let version = system.version();
            let raw = system.raw();

            (raw.locale().to_owned(), raw.db().len(), version)
        };
        let language = "ko-for-a-test";

        set_text_locale(language);

        let (locale, faces_after, version_after) = {
            let mut system = font_system().write().expect("font system");
            let version = system.version();
            let raw = system.raw();

            (raw.locale().to_owned(), raw.db().len(), version)
        };
        set_text_locale(&locale_before);

        assert_eq!(locale, language);
        assert_eq!(faces_after, faces);
        assert!(version_after > version);
    }

    #[test]
    fn a_computer_without_fonts_is_fine() {
        let mut db = fontdb::Database::new();

        add_variable_cjk_faces(&mut db);

        assert_eq!(db.len(), 0);
    }
}
