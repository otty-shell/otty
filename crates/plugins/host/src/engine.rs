//! One wasmtime engine for all plugins: the sync API, Cranelift, the
//! component model, epoch interruption with a ticker thread, and the
//! compiled cache. Loading a plugin checks its imports and exports.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Weak};
use std::thread;
use std::time::Duration;

use otty_plugin_manifest::{Environment, Manifest};
use wasmtime::component::{HasSelf, Linker};
use wasmtime::{Config, Engine};

use crate::cache::CompiledCache;
use crate::calls::CallState;
use crate::contract::{ContractExports, ExportedApi};
use crate::imports::refused_imports;
use crate::operations::v0_1::Operations;
use crate::outside::Outside;
use crate::plugin::LoadedPlugin;
use crate::programs::StartProgram;
use crate::terminal::StartTerminal;

/// How often the epoch moves on: a call's deadline is counted in these
/// ticks (the spike: 1.005 s for a 1 s deadline).
pub const TICK: Duration = Duration::from_millis(10);

/// The one engine of all plugins, and the ticker thread that moves its
/// epoch on. The thread stops once the engine and every plugin loaded by
/// it are dropped.
pub struct PluginEngine {
    shared: Arc<EngineShared>,
    starters: Starters,
}

impl PluginEngine {
    /// An engine whose compiled plugins are kept in `cache_folder`, such
    /// as `~/.cache/otty/plugins/`. Its plugins reach the network and,
    /// once the user allows it, the system keyring.
    pub fn new(cache_folder: PathBuf) -> Result<Self, EngineError> {
        Self::with_outside(cache_folder, Outside::real())
    }

    /// Lets plugins read the keyring entries their parts declare, or
    /// stops them, as the user chose in Agents mode. Until then every
    /// keyring read is refused.
    pub fn set_keyring_allowed(&self, allowed: bool) {
        self.shared.outside.set_keyring_allowed(allowed);
    }

    /// An engine whose plugins reach the network and the keyring through
    /// `outside`, such as mocks in tests.
    pub(crate) fn with_outside(
        cache_folder: PathBuf,
        outside: Outside,
    ) -> Result<Self, EngineError> {
        let mut config = Config::new();
        config.epoch_interruption(true);
        let engine = Engine::new(&config)
            .map_err(|error| EngineError::Setup(error.to_string()))?;

        let mut linker = Linker::new(&engine);
        wasmtime_wasi::p2::add_to_linker_sync(&mut linker)
            .map_err(|error| EngineError::Setup(error.to_string()))?;
        Operations::add_to_linker::<CallState, HasSelf<CallState>>(
            &mut linker,
            |state| state,
        )
        .map_err(|error| EngineError::Setup(error.to_string()))?;

        let shared = Arc::new(EngineShared {
            engine,
            linker,
            cache: CompiledCache::new(cache_folder),
            outside: Arc::new(outside),
        });
        start_ticker(Arc::downgrade(&shared))?;

        Ok(Self {
            shared,
            starters: Starters::default(),
        })
    }

    /// The same engine whose plugins run programs with `programs`, the
    /// app's way of starting them. Without it every program is refused.
    pub fn with_programs(mut self, programs: Arc<dyn StartProgram>) -> Self {
        self.starters.programs = Some(programs);
        self
    }

    /// The same engine whose plugins start hidden terminals with
    /// `terminals`, the app's way of starting them. Without it every
    /// hidden terminal is refused.
    pub fn with_terminals(mut self, terminals: Arc<dyn StartTerminal>) -> Self {
        self.starters.terminals = Some(terminals);
        self
    }

