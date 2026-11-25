//! Runs OTTY plugins: WebAssembly components built for `wasm32-wasip2`
//! that teach OTTY about an agent or a provider (spec
//! 2026-11-19-agent-plugins).
//!
//! A plugin is described by its [`Manifest`], `otty-plugin.toml`, read and
//! checked by `otty-plugin-manifest`: the plugin and its parts, each with
//! its own [`PartPermissions`]. The contract versions are not in it: the
//! component exports them, as the SDK it was built with decides, and
//! [`ExportedApi`] reads them.
//!
//! The one [`PluginEngine`] compiles a plugin, or reads it from the disk
//! cache, refuses imports other than `wasi:cli`, `wasi:io` and `otty:*`,
//! and checks that every part has its contract exported, that every
//! exported contract of a kind of part has a part, and that this OTTY
//! knows every exported version.
//! A [`LoadedPlugin`] is then called part by part: a [`CallBatch`] holds a
//! store with an empty WASI context, a 64 MiB memory cap and the
//! [`Permissions`] of that part only, and each call gets the deadline of
//! its [`CallKind`]. A trap, a missed deadline or the memory cap fails that
//! call only. What the plugin writes to stderr goes to its [`PluginLog`].
//!
//! The extension points of a domain are a module of that domain, such as
//! [`agents`]: [`CallBatch::agent`] and [`CallBatch::provider`] call the
//! functions of the `agents.agent` and `agents.provider` contracts with
//! the types of their newest versions, [`agents::agent`] and
//! [`agents::provider`]; each version the host keeps has its own
//! bindings, such as [`agents::agent::v0_1`]. An agent part prices the
//! tokens of its answers itself with the price of a model it asks the
//! [`agents::prices::PriceList`] the app gives each read of answers. The
//! host operations a plugin imports are [`operations`]. A plugin may also export the
//! [`language`] receiver, which belongs to no part and no domain:
//! [`PluginEngine::set_language`] gives the
//! plugins the language of OTTY's interface at each call, and
//! [`LoadedPlugin::language_changed`] tells a plugin that it changed.
//!
//! A plugin never holds a key, only a [`KeyHandle`]: where the key sits.
//! The host reads the key there for each use (its fingerprint, a prefix,
//! the claims of a JWT, a request header) and drops it right after.
//! `http.send` reaches only the hosts of the called part, and
//! `keyring.read` only its entries once the user allowed the keyring
//! ([`PluginEngine::set_keyring_allowed`]).
//!
//! The host starts no program itself: the app gives the engine a
//! [`StartProgram`] and a [`StartTerminal`], and the host checks the
//! called part's `run` and `terminal` permissions, keeps the time and
//! output limits, and stops what a call started when it ends. A limit
//! source that ran a program or a terminal is paced: at most once in five
//! minutes per profile.

pub mod agents;
mod cache;
mod call_programs;
mod call_terminals;
mod called_part;
mod calls;
mod contract;
mod engine;
#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;
mod https;
mod imports;
mod keys;
pub mod language;
mod memory_cap;
pub mod operations;
mod outside;
mod pacing;
mod part_health;
mod plugin;
mod plugin_failure;
mod plugin_log;
mod plugin_part;
mod program_paths;
mod programs;
mod system_keyring;
mod terminal;
#[cfg(test)]
mod testing;

pub use otty_plugin_manifest::{
    AgentPart, Contract, ContractVersion, Domain, Environment, GlobProblem,
    JsonPointer, KeyPlace, KeyRule, KeyringRule, Manifest, ManifestError,
    PartKind, PartPermissions, PartRef, PathGlob, PermissionLine, Permissions,
    PluginInfo, PluginVersion, ProgramInstall, ProgramPlace, ProgramRule,
    ProviderPart,
};
/// The error wasmtime gives inside a call.
pub use wasmtime::Error;

pub use crate::agents::agent::AgentCalls;
pub use crate::agents::provider::ProviderCalls;
pub use crate::cache::LoadKind;
pub use crate::call_programs::TalkHandle;
pub use crate::call_terminals::TerminalHandle;
pub use crate::called_part::CalledPart;
pub use crate::calls::{CallBatch, CallError, CallKind, CallState};
pub use crate::contract::{ContractError, ExportedApi};
pub use crate::engine::{EngineError, LoadError, PluginEngine, TICK};
pub use crate::keys::{KeyHandle, KeySource, KeyText};
pub use crate::memory_cap::MEMORY_CAP;
/// A file `files.list` found: its path, size and change time.
pub use crate::operations::v0_1::otty::host::files::FileInfo;
pub use crate::part_health::{LastFailure, PAUSE_AFTER, PartHealth};
pub use crate::plugin::LoadedPlugin;
pub use crate::plugin_failure::{PluginFailure, PluginFailureKind};
pub use crate::plugin_log::{LOG_CAP, PluginLog};
pub use crate::plugin_part::PluginPart;
pub use crate::programs::{
    LineOutput, ProgramCommand, ProgramError, RunOutput, RunningProgram,
    StartProgram,
};
pub use crate::terminal::{RunningTerminal, ScreenOutput, StartTerminal};
