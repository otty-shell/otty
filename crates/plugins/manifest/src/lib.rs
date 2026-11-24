//! The plugin manifest, `otty-plugin.toml`, and the permissions of each
//! part of a plugin (spec 2026-11-19-agent-plugins, "What a plugin is" and
//! "Permissions").
//!
//! A [`Manifest`] names the plugin and its parts, [`AgentPart`] and
//! [`ProviderPart`], each with its own [`PartPermissions`], which list
//! themselves in plain words as [`PermissionLine`]s. It names no
//! [`Contract`]: the SDK a plugin is built with decides the version of
//! each, and `otty-plugin-host` reads them from the `.wasm`, which is
//! where "Needs a newer OTTY" comes from.
//! [`Permissions`] are the permissions of one part made concrete for an
//! [`Environment`]: they answer whether that part may read a path, run a
//! program, call a host or open a key place. A provider part's
//! [`LoginRule`]s add what it reads inside the [`LoginFolder`]s OTTY gives
//! it for one call.

mod agent_program;
mod contract;
mod environment;
mod manifest;
mod part;
mod path_glob;
mod permissions;
#[cfg(test)]
mod testing;
mod version;

pub use crate::agent_program::{ProgramInstall, ProgramPlace};
pub use crate::contract::{Contract, ContractVersion, known_versions};
pub use crate::environment::Environment;
pub use crate::manifest::{
    AgentPart, Manifest, ManifestError, PluginInfo, ProviderPart,
};
pub use crate::part::{Domain, PartKind, PartRef};
pub use crate::path_glob::{FolderGlob, GlobProblem, PathGlob};
pub use crate::permissions::{
    JsonPointer, KeyColumn, KeyPlace, KeyRule, KeyringRule, LoginFolder,
    LoginKeys, LoginRule, PartPermissions, PermissionLine, Permissions,
    ProgramRule,
};
pub use crate::version::PluginVersion;
