//! Runs OTTY plugins: WebAssembly components built for `wasm32-wasip2`
//! that teach OTTY about an agent or a provider (spec
//! 2026-11-19-agent-plugins).
//!
//! A plugin is described by its [`Manifest`], `otty-plugin.toml`: the
//! plugin, the contract version of each kind of part in `[api]`, and its
//! parts, [`AgentPart`] and [`ProviderPart`], each with its own
//! [`PartPermissions`], which list themselves in plain words as
//! [`PermissionLine`]s. A manifest that names a contract version this OTTY
//! does not know is refused with "Needs a newer OTTY".
//!
//! The one [`PluginEngine`] compiles a plugin, or reads it from the disk
//! cache, refuses imports other than `wasi:cli`, `wasi:io` and `otty:*`,
//! and checks that the component exports exactly the contracts of `[api]`.
//! A [`LoadedPlugin`] is then called part by part: a [`CallBatch`] holds a
//! store with an empty WASI context, a 64 MiB memory cap and the
//! [`Permissions`] of that part only, and each call gets the deadline of
//! its [`CallKind`]. A trap, a missed deadline or the memory cap fails that
//! call only. What the plugin writes to stderr goes to its [`PluginLog`].
//!
//! The host operations a plugin calls arrive with issues 04 to 06, and the
//! bindings of each contract version with issue 03.

mod cache;
mod calls;
mod contract;
mod engine;
mod environment;
mod imports;
mod manifest;
mod memory_cap;
mod part;
mod path_glob;
mod permissions;
mod plugin;
mod plugin_log;
#[cfg(test)]
mod testing;
mod version;

/// The error wasmtime gives inside a call.
pub use wasmtime::Error;

pub use crate::cache::LoadKind;
pub use crate::calls::{CallBatch, CallError, CallKind, CallState};
pub use crate::contract::{Api, ContractVersion, ExportedApi};
pub use crate::engine::{EngineError, LoadError, PluginEngine, TICK};
pub use crate::environment::Environment;
pub use crate::manifest::{
    AddByHand, AgentPart, Manifest, ManifestError, PluginInfo, ProviderPart,
};
pub use crate::memory_cap::MEMORY_CAP;
pub use crate::part::{PartKind, PartRef};
pub use crate::path_glob::{GlobProblem, PathGlob};
pub use crate::permissions::{
    JsonPointer, KeyPlace, KeyRule, KeyringRule, PartPermissions,
    PermissionLine, Permissions, ProgramRule,
};
pub use crate::plugin::LoadedPlugin;
pub use crate::plugin_log::{LOG_CAP, PluginLog};
pub use crate::version::PluginVersion;
