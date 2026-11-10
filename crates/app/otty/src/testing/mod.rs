//! Code that exists only for tests and is never built into the app.
//!
//! `window` builds an app without reading files and puts it in the states
//! the tests start from. `keyboard` has the key presses the keyboard
//! subscription reports. `task_outputs` reads what a [`iced::Task`] of
//! `update` or of the effects carries. The mouse input `iced_test` has no
//! helper for and the folder a test of the effects works in are shared with
//! the feature crates in `otty-app-testing`.

pub(crate) mod keyboard;
pub(crate) mod task_outputs;
pub(crate) mod window;
