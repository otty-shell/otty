//! Test helpers the feature crates of the OTTY app share.
//!
//! The app crates take this crate only as a dev-dependency, so nothing of it
//! is built into the app. A helper lands here when tests of more than one
//! crate need it; a helper that needs the whole app stays in the tests of the
//! binary `otty-app`.
//!
//! - [`mouse`]: the mouse input `iced_test` has no helper for.
//! - [`temp_dir`]: a folder for one test, for the tests of the binary's
//!   effects and the drawing tests that keep a reference picture.

pub mod mouse;
pub mod temp_dir;
