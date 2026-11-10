//! All I/O of the app: files, the host name, the system language and the
//! background work. An effect changes no state: it does the work and turns
//! the answer into an [`crate::message::AppEvent`] for the feature that asked.
//!
//! The clippy bans of `clippy.toml` (files, processes, background tasks) are
//! lifted only at the top of the files here and in `subscription.rs`, so
//! clippy shows I/O that lands anywhere else.

pub(crate) mod containers;
pub(crate) mod host_name;
pub(crate) mod locale;
pub(crate) mod modes;
pub(crate) mod shell_integration;
pub(crate) mod startup;
