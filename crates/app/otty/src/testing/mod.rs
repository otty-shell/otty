//! Code that exists only for tests and is never built into the app.
//!
//! `window` builds an app without reading files. `keyboard` has the key
//! presses the keyboard subscription reports. `task_outputs` reads what a
//! [`iced::Task`] of `update` or of the effects carries. `mouse` has the
//! mouse input that `iced_test` has no helper for.

pub(crate) mod keyboard;
pub(crate) mod mouse;
pub(crate) mod task_outputs;
pub(crate) mod window;
