//! Runs one test by itself, in a copy of the test program.
//!
//! The kit's fonts and the language of the app belong to the whole process, so
//! a test that changes them would disturb the drawing tests running beside it.

use std::borrow::Cow;

use iced::Element;
use otty_ui_kit::fonts;

/// Set in the copy, so the test knows to go on instead of starting another.
const IN_THE_COPY: &str = "OTTY_TEST_ALONE";

/// Says whether the test `name` runs in its copy and should go on. In the
/// normal run it starts the copy, checks that the test passed there and
/// returns `false`.
pub(crate) fn is_alone(name: &str) -> bool {
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

/// Loads the kit's fonts and lists the computer's variable CJK fonts, so text
/// is measured and drawn as in the app. Only for a test that [`is_alone`].
pub(crate) fn load_kit_fonts() {
    fonts::register_variable_cjk_faces();

    let settings = iced::Settings {
        fonts: fonts::all_bytes().into_iter().map(Cow::Borrowed).collect(),
        ..iced::Settings::default()
    };
    let empty: Element<'_, ()> = iced::widget::text("").into();

    let _ = iced_test::Simulator::with_settings(settings, empty);
}
