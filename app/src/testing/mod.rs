//! Code that exists only for tests and is never built into the app.
//!
//! `task_outputs` is a helper that unit tests call to read the events a
//! reducer emitted. `temp_dir` is a folder for one test that goes away with
//! the test. `alone` runs one test by itself, for what belongs to the whole
//! process. `shell_tab` opens a terminal tab with a real shell, for tests
//! that need a pane. `window` puts the app in a given state and draws the
//! whole window, for the tests of the root view and the mode screens.
//! `overlays` opens each menu and dialog of the window. `mouse` has the
//! mouse input that `iced_test` has no helper for.

pub(crate) mod alone;
pub(crate) mod mouse;
pub(crate) mod overlays;
pub(crate) mod shell_tab;
pub(crate) mod task_outputs;
pub(crate) mod temp_dir;
pub(crate) mod window;
