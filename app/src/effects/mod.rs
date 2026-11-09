//! The Effects layer: runs what widget reducers ask for and what the app
//! needs when the window is ready. Each widget has one file here; it never
//! reduces, and sends results and messages back as
//! `Task::done(AppEvent::...)`.

pub(crate) mod chrome;
pub(crate) mod containers;
pub(crate) mod explorer;
pub(crate) mod modes;
pub(crate) mod notifications;
pub(crate) mod palette;
pub(crate) mod quick_launch;
pub(crate) mod settings;
pub(crate) mod sidebar;
pub(crate) mod startup;
pub(crate) mod tabs;
pub(crate) mod terminal_workspace;