    /// The plugin of `manifest` whose component is the file `wasm`, its
    /// permissions made concrete for `env`. The component comes from the
    /// cache when it was compiled before; it is refused when it imports
    /// what OTTY does not give or exports other contracts than `[api]`.
    pub fn load(
        &self,
        manifest: Manifest,
        wasm: &Path,
        env: &Environment,
    ) -> Result<LoadedPlugin, LoadError> {
        let bytes = std::fs::read(wasm)
            .map_err(|error| LoadError::Read(error.to_string()))?;

        let (component, load_kind) = self
            .shared
            .cache
            .load(&self.shared.engine, &bytes)
            .map_err(|error| LoadError::Compile(error.to_string()))?;

        let engine = &self.shared.engine;
        let component_type = component.component_type();
        let refused = refused_imports(
            component_type.imports(engine).map(|(name, _)| name),
        );
        if !refused.is_empty() {
            return Err(LoadError::RefusedImports(refused));
        }

        let exported = ExportedApi::of_exports(
            component_type.exports(engine).map(|(name, _)| name),
        );
        let matches = exported
            .as_ref()
            .is_some_and(|exported| exported.matches(manifest.api()));
        if !matches {
            return Err(LoadError::ApiMismatch {
                declared: manifest.api().to_string(),
                exported: exported.map_or_else(
                    || "one contract in two versions".to_string(),
                    |exported| exported.to_string(),
                ),
            });
        }

        let instance_pre = self
            .shared
            .linker
            .instantiate_pre(&component)
            .map_err(|error| LoadError::Link(error.to_string()))?;
        let exports = ContractExports::of_api(manifest.api(), &instance_pre)?;

        LoadedPlugin::new(
            Arc::clone(&self.shared),
            manifest,
            instance_pre,
            exports,
            load_kind,
            env,
            self.starters.clone(),
        )
        .map_err(|error| LoadError::Log(error.to_string()))
    }

    /// Removes the compiled copy of the `.wasm` whose SHA-256 is
    /// `wasm_sha256` (lowercase hex) from the cache, as removing or
    /// updating a plugin does. A copy that is not there is not an error.
    pub fn remove_compiled(&self, wasm_sha256: &str) -> std::io::Result<()> {
        self.shared.cache.remove(&self.shared.engine, wasm_sha256)
    }
}

/// What the engine and the plugins it loaded share.
pub(crate) struct EngineShared {
    engine: Engine,
    linker: Linker<CallState>,
    cache: CompiledCache,
    outside: Arc<Outside>,
}

impl EngineShared {
    pub(crate) fn engine(&self) -> &Engine {
        &self.engine
    }

    pub(crate) fn outside(&self) -> &Arc<Outside> {
        &self.outside
    }
}

/// What the app gives the host to start programs and hidden terminals
/// with; the plugins of an engine without them start none.
#[derive(Clone, Default)]
pub(crate) struct Starters {
    programs: Option<Arc<dyn StartProgram>>,
    terminals: Option<Arc<dyn StartTerminal>>,
}

impl Starters {
    /// How programs are started, if the app gave a way.
    pub(crate) fn programs(&self) -> Option<Arc<dyn StartProgram>> {
        self.programs.clone()
    }

    /// How hidden terminals are started, if the app gave a way.
    pub(crate) fn terminals(&self) -> Option<Arc<dyn StartTerminal>> {
        self.terminals.clone()
    }
}

/// Why the engine could not be set up.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EngineError {
    /// wasmtime refused the settings, or WASI could not be linked.
    #[error("The plugin engine could not be set up: {0}")]
    Setup(String),
    /// The ticker thread could not start.
    #[error("The plugin engine's clock could not start: {0}")]
    Ticker(String),
}

/// Why a plugin was not loaded, in plain words.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LoadError {
    /// The `.wasm` file could not be read.
    #[error("The plugin's .wasm can not be read: {0}")]
    Read(String),
    /// The `.wasm` is not a component wasmtime can compile.
    #[error("The plugin's .wasm is not a component OTTY can run: {0}")]
    Compile(String),
    /// The component imports what OTTY does not give, by name.
    #[error("The plugin uses what OTTY does not give: {}", .0.join(", "))]
    RefusedImports(Vec<String>),
    /// The component exports other contracts than the manifest's `[api]`.
    #[error(
        "The manifest's [api] names {declared}, but the .wasm exports {exported}"
    )]
    ApiMismatch {
        /// The contracts of `[api]`.
        declared: String,
        /// The contracts the component exports.
        exported: String,
    },
    /// An allowed import is not one this OTTY gives.
    #[error("The plugin needs what this OTTY does not give: {0}")]
    Link(String),
    /// The plugin's log could not be set up.
    #[error("The plugin's log could not be set up: {0}")]
    Log(String),
}

