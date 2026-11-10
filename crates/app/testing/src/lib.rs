//! Test helpers the feature crates of the OTTY app share.
//!
//! The app crates take this crate only as a dev-dependency, so nothing of it
//! is built into the app. A helper lands here when tests of more than one
//! crate need it; a helper that needs the whole app stays in the tests of the
//! binary `otty-app`.
//!
//! - [`mouse`]: the mouse input `iced_test` has no helper for.

pub mod mouse;
