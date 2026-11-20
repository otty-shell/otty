//! The OTTY plugin template: one agent part over a made-up coding agent
//! that keeps its files in `~/.example-agent/`, and one provider part
//! that reads the balance of a made-up provider. Copy it, rename the ids
//! in `otty-plugin.toml`, and replace what the parts read.

mod agent;
mod provider;
mod session_file;

/// The plugin: one type serves every part its manifest lists.
pub struct Example;

otty_plugin_sdk::export!(Example with agent, provider);