/// Starts the thread that moves the epoch on every [`TICK`] while the
/// engine or one of its plugins lives.
fn start_ticker(shared: Weak<EngineShared>) -> Result<(), EngineError> {
    thread::Builder::new()
        .name("otty-plugin-epoch".to_string())
        .spawn(move || {
            while let Some(shared) = shared.upgrade() {
                shared.engine.increment_epoch();
                drop(shared);

                thread::sleep(TICK);
            }
        })
        .map(|_| ())
        .map_err(|error| EngineError::Ticker(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::LoadKind;
    use crate::testing::{
        TEST_PLUGIN_MANIFEST, TempDir, edited, fixture, load_fixture, test_env,
    };

    fn load(
        engine: &PluginEngine,
        manifest: &str,
        name: &str,
    ) -> Result<LoadedPlugin, LoadError> {
        let manifest = Manifest::parse(manifest).expect("valid");

        engine.load(manifest, &fixture(name), &test_env())
    }

    #[test]
    fn a_minimal_plugin_loads_and_its_second_load_comes_from_the_cache() {
        let cache = TempDir::new();
        let engine =
            PluginEngine::new(cache.path().to_path_buf()).expect("engine");

        let first = load_fixture(&engine, "minimal");
        let second = load_fixture(&engine, "minimal");

        assert_eq!(first.load_kind(), LoadKind::Compiled);
        assert_eq!(second.load_kind(), LoadKind::Cached);
        assert_eq!(second.manifest().plugin().id(), "test-plugin");
    }

    #[test]
    fn a_plugin_that_imports_the_filesystem_is_refused_by_name() {
        let cache = TempDir::new();
        let engine =
            PluginEngine::new(cache.path().to_path_buf()).expect("engine");

        let error = load(&engine, TEST_PLUGIN_MANIFEST, "read-files")
            .err()
            .expect("refused");

        let LoadError::RefusedImports(refused) = &error else {
            panic!("refused for its imports: {error}");
        };
        assert!(
            refused
                .iter()
                .any(|name| name.starts_with("wasi:filesystem/")),
            "{refused:?}"
        );
        assert!(error.to_string().contains("wasi:filesystem/"));
    }

    #[test]
    fn api_that_does_not_match_the_exports_is_refused() {
        let cache = TempDir::new();
        let engine =
            PluginEngine::new(cache.path().to_path_buf()).expect("engine");
        let with_provider = format!(
            "{}\n[[provider]]\nid = \"test-provider\"\nname = \"Test \
             provider\"\nicon = \"test.svg\"\nhosts = [\"example.com\"]\n",
            edited(
                TEST_PLUGIN_MANIFEST,
                "agent = \"0.1\"",
                "agent = \"0.1\"\nprovider = \"0.1\""
            ),
        );

        let error = load(&engine, &with_provider, "minimal")
            .err()
            .expect("refused");

        assert_eq!(
            error,
            LoadError::ApiMismatch {
                declared: "agent 0.1, provider 0.1".to_string(),
                exported: "agent 0.1.0".to_string(),
            }
        );
    }

    #[test]
    fn a_file_that_is_not_a_component_is_refused() {
        let cache = TempDir::new();
        let engine =
            PluginEngine::new(cache.path().to_path_buf()).expect("engine");
        let wasm = cache.path().join("broken.wasm");
        std::fs::write(&wasm, b"not wasm").expect("write");
        let manifest = Manifest::parse(TEST_PLUGIN_MANIFEST).expect("valid");

        let error = engine
            .load(manifest, &wasm, &test_env())
            .err()
            .expect("refused");

        assert!(matches!(error, LoadError::Compile(_)));
    }
}
