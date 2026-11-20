//! The official DeepSeek plugin of OTTY. Its provider part `deepseek`
//! reads the money left on a DeepSeek account with the account's key:
//! a key added by hand, or one an agent part found, such as the key of
//! a Claude Code profile that talks to `api.deepseek.com`. OTTY hands the
//! part a handle to the key, never the key.

mod balance;

/// The plugin: one type serves the provider part its manifest lists.
pub struct DeepSeek;

otty_plugin_sdk::export!(DeepSeek with provider);
