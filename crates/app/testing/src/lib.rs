//! Test helpers the feature crates of the OTTY app share.
//!
//! The app crates take this crate only as a dev-dependency, so nothing of it
//! is built into the app. A helper lands here when tests of more than one
//! crate need it; a helper that needs the whole app stays in the tests of the
//! binary `otty`.
//!
//! - [`alone`]: running a test alone in a copy of the test program, for the
//!   tests that load the kit's fonts or switch the language of the app.
//! - [`fonts`]: the kit's fonts, for the drawing tests that measure text.
//! - [`mouse`]: the mouse input `iced_test` has no helper for.
//! - [`temp_dir`]: a folder for one test, for the tests that read and write
//!   files and the drawing tests that keep a reference picture.
//! - [`terminal`]: terminals whose program is a mock, for the tests that
//!   decide what a terminal shows without starting a shell.

pub mod alone;
pub mod fonts;
pub mod mouse;
pub mod temp_dir;
pub mod terminal;
