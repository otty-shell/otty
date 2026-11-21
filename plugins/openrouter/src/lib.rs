//! The official OpenRouter plugin of OTTY. Its provider part `openrouter`
//! reads the limit of an OpenRouter key and how much of it was used, with
//! the key itself: a key added by hand, or one an agent part found for
//! `openrouter.ai`. OTTY hands the part a handle to the key, never the
//! key.
//!
//! The texts of its needs come in every language of OTTY, so the plugin
//! exports the language receiver.

// A `ReadError` may hold a need, a record the contract can not box.
#![allow(clippy::result_large_err)]

mod key_limit;
mod needs;

use otty_plugin_sdk::Language;

/// The plugin: one type serves the provider part its manifest lists.
pub struct OpenRouter;

impl Language for OpenRouter {
    /// The texts of the needs come in every language of OTTY.
    fn language_changed(_tag: &str) -> bool {
        true
    }
}

otty_plugin_sdk::export!(OpenRouter with agents::provider, language);
