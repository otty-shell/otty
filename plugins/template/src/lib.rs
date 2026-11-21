//! The OTTY plugin template: one agent part over a made-up coding agent
//! that keeps its files in `~/.example-agent/`, and one provider part
//! that reads the balance of a made-up provider. Its texts come in the ten
//! languages of OTTY, so it also exports the language receiver. Copy it,
//! rename the ids in `otty-plugin.toml`, and replace what the parts read.

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
    /// The texts of the needs come in every language of OTTY, so OTTY
    /// reads the parts again to show them in the new one.
    fn language_changed(_tag: &str) -> bool {
        true
    }
}

otty_plugin_sdk::export!(Example with agents::agent, agents::provider, language);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_texts_depend_on_the_language() {
        assert!(Example::language_changed("ru"));
    }
}
