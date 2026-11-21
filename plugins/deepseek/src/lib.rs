//! The official DeepSeek plugin of OTTY. Its provider part `deepseek`
//! reads the money left on a DeepSeek account with the account's key:
//! a key added by hand, or one an agent part found, such as the key of
//! a Claude Code profile that talks to `api.deepseek.com`. OTTY hands the
//! part a handle to the key, never the key.
//!
//! The texts of its needs come in every language of OTTY, so the plugin
//! exports the language receiver.

// A `ReadError` may hold a need, a record the contract can not box.
#![allow(clippy::result_large_err)]

mod balance;
mod needs;

use otty_plugin_sdk::Language;

/// The plugin: one type serves the provider part its manifest lists.
pub struct DeepSeek;

impl Language for DeepSeek {
    /// The texts of the needs come in every language of OTTY.
    fn language_changed(_tag: &str) -> bool {
        true
    }
}

otty_plugin_sdk::export!(DeepSeek with agents::provider, language);
