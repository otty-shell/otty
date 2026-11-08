//! Code that exists only for tests and is never built into the app.
//!
//! `task_outputs` is a helper that unit tests call to read the events a
//! reducer emitted. `temp_dir` is a folder for one test that goes away with
//! the test. `alone` runs one test by itself, for what belongs to the whole
//! process.

pub(crate) mod alone;
pub(crate) mod task_outputs;
pub(crate) mod temp_dir;
