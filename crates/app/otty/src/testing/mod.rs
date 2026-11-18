//! Code that exists only for tests and is never built into the app.
//!
//! `window` builds an app without reading files and puts it in the states
//! the tests start from; `agents` does the same for Agents mode, with
//! made-up profiles and sessions. `frames` builds the whole window in a small size,
//! frame after frame, for the scrolling tests. `running` runs the app as
//! the iced runtime does, with the widget operations of its tasks, for the
//! tests of a whole path from a press to a terminal. `keyboard` has the
//! key presses the keyboard subscription reports. `task_outputs` reads
//! what a [`iced::Task`] of `update` or of the startup work carries. The
//! mouse input `iced_test` has no helper for, a folder for one test, the
//! fonts of the kit and running a test alone in a copy of the test program
//! are shared with the feature crates in `otty-app-testing`.

pub(crate) mod agents;
pub(crate) mod frames;
pub(crate) mod keyboard;
pub(crate) mod running;
pub(crate) mod task_outputs;
pub(crate) mod window;
