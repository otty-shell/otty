//! Write OTTY plugins in Rust (spec 2026-11-19-agent-plugins, "SDK").
//!
//! A plugin is one WebAssembly component, built for `wasm32-wasip2`, with
//! one or more parts listed in its `otty-plugin.toml`. Its agent parts are
//! served by one [`Agent`] implementation and its provider parts by one
//! [`Provider`] implementation; OTTY names the part in each call. The
//! plugin reads what it needs through the host operations of the modules
//! below ([`files`], [`sqlite`], [`http`], [`programs`], [`terminal`],
//! [`keyring`], [`env`], [`clock`], [`log`] and [`keys`]); each call is
//! checked against the permissions of the part being called. A key is
//! never read: the plugin holds a [`Key`] handle.
//!
//! The plugin ends with one line that names its type and the traits it
//! exports; they must match its `[api]`:
//!
//! ```ignore
//! otty_plugin_sdk::export!(Example with agent, provider);
//! ```
//!
//! Plugin tests run natively with plain `cargo test`: the host operations
//! then come from the [`test_host`], which serves the plugin's
//! `fixtures/` folder and canned HTTP answers.
//!
//! Each trait maps to its own versioned WIT contract in `wit/`:
//! `otty:agent@0.1.0` and `otty:provider@0.1.0`, which import the host
//! operations of `otty:host@0.1.0`. The types below are the contracts'
//! types as `wit-bindgen` makes them.

mod agent;
#[doc(hidden)]
pub mod bindings;
pub mod clock;
pub mod env;
pub mod files;
pub mod http;
pub mod keyring;
pub mod keys;
pub mod log;
pub mod programs;
mod provider;
mod record_counts;
pub mod sqlite;
pub mod terminal;
#[cfg(not(target_arch = "wasm32"))]
pub mod test_host;
#[cfg(test)]
mod testing;

pub use crate::agent::Agent;
#[doc(hidden)]
pub use crate::agent::AgentExport;
/// The types of the agent contract.
pub use crate::bindings::agent::exports::otty::agent::agent::{
    Answer, Answers, Billing, Command, Connection, Login, LoginRef,
    ModelTokens, Profile, ProfilePlace, ProfileStatus, ProjectStats,
    ProviderRef, RecordCounts, Session, SessionStats, SpendItem, SpendKind,
    SpendPlace, TokenCount,
};
/// Where a key sits, as an agent part names it.
pub use crate::bindings::host::otty::host::keys::{
    JsonField, KeyPlace, KeyringEntry, SqliteField,
};
/// The values the host operations and both contracts share.
pub use crate::bindings::host::otty::host::types::{
    Credits, HostError, Identity, LimitWindow, Limits, LoginKind, ReadError,
};
/// The types of the provider contract.
pub use crate::bindings::provider::exports::otty::provider::provider::{
    AccountFacts, AccountNumbers, Bill, DayCost, DayRange, KeyLimit,
    LoginFacts, Money,
};
pub use crate::keys::{JsonWithKeys, Key};
pub use crate::provider::Provider;
#[doc(hidden)]
pub use crate::provider::ProviderExport;

/// Exports the plugin type and the traits it implements, the contracts
/// its manifest's `[api]` names: `export!(Example with agent)`,
/// `export!(Example with provider)` or
/// `export!(Example with agent, provider)`.
///
/// Inside, it uses the `wit-bindgen` binding of each trait; when the
/// component is made, the worlds of both join. Outside `wasm32` it only
/// checks that the type implements the traits, so plugin tests build
/// natively.
#[macro_export]
macro_rules! export {
    ($plugin:ident with agent) => {
        $crate::__export_agent!($plugin);
    };
    ($plugin:ident with provider) => {
        $crate::__export_provider!($plugin);
    };
    ($plugin:ident with agent, provider) => {
        $crate::__export_agent!($plugin);
        $crate::__export_provider!($plugin);
    };
}

/// The `agent` half of [`export!`].
#[doc(hidden)]
#[macro_export]
macro_rules! __export_agent {
    ($plugin:ident) => {
        const _: () = {
            fn implements_agent<T: $crate::Agent>() {}

            #[allow(dead_code)]
            fn check() {
                implements_agent::<$plugin>();
            }

            #[cfg(target_arch = "wasm32")]
            type OttyAgentExport = $crate::AgentExport<$plugin>;

            #[cfg(target_arch = "wasm32")]
            $crate::bindings::agent::export_agent!(
                OttyAgentExport with_types_in $crate::bindings::agent
            );
        };
    };
}

/// The `provider` half of [`export!`].
#[doc(hidden)]
#[macro_export]
macro_rules! __export_provider {
    ($plugin:ident) => {
        const _: () = {
            fn implements_provider<T: $crate::Provider>() {}

            #[allow(dead_code)]
            fn check() {
                implements_provider::<$plugin>();
            }

            #[cfg(target_arch = "wasm32")]
            type OttyProviderExport = $crate::ProviderExport<$plugin>;

            #[cfg(target_arch = "wasm32")]
            $crate::bindings::provider::export_provider!(
                OttyProviderExport with_types_in $crate::bindings::provider
            );
        };
    };
}
