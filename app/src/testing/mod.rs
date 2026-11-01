//! Code that exists only for tests and is never built into the app.
//!
//! `task_outputs` is a helper that unit tests call to read the events a
//! reducer emitted. `screenshots` renders the app screens to PNG files.

mod screenshots;
pub(crate) mod task_outputs;
