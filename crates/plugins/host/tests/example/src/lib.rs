//! The example test plugin of `otty-plugin-host`: the plugin template of
//! the `otty-plugin-template` repository, one agent part over a made-up
//! coding agent that keeps its files in `~/.example-agent/` and one
//! provider part that reads the balance of a made-up provider, plus the
//! `/usage` read of a login with no key in a hidden terminal. Its texts
//! come in English and Russian, so it also exports the language receiver.
//! The tests of OTTY run it where they need a whole plugin.

// A `ReadError` may hold a need, a record the contract can not box.
#![allow(clippy::result_large_err)]

mod agent;
mod needs;
mod provider;
mod session_file;

use otty_plugin_sdk::Language;

/// The plugin: one type serves every part its manifest lists.
pub struct Example;

impl Language for Example {
    /// The texts of the needs depend on the language, so OTTY reads the
    /// parts again to show them in the new one.
    fn language_changed(_tag: &str) -> bool {
        true
    }
}

otty_plugin_sdk::export!(Example with agents::agent, agents::provider, language);
