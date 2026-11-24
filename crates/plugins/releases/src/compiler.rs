//! The check and the compile of a plugin's `.wasm` against its manifest,
//! done by the plugin engine of `otty-plugin-host`.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use otty_plugin_host::{Environment, Manifest, PluginEngine};

/// Checks a plugin's `.wasm` and keeps its compiled copy in the cache.
///
/// It is a trait because the real one runs wasmtime's compiler, which
/// takes about a second per plugin: the install and update rules are
/// tested with a mock, and the real engine is tested once on the template
/// plugin.
#[cfg_attr(test, mockall::automock)]
pub trait PluginCompiler: Send {
    /// Checks that the component at `wasm` imports only what OTTY gives
    /// and exports the contracts of the manifest's parts, each in a
    /// version this OTTY knows, and compiles it into the cache. The error
    /// is a plain reason.
    fn compile(&self, manifest: &Manifest, wasm: &Path) -> Result<(), String>;

    /// Removes the compiled copy of the `.wasm` whose SHA-256 is
    /// `wasm_sha256` from the cache.
    fn remove_compiled(&self, wasm_sha256: &str) -> Result<(), String>;
}

impl PluginCompiler for PluginEngine {
    fn compile(&self, manifest: &Manifest, wasm: &Path) -> Result<(), String> {
        // The check needs no real folder: nothing is called.
        let env = Environment::new(PathBuf::from("/"));

        self.load(manifest.clone(), wasm, &env)
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    fn remove_compiled(&self, wasm_sha256: &str) -> Result<(), String> {
        PluginEngine::remove_compiled(self, wasm_sha256)
            .map_err(|error| error.to_string())
    }
}

/// The engine the app shares between the installer and the plugins it
/// runs, so a plugin is compiled once and loaded from the same cache.
impl PluginCompiler for Arc<PluginEngine> {
    fn compile(&self, manifest: &Manifest, wasm: &Path) -> Result<(), String> {
        PluginCompiler::compile(self.as_ref(), manifest, wasm)
    }

    fn remove_compiled(&self, wasm_sha256: &str) -> Result<(), String> {
        PluginCompiler::remove_compiled(self.as_ref(), wasm_sha256)
    }
}
