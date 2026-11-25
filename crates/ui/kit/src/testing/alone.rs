//! Running one test alone, in a copy of the test program, and the kit's
//! fonts that such a test loads.
//!
//! The kit's fonts belong to the whole process. A test that loads them would
//! disturb the drawing tests running beside it, so it runs in a copy of the
//! test program where it is the only test.

use std::borrow::Cow;

use iced::Element;

use crate::fonts;

/// Set in the copy of the test program that runs one test alone, so the test
/// knows to go on instead of starting another copy.
const IN_THE_COPY: &str = "OTTY_TEST_ALONE";

/// Says whether the test `name` runs in its own copy of the test program and
/// should go on. In the normal run it starts the copy with `name` as the
/// filter, checks that exactly one test passed there and returns `false`.
///
/// The kit's fonts belong to the whole process, so a test that loads them
/// (see [`load_kit_fonts`]) would disturb the drawing tests running beside
/// it. `name` must match the test's own name, so the copy runs it and
/// nothing else.
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

    false
}

/// Loads the kit's fonts and lists the computer's variable CJK fonts, so
/// text is measured and drawn as in the app. The fonts stay loaded for the
/// whole process, so a test that loads them runs alone, see [`is_alone`].
pub(crate) fn load_kit_fonts() {
    fonts::register_variable_cjk_faces();

    let settings = iced::Settings {
        fonts: fonts::all_bytes().into_iter().map(Cow::Borrowed).collect(),
        ..iced::Settings::default()
    };
    let empty: Element<'_, ()> = iced::widget::text("").into();

    let _ = iced_test::Simulator::with_settings(settings, empty);
}
