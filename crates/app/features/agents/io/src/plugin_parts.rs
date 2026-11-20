//! The plugin parts Agents mode reads with (spec 2026-11-19-agent-plugins,
//! "How plugins talk to OTTY"): the folder of each installed plugin holds
//! its manifest, its `.wasm` and its icons; the engine loads it, and its
//! parts that are on join the readers built into OTTY in one list of
//! agents, one list of providers and one registry.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use otty_agents::{Agent, PartHealth, PluginAgent, PluginProvider, Registry};
use otty_plugin_host::{
    Environment, LoadedPlugin, Manifest, PartKind, PartRef, PluginEngine,
    PluginLog,
};

/// The file of a plugin's folder that holds its manifest.
const MANIFEST_FILE: &str = "otty-plugin.toml";

/// The provider parts of plugins, shared by the reads that run on a
/// background thread.
pub type ProviderList = Arc<[PluginProvider]>;

/// One installed plugin as Agents mode loads it: the folder of its
/// installed version, with its manifest, `.wasm` and icons, and its parts
/// that are on. A plain parameter bag: the Plugins page knows which parts
/// are on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledParts {
    /// The folder of the installed version of the plugin.
    pub folder: PathBuf,
    /// Its parts that are on; the others are not loaded.
    pub on: Vec<PartRef>,
}

/// The readers of Agents mode: the agents, plugin parts first, the
/// provider parts, the registry that names them all, and the plugins that
/// run. A plain bag of what [`PluginParts::into_readers`] gives.
pub struct Readers {
    /// The agents, each served once: a built-in reader of an agent a part
    /// serves is left out.
    pub agents: Vec<Box<dyn Agent>>,
    /// The provider parts that are on.
    pub providers: Vec<PluginProvider>,
    /// The agents and providers OTTY knows.
    pub registry: Registry,
    /// Each loaded plugin with its log and the health of its parts.
    pub running: Vec<RunningParts>,
}

/// One loaded plugin: its log and the health of each of its parts that
/// was loaded, which the Plugins page shows and starts again. A plain bag
/// of shared handles.
#[derive(Debug, Clone)]
pub struct RunningParts {
    /// The plugin id.
    pub plugin: String,
    /// What the plugin writes to its log.
    pub log: PluginLog,
    /// Each loaded part with its health.
    pub parts: Vec<(PartRef, PartHealth)>,
}

/// The parts of the installed plugins that are on, loaded and ready to
/// read.
#[derive(Default)]
pub struct PluginParts {
    agents: Vec<PluginAgent>,
    providers: Vec<PluginProvider>,
    running: Vec<RunningParts>,
}

impl PluginParts {
    /// The agent parts.
    pub fn agents(&self) -> &[PluginAgent] {
        &self.agents
    }

    /// The provider parts.
    pub fn providers(&self) -> &[PluginProvider] {
        &self.providers
    }

    /// The parts that are on of the plugins `installed`, loaded by
    /// `engine`. The parts see the home folder `home` and the variables
    /// they list in `env`, with the values `variable` gives. A plugin that
    /// can not be read or loaded is left out, with a warning in the log.
    pub fn load(
        engine: &PluginEngine,
        installed: &[InstalledParts],
        home: PathBuf,
        variable: impl Fn(&str) -> Option<String>,
    ) -> Self {
        let manifests: Vec<(&InstalledParts, Manifest)> = installed
            .iter()
            .filter_map(|plugin| Some((plugin, read_manifest(&plugin.folder)?)))
            .collect();
        let env = plugin_env(
            home,
            manifests.iter().map(|(_, manifest)| manifest),
            variable,
        );

        let mut parts = Self::default();
        for (plugin, manifest) in manifests {
            let wasm = plugin.folder.join(manifest.plugin().wasm());
            let id = manifest.plugin().id().to_string();
            let loaded = match engine.load(manifest, &wasm, &env) {
                Ok(loaded) => Arc::new(loaded),
                Err(error) => {
                    log::warn!("the plugin {id} was not loaded: {error}");
                    continue;
                },
            };

            parts.add_parts(&loaded, plugin);
        }

        parts
    }

    /// The readers of Agents mode: these parts, then the readers built
    /// into OTTY, `built_in`, for what no part serves.
    pub fn into_readers(self, built_in: Vec<Box<dyn Agent>>) -> Readers {
        let mut agents: Vec<Box<dyn Agent>> = self
            .agents
            .into_iter()
            .map(|agent| Box::new(agent) as Box<dyn Agent>)
            .collect();
        agents.extend(built_in);

        let registry = Registry::of_readers(&agents, &self.providers);
        let agents = registry.listed_agents(agents);
        Readers {
            agents,
            providers: self.providers,
            registry,
            running: self.running,
        }
    }

    /// Adds the parts of `plugin` that `installed` has on.
    fn add_parts(
        &mut self,
        plugin: &Arc<LoadedPlugin>,
        installed: &InstalledParts,
    ) {
        let manifest = plugin.manifest();
        let mut running = RunningParts {
            plugin: manifest.plugin().id().to_string(),
            log: plugin.log().clone(),
            parts: Vec::new(),
        };

        for part in &installed.on {
            let icon = icon_of(manifest, part).and_then(|icon| {
                std::fs::read(installed.folder.join(icon)).ok()
            });

            match part.kind() {
                PartKind::Agent => {
                    let Some(agent) =
                        PluginAgent::new(Arc::clone(plugin), part.id())
                    else {
                        continue;
                    };
                    running.parts.push((part.clone(), agent.health().clone()));
                    self.agents.push(match icon {
                        Some(icon) => agent.with_icon(icon),
                        None => agent,
                    });
                },
                PartKind::Provider => {
                    let Some(provider) =
                        PluginProvider::new(Arc::clone(plugin), part.id())
                    else {
                        continue;
                    };
                    running
                        .parts
                        .push((part.clone(), provider.health().clone()));
                    self.providers.push(match icon {
                        Some(icon) => provider.with_icon(icon),
                        None => provider,
                    });
                },
            }
        }

        self.running.push(running);
    }
}

