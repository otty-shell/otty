//! Runs OTTY plugins: WebAssembly components built for `wasm32-wasip2`
//! that teach OTTY about an agent or a provider (spec
//! 2026-11-19-agent-plugins).
//!
//! A plugin is described by its [`Manifest`], `otty-plugin.toml`, read and
//! checked by `otty-plugin-manifest`: the plugin, the contract version of
//! each kind of part in `[api]`, and its parts, each with its own
//! [`PartPermissions`].
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
//! [`CallBatch::agent`] and [`CallBatch::provider`] call the functions of
//! a contract with the types of its newest version, [`agent`] and
//! [`provider`]; each version the host keeps has its own bindings, such
//! as [`agent::v0_1`]. The host operations a plugin imports are
//! [`operations`].
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

pub mod agent;
mod cache;
mod call_programs;
mod call_terminals;
mod calls;
mod contract;
mod engine;
mod https;
mod imports;
mod keys;
mod memory_cap;
pub mod operations;
mod outside;
mod pacing;
mod plugin;
mod plugin_log;
mod programs;
pub mod provider;
mod system_keyring;
mod terminal;
#[cfg(test)]
mod testing;

pub use otty_plugin_manifest::{
    AddByHand, AgentPart, Api, ContractVersion, Environment, GlobProblem,
    JsonPointer, KeyPlace, KeyRule, KeyringRule, Manifest, ManifestError,
    PartKind, PartPermissions, PartRef, PathGlob, PermissionLine, Permissions,
    PluginInfo, PluginVersion, ProgramRule, ProviderPart,
};
/// The error wasmtime gives inside a call.
pub use wasmtime::Error;

pub use crate::agent::AgentCalls;
pub use crate::cache::LoadKind;
pub use crate::call_programs::TalkHandle;
pub use crate::call_terminals::TerminalHandle;
pub use crate::calls::{CallBatch, CallError, CallKind, CallState};
pub use crate::contract::ExportedApi;
pub use crate::engine::{EngineError, LoadError, PluginEngine, TICK};
pub use crate::keys::KeyHandle;
pub use crate::memory_cap::MEMORY_CAP;
pub use crate::plugin::LoadedPlugin;
pub use crate::plugin_log::{LOG_CAP, PluginLog};
pub use crate::programs::{
    LineOutput, ProgramCommand, ProgramError, RunOutput, RunningProgram,
    StartProgram,
};
pub use crate::provider::ProviderCalls;
pub use crate::terminal::{RunningTerminal, ScreenOutput, StartTerminal};
