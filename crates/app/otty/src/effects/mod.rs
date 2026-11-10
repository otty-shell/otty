//! The I/O that belongs to the whole app: the host name, the pinned modes
//! file of the frame, the language of the interface and the fonts, and
//! writing the shell integration files of the app's shell. A feature's I/O
//! lives in its own `io` crate. An effect changes no state: it does the work
//! and turns the answer into an [`crate::message::AppEvent`].
//!
//! The clippy bans of `clippy.toml` (files, processes, background tasks) are
//! lifted only at the top of the files here, so clippy shows I/O that lands
//! anywhere else.

pub(crate) mod host_name;
pub(crate) mod locale;
pub(crate) mod modes;
pub(crate) mod shell_integration;
pub(crate) mod startup;