/// The manifest in `folder`; `None`, with a warning in the log, when it
/// can not be read or is not valid.
fn read_manifest(folder: &Path) -> Option<Manifest> {
    let path = folder.join(MANIFEST_FILE);

    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) => {
            log::warn!("{} can not be read: {error}", path.display());
            return None;
        },
    };
    match Manifest::parse(&text) {
        Ok(manifest) => Some(manifest),
        Err(error) => {
            log::warn!("{} is not a valid manifest: {error}", path.display());
            None
        },
    }
}

/// What the parts of `manifests` see: the home folder `home` and each
/// variable a part lists in `env` that is set, with the value `variable`
/// gives. An empty value counts as not set.
fn plugin_env<'a>(
    home: PathBuf,
    manifests: impl Iterator<Item = &'a Manifest>,
    variable: impl Fn(&str) -> Option<String>,
) -> Environment {
    let mut env = Environment::new(home);

    for manifest in manifests {
        for part in manifest.parts() {
            let Some(permissions) = manifest.permissions_of(&part) else {
                continue;
            };
            for name in permissions.env() {
                let value = variable(name).filter(|value| !value.is_empty());
                if let Some(value) = value {
                    env = env.with_var(name, &value);
                }
            }
        }
    }

    env
}

/// The file name of the icon of `part` in `manifest`.
fn icon_of<'a>(manifest: &'a Manifest, part: &PartRef) -> Option<&'a str> {
    match part.kind() {
        PartKind::Agent => manifest
            .agents()
            .iter()
            .find(|agent| agent.id() == part.id())
            .map(|agent| agent.icon()),
        PartKind::Provider => manifest
            .providers()
            .iter()
            .find(|provider| provider.id() == part.id())
            .map(|provider| provider.icon()),
    }
}

#[cfg(test)]
mod tests {
    use otty_app_testing::temp_dir::TempDir;

    use super::*;

    /// The template plugin as OTTY carries it built in: its manifest, its
    /// `.wasm` and its icons in one folder.
    fn template_folder() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../../../assets/plugins/example")
    }

    fn load(home: &TempDir, cache: &TempDir, on: Vec<PartRef>) -> PluginParts {
        let engine =
            PluginEngine::new(cache.path().to_path_buf()).expect("the engine");
        let installed = [InstalledParts {
            folder: template_folder(),
            on,
        }];

        PluginParts::load(
            &engine,
            &installed,
            home.path().to_path_buf(),
            |_| None,
        )
    }

    /// Issue 07: the parts that are on fill the registry with the names and
    /// icons of the manifest; a part that is off adds nothing.
    #[test]
    fn the_parts_that_are_on_fill_the_registry() {
        let home = TempDir::new("plugin-parts-home");
        let cache = TempDir::new("plugin-parts-cache");

        let parts = load(&home, &cache, vec![PartRef::agent("example-agent")]);
        let readers = parts.into_readers(otty_agents::known_agents());

        let agent = readers
            .registry
            .agent(&otty_agents::AgentId::new("example-agent"))
            .expect("the agent part's entry");
        assert_eq!(agent.name(), "Example Agent");
        assert!(agent.icon().is_some());
        assert!(
            readers
                .registry
                .provider(&otty_agents::ProviderId::new("example-cloud"))
                .is_none()
        );
        assert!(readers.providers.is_empty());
        let ids: Vec<_> = readers
            .agents
            .iter()
            .map(|agent| agent.entry().id().as_str().to_string())
            .collect();
        assert_eq!(
            ids,
            ["example-agent", "claude-code", "codex", "kilo", "opencode"]
        );
    }

    #[test]
    fn a_provider_part_that_is_on_serves_its_hosts() {
        let home = TempDir::new("plugin-parts-provider-home");
        let cache = TempDir::new("plugin-parts-provider-cache");

        let parts =
            load(&home, &cache, vec![PartRef::provider("example-cloud")]);
        let readers = parts.into_readers(Vec::new());

        let provider = readers
            .registry
            .provider_of_host("api.example.com")
            .expect("the provider part's entry");
        assert_eq!(provider.name(), "Example Cloud");
        assert_eq!(readers.providers.len(), 1);
        assert!(readers.agents.is_empty());
    }

    #[test]
    fn a_folder_with_no_plugin_is_left_out() {
        let home = TempDir::new("plugin-parts-none-home");
        let cache = TempDir::new("plugin-parts-none-cache");
        let engine =
            PluginEngine::new(cache.path().to_path_buf()).expect("the engine");
        let installed = [InstalledParts {
            folder: home.path().join("no-such-plugin"),
            on: vec![PartRef::agent("example-agent")],
        }];

        let parts = PluginParts::load(
            &engine,
            &installed,
            home.path().to_path_buf(),
            |_| None,
        );

        assert!(parts.agents().is_empty());
        assert!(parts.providers().is_empty());
    }

    #[test]
    fn the_parts_see_only_the_variables_they_list() {
        let manifest = read_manifest(&template_folder()).expect("a manifest");

        let env = plugin_env(
            PathBuf::from("/home/dev"),
            std::iter::once(&manifest),
            |name| Some(format!("/data/{name}")),
        );

        let vars: Vec<_> = env.vars().collect();
        assert_eq!(vars, [("EXAMPLE_AGENT_HOME", "/data/EXAMPLE_AGENT_HOME")]);
    }
}
