//! Write OTTY plugins in Rust (spec 2026-11-19-agent-plugins, "SDK").
//!
//! A plugin is one WebAssembly component, built for `wasm32-wasip2`, with
//! one or more parts listed in its `otty-plugin.toml`. Each part is an
//! extension point of a domain of OTTY; the extension points of the
//! `agents` domain, Agents mode, are in [`agents`]: its agent parts are
//! served by one [`agents::Agent`] implementation and its provider parts
//! by one [`agents::Provider`] implementation; OTTY names the part in
//! each call. A plugin whose texts come in several languages also
//! implements [`Language`], which belongs to no part and no domain: OTTY
//! calls it when the user changes the language of its interface.
//!
//! The plugin reads what it needs through the host operations of the
//! modules at the root ([`files`], [`sqlite`], [`http`], [`programs`],
//! [`terminal`], [`keyring`], [`env`], [`clock`], [`log`] and [`keys`]),
//! which no domain owns; each call is checked against the permissions of
//! the part being called. A key is never read: the plugin holds a [`Key`]
//! handle. What only the user can do is a [`Need`], the same for every
//! domain.
//!
//! The plugin ends with one line that names its type and the extension
//! points it exports; they must match its `[api]`:
//!
//! ```ignore
//! otty_plugin_sdk::export!(Example with agents::agent, agents::provider, language);
//! ```
//!
//! Plugin tests run natively with plain `cargo test`: the host operations
//! then come from the [`test_host`], which serves the plugin's
//! `fixtures/` folder and canned HTTP answers.
//!
//! Each trait maps to its own versioned WIT contract in `wit/`:
//! `otty:agents-agent@0.1.0` and `otty:agents-provider@0.1.0`, which
//! import the host operations of `otty:host@0.1.0`, and
//! `otty:language@0.1.0`, which imports nothing. The types are the
//! contracts' types as `wit-bindgen` makes them.

// A `ReadError` may hold a need, a record the contract can not box.
#![allow(clippy::result_large_err)]

pub mod agents;
#[doc(hidden)]
pub mod bindings;
pub mod clock;
pub mod env;
pub mod files;
pub mod http;
pub mod keyring;
pub mod keys;
mod language;
pub mod log;
pub mod programs;
pub mod sqlite;
pub mod terminal;
#[cfg(not(target_arch = "wasm32"))]
pub mod test_host;
#[cfg(test)]
mod testing;

/// Where a key sits, as a part names it.
pub use crate::bindings::host::otty::host::keys::{
    JsonField, KeyPlace, KeyringEntry, SqliteField,
};
/// The values the host operations and the contracts of every domain
/// share, among them the needs a part gives: one thing only the user can
/// do, with one button, in the language of [`env::language`].
pub use crate::bindings::host::otty::host::types::{
    Command, HostError, KeyAsk, Need, NeedAction,
};
pub use crate::keys::{GivenKey, JsonWithKeys, Key, given_key};
pub use crate::language::Language;
#[doc(hidden)]
pub use crate::language::LanguageExport;

/// Exports the plugin type and the extension points it implements, the
/// contracts its manifest's `[api]` names, in any order: a point of a
/// domain by its domain and name, such as `agents::agent` and
/// `agents::provider`, and `language` for a plugin with translations:
/// `export!(Example with agents::agent)`,
/// `export!(Example with agents::provider)`,
/// `export!(Example with agents::agent, agents::provider)` or
/// `export!(Example with agents::agent, agents::provider, language)`.
///
/// Inside, it uses the `wit-bindgen` binding of each trait; when the
/// component is made, their worlds join. Outside `wasm32` it only checks
/// that the type implements the traits, so plugin tests build natively.
#[macro_export]
macro_rules! export {
    ($plugin:ident with $($first:ident $(:: $rest:ident)*),+ $(,)?) => {
        $( $crate::__export_contract!($plugin, $first $(:: $rest)*); )+
    };
}

/// One contract of [`export!`].
#[doc(hidden)]
#[macro_export]
macro_rules! __export_contract {
    ($plugin:ident, agents::agent) => {
        const _: () = {
            fn implements_agent<T: $crate::agents::Agent>() {}

            #[allow(dead_code)]
            fn check() {
                implements_agent::<$plugin>();
            }

            #[cfg(target_arch = "wasm32")]
            type OttyAgentExport = $crate::agents::AgentExport<$plugin>;

            #[cfg(target_arch = "wasm32")]
            $crate::bindings::agents_agent::export_agent!(
                OttyAgentExport with_types_in $crate::bindings::agents_agent
            );
        };
    };
    ($plugin:ident, agents::provider) => {
        const _: () = {
            fn implements_provider<T: $crate::agents::Provider>() {}

            #[allow(dead_code)]
            fn check() {
                implements_provider::<$plugin>();
            }

            #[cfg(target_arch = "wasm32")]
            type OttyProviderExport = $crate::agents::ProviderExport<$plugin>;

            #[cfg(target_arch = "wasm32")]
            $crate::bindings::agents_provider::export_provider!(
                OttyProviderExport with_types_in $crate::bindings::agents_provider
            );
        };
    };
    ($plugin:ident, language) => {
        const _: () = {
            fn implements_language<T: $crate::Language>() {}

            #[allow(dead_code)]
            fn check() {
                implements_language::<$plugin>();
            }

            #[cfg(target_arch = "wasm32")]
            type OttyLanguageExport = $crate::LanguageExport<$plugin>;

            #[cfg(target_arch = "wasm32")]
            $crate::bindings::language::export_language!(
                OttyLanguageExport with_types_in $crate::bindings::language
            );
        };
    };
}
