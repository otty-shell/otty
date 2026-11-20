//! The official OpenRouter plugin of OTTY. Its provider part `openrouter`
//! reads the limit of an OpenRouter key and how much of it was used, with
//! the key itself: a key added by hand, or one an agent part found for
//! `openrouter.ai`. OTTY hands the part a handle to the key, never the
//! key.

mod key_limit;

/// The plugin: one type serves the provider part its manifest lists.
pub struct OpenRouter;

otty_plugin_sdk::export!(OpenRouter with provider);
