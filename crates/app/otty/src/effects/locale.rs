//! The language of the interface: the one the user chose or the system's,
//! and the language the kit's fonts shape Han letters for.

use otty_app_i18n::{self as i18n, Locale};
use otty_ui_kit::fonts;

/// Make `language` the language of the interface, or the system's when it
/// is `None`, and tell the fonts.
pub(crate) fn apply(language: Option<Locale>) {
    let locale = language.unwrap_or_else(system_locale);

    i18n::set_locale(locale);
    apply_text_locale();
}

/// Tell the font system the language of the active locale. Call it after the
/// locale changes.
fn apply_text_locale() {
    fonts::set_text_locale(i18n::text_locale_tag(
        i18n::current_locale(),
        system_locale(),
    ));
}

/// The language the system prefers; English when it says none.
fn system_locale() -> Locale {
    i18n::locale_from_system_preference(sys_locale::get_locale().as_deref())
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::disallowed_methods,
        reason = "the test runs itself in a copy of the test program"
    )]

    use std::borrow::Cow;

    use iced::advanced::graphics::text::{Paragraph, font_system};
    use iced::advanced::text;
    use iced::advanced::text::paragraph::Plain;
    use iced::alignment::Vertical;
    use iced::widget::text::{LineHeight, Shaping, Wrapping};
    use iced::{Element, Font, Pixels, Size};

    use super::*;

    /// Set in the copy, so the test knows to go on instead of starting
    /// another.
    const IN_THE_COPY: &str = "OTTY_TEST_ALONE";

    /// Says whether the test `name` runs in its copy and should go on. In
    /// the normal run it starts the copy, checks that the test passed there
    /// and returns `false`. The kit's fonts and the language of the app
    /// belong to the whole process, so a test that changes them would
    /// disturb the drawing tests running beside it.
    fn is_alone(name: &str) -> bool {
        if std::env::var_os(IN_THE_COPY).is_some() {
            return true;
        }

        let program = std::env::current_exe().expect("the test program");
        let output = std::process::Command::new(program)
            .arg(name)
            .arg("--nocapture")
            .env(IN_THE_COPY, "1")
            .output()
            .expect("the copy should start");
        let said = String::from_utf8_lossy(&output.stdout);
        let complained = String::from_utf8_lossy(&output.stderr);

        assert!(
            output.status.success() && said.contains("1 passed"),
            "{said}{complained}"
        );

        // A test that skips a language says so here.
        eprint!("{complained}");

        false
    }

    /// Loads the kit's fonts and lists the computer's variable CJK fonts, so
    /// text is measured and drawn as in the app.
    fn load_kit_fonts() {
        fonts::register_variable_cjk_faces();

        let settings = iced::Settings {
            fonts: fonts::all_bytes().into_iter().map(Cow::Borrowed).collect(),
            ..iced::Settings::default()
        };
        let empty: Element<'_, ()> = iced::widget::text("").into();

        let _ = iced_test::Simulator::with_settings(settings, empty);
    }

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

    /// The launch editor marks its optional fields in the language of the
    /// interface, for either launch type. Copied from the old launch editor
    /// view: it switches the process language, so it runs in a copy here,
    /// where the quick launch feature's drawing meets the language switch.
    #[test]
    fn the_optional_marker_of_the_launch_editor_is_in_each_language() {
        use otty_app_domain::commands::AppCommand;
        use otty_app_quick_launch::{QuickLaunchIntent, QuickLaunchType};
        use otty_app_slots::MainArea;
        use otty_app_theme::theme::ThemeProps;

        use crate::message::AppEvent;
        use crate::testing::window::app;

        if !is_alone(
            "the_optional_marker_of_the_launch_editor_is_in_each_language",
        ) {
            return;
        }

        for locale in Locale::ALL {
            apply(Some(locale));
            let marker = i18n::t(otty_app_i18n::Key::FieldOptional);
            for command_type in [QuickLaunchType::Custom, QuickLaunchType::Ssh]
            {
                let mut app = app();
                let _ = crate::dispatch::dispatch(
                    &mut app,
                    AppEvent::Command(AppCommand::NewLaunch),
                );
                let tab_id = app.tabs.active_tab_id().expect("the editor");
                let _ = crate::dispatch::dispatch(
                    &mut app,
                    AppEvent::QuickLaunch(
                        QuickLaunchIntent::WizardSelectCommandType {
                            tab_id,
                            command_type,
                        },
                    ),
                );
                let theme = app.theme_manager.current().clone();
                let editor = app
                    .quick_launch
                    .main_area(ThemeProps::new(&theme))
                    .into_element(ThemeProps::new(&theme));
                let mut ui = iced_test::Simulator::with_size(
                    iced::Settings::default(),
                    (1024.0, 1400.0),
                    editor,
                );

                assert!(ui.find(marker).is_ok(), "{locale:?}: {marker}");
                assert_eq!(
                    ui.find("optional").is_ok(),
                    marker == "optional",
                    "{locale:?}"
                );
            }
        }
    }
}
