//! The kit's fonts for drawing tests that measure text.

use std::borrow::Cow;

use iced::Element;
use otty_ui_kit::fonts;

/// Loads the kit's fonts and lists the computer's variable CJK fonts, so
/// text is measured and drawn as in the app. The fonts stay loaded for the
/// whole process, so a test that loads them runs alone, see
/// [`crate::alone::is_alone`].
///
/// # Example
///
/// ```no_run
/// use otty_app_testing::fonts::load_kit_fonts;
///
/// load_kit_fonts();
/// ```
pub fn load_kit_fonts() {
    fonts::register_variable_cjk_faces();

    let settings = iced::Settings {
        fonts: fonts::all_bytes().into_iter().map(Cow::Borrowed).collect(),
        ..iced::Settings::default()
    };
    let empty: Element<'_, ()> = iced::widget::text("").into();

    let _ = iced_test::Simulator::with_settings(settings, empty);
}
