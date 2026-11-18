//! Test-only helpers of the kit.
//!
//! - [`alone`]: running a test alone in a copy of the test program, and the
//!   kit's fonts, which such a test loads.
//! - [`events`]: input the tests send: a press of Tab, the wheel event the
//!   system sends while Shift is held, and an input that says its name when
//!   typed in.
//! - [`live_app`]: an app that runs for a test over many frames.
//! - [`pictures`]: the check that a view draws a saved picture, and the
//!   check that an icon lights up over its whole control.
//! - [`temp_dir`]: a folder for the tests that keep reference pictures on
//!   disk.
//!
//! The app crates have the helpers of [`alone`] in `otty-app-testing`; the
//! kit can not use that crate, because it depends on the kit.

pub(crate) mod alone;
pub(crate) mod events;
pub(crate) mod live_app;
pub(crate) mod pictures;
pub(crate) mod temp_dir;
