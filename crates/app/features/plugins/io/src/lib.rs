//! The I/O of the Plugins page: the installed plugins in `plugins.json`
//! and everything that changes them, through `otty-plugin-releases`.
//!
//! [`SharedInstaller::open`] opens the installed plugins at start and
//! [`SharedInstaller::plugins`] lists them for the page. Every other
//! function runs on a thread of its own, since it may reach GitHub, copy
//! files or compile a plugin, and answers with a
//! [`PluginsIntent`](otty_app_plugins::PluginsIntent): [`find_release`]
//! and [`find_local`] find a plugin for the install dialog; [`install`],
//! [`set_part_on`], [`remove`] and [`apply_update`] change the installed
//! plugins; [`check_update`] and [`check_updates_if_due`] check for newer
//! versions; [`reload`] reads the folder of a local plugin again. OTTY
//! installs no plugin by itself. The binary only calls them and wraps
//! the intent into its message.

mod installed;
mod work;

pub use installed::{SharedInstaller, places};
pub use work::{
    apply_update, check_update, check_updates_if_due, find_local, find_release,
    install, reload, remove, set_part_on,
};
