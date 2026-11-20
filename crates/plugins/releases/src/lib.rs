//! Installs, updates and removes OTTY plugins (spec
//! 2026-11-19-agent-plugins, Decisions 4, 9 and 10, "Install and update
//! flow").
//!
//! A plugin comes from the GitHub releases of its repository
//! ([`Installer::find_release`]), from a local folder its author builds it
//! in ([`Installer::find_local`]), or from the copy built into OTTY
//! ([`Installer::install_embedded`]). An [`InstallPlan`] holds what the
//! install dialog shows: the manifest, "Official", "Community" or "Local",
//! and each id or host another plugin already serves, which the user
//! settles per part. [`Installer::install`] downloads or copies the files,
//! checks their size, the icons and the `.wasm`, keeps them in
//! `<plugins folder>/<id>/<version>/`, compiles the plugin into the cache
//! and records it in `plugins.json`.
//!
//! [`Installer::check_updates`] runs once a day and
//! [`Installer::check_update`] on demand: a newer release that asks for no
//! new permission and no new part installs on its own; one that does
//! waits as a [`PendingUpdate`] with its [`PermissionDiff`] until the user
//! agrees. The network is behind [`HttpClient`] and the compiler behind
//! [`PluginCompiler`], so the rules are tested without either.

mod choose;
mod compiler;
mod diff;
mod embedded;
mod error;
mod github;
mod http;
mod icon;
mod installer;
mod package;
mod plan;
mod plugins_file;
#[cfg(test)]
mod testing;

pub use crate::choose::SkippedRelease;
pub use crate::compiler::PluginCompiler;
pub use crate::diff::{PartChange, PermissionDiff};
pub use crate::embedded::{EmbeddedPlugin, SUMS_FILE, embedded_plugins};
pub use crate::error::InstallError;
pub use crate::github::{OFFICIAL_ORGANIZATION, RepositoryUrl};
pub use crate::http::{HttpClient, HttpError, USER_AGENT, UreqClient};
pub use crate::icon::{IconProblem, MAX_ICON_BYTES};
pub use crate::installer::updates::UPDATE_CHECK_EVERY;
pub use crate::installer::{Installer, Places};
pub use crate::package::{MANIFEST_FILE, MAX_WASM_BYTES};
pub use crate::plan::{
    Conflict, InstallPlan, PendingUpdate, UpdateOutcome, UpdateReport,
};
pub use crate::plugins_file::{
    Choice, FileHashes, InstalledPart, InstalledPlugin, Served, Source, Trust,
    Winner,
};
