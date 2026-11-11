//! Running one test alone, in a copy of the test program.
//!
//! The kit's fonts and the language of the app belong to the whole process.
//! A test that loads the fonts or switches the language would disturb the
//! drawing tests running beside it, so it runs in a copy of the test program
//! where it is the only test.

#![allow(
    clippy::disallowed_methods,
    reason = "the test program starts a copy of itself"
)]

/// Set in the copy, so the test knows to go on instead of starting another.
const IN_THE_COPY: &str = "OTTY_TEST_ALONE";

/// Says whether the test `name` runs in its copy and should go on. In the
/// normal run it starts the copy with `name` as the filter, checks that
/// exactly one test passed there and returns `false`. `name` must match the
/// test's own name, so the copy runs it and nothing else.
///
/// # Example
///
/// ```no_run
/// use otty_app_testing::alone::is_alone;
///
/// #[test]
/// fn texts_in_every_language() {
///     if !is_alone("texts_in_every_language") {
///         return;
///     }
///
///     // Switch the language and check the texts here.
/// }
/// ```
pub fn is_alone(name: &str) -> bool {
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
