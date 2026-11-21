//! The plugin manifest, `otty-plugin.toml`, and the permissions of each
//! part of a plugin (spec 2026-11-19-agent-plugins, "What a plugin is" and
//! "Permissions").
//!
//! A [`Manifest`] names the plugin, the version of each [`Contract`] it
//! exports in `[api]`, and its parts, [`AgentPart`] and [`ProviderPart`],
//! each with its own [`PartPermissions`], which list themselves in plain
//! words as [`PermissionLine`]s. A manifest that names a contract version
//! this OTTY does not know is refused with "Needs a newer OTTY".
//! [`Permissions`] are the permissions of one part made concrete for an
//! [`Environment`]: they answer whether that part may read a path, run a
//! program, call a host or open a key place.

mod contract;
mod environment;
mod manifest;
mod part;
mod path_glob;
mod permissions;
#[cfg(test)]
mod testing;
mod version;

pub use crate::contract::{Api, Contract, ContractVersion, known_versions};
pub use crate::environment::Environment;
pub use crate::manifest::{
    AgentPart, Manifest, ManifestError, PluginInfo, ProviderPart,
};
pub use crate::part::{Domain, PartKind, PartRef};
pub use crate::path_glob::{GlobProblem, PathGlob};
pub use crate::permissions::{
    JsonPointer, KeyColumn, KeyPlace, KeyRule, KeyringRule, PartPermissions,
    PermissionLine, Permissions, ProgramRule,
};
pub use crate::version::PluginVersion;
