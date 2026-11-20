//! What Agents mode reads with once the plugins are loaded (spec
//! 2026-11-19-agent-plugins, "How plugins talk to OTTY"): the one plugin
//! engine, the parts that are on of the installed plugins joined with
//! the readers built into OTTY, and what the agents look at in the
//! environment. It is built at start and again on a thread of its own
//! each time the installed plugins change.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use iced::Task;
use otty_agents::{Agent, AgentEnv, AgentId, ProviderId, Registry};
use otty_app_agents::AgentReaders;
use otty_plugin_host::{EngineError, PluginEngine};

use crate::agent_program::LoginShellPrograms;
use crate::background::wait_for_thread;
use crate::hidden_terminal::LoginShellTerminals;
use crate::limits::PROBE_FOLDER;
use crate::plugin_parts::{
    InstalledParts, PluginParts, ProviderList, RunningParts,
};

/// The folder of `cache_dir` the compiled plugins are kept in.
const COMPILED_FOLDER: &str = "plugins";

/// How a variable of the environment is read: the binary gives
/// `std::env::var_os`, a test a made-up one.
pub type ReadVariable = fn(&str) -> Option<OsString>;

/// What Agents mode reads with: the agents and the registry, the provider
/// parts that read the numbers of the accounts, and the plugins that run
/// with their logs and the health of their parts. A plain bag of shared
/// values.
#[derive(Clone)]
pub struct LoadedReaders {
    /// The agents, the registry and the environment of Agents mode.
    pub readers: AgentReaders,
    /// The provider parts that are on.
    pub providers: ProviderList,
    /// The plugins that run.
    pub running: Vec<RunningParts>,
}

impl std::fmt::Debug for LoadedReaders {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LoadedReaders")
            .field("readers", &self.readers)
            .field("providers", &self.providers.len())
            .field("running", &self.running)
            .finish()
    }
}

/// The one engine of the app's plugins: its compiled plugins are kept in
/// `cache_dir`, and it starts their programs and hidden terminals through
/// the user's login shell `shell`, as the readers built into OTTY do.
pub fn plugin_engine(
    cache_dir: &Path,
    shell: &str,
) -> Result<PluginEngine, EngineError> {
    let engine = PluginEngine::new(cache_dir.join(COMPILED_FOLDER))?;
    let probe_dir = cache_dir.join(PROBE_FOLDER);

    Ok(engine
        .with_programs(Arc::new(LoginShellPrograms::new(shell)))
        .with_terminals(Arc::new(LoginShellTerminals::new(shell, &probe_dir))))
}

/// The readers of Agents mode now: the parts that are on of `plugins`,
/// loaded by `engine`, then the readers built into OTTY. The agents look
/// at `home` and the variables `variable` gives. The agents and providers
/// whose ids `hidden` lists are left out, as the owner asks with
/// `OTTY_HIDE_ENTRIES` in a debug build to see one no plugin serves. It
/// may compile a plugin, so it is for the start; [`load_readers`] reads on
/// a thread of its own.
pub fn readers_now(
    engine: &PluginEngine,
    plugins: &[InstalledParts],
    home: PathBuf,
    variable: ReadVariable,
    hidden: &[String],
) -> LoadedReaders {
    let parts = PluginParts::load(engine, plugins, home.clone(), |name| {
        variable(name).and_then(|value| value.into_string().ok())
    });

    readers_of(parts, home, variable, hidden)
}

/// [`readers_now`] on a thread of its own, as when the installed plugins
/// change. When the thread stops before it answers, only the readers
/// built into OTTY serve.
pub fn load_readers(
    engine: Arc<PluginEngine>,
    plugins: Vec<InstalledParts>,
    home: PathBuf,
    variable: ReadVariable,
    hidden: Vec<String>,
) -> Task<LoadedReaders> {
    let fallback_home = home.clone();
    let fallback_hidden = hidden.clone();
    let work = move || readers_now(&engine, &plugins, home, variable, &hidden);

    Task::perform(wait_for_thread(work), move |loaded| {
        loaded.unwrap_or_else(|| {
            log::warn!("loading the plugins stopped before it answered");
            readers_of(
                PluginParts::default(),
                fallback_home,
                variable,
                &fallback_hidden,
            )
        })
    })
}

/// What the agents look at in the environment: `home` and, for each of
/// `agents`, the folder its config folder variable names, such as
/// `CODEX_HOME`, and the file its database variable names, such as
/// `KILO_DB`; `variable` gives the value of a variable. An empty value
/// counts as not set.
pub fn agent_env(
    home: PathBuf,
    agents: &[Box<dyn Agent>],
    variable: impl Fn(&str) -> Option<OsString>,
) -> AgentEnv {
    let mut env = AgentEnv::new(home);

    for agent in agents {
        let folder = agent.config_folder();
        let names = std::iter::once(folder.variable())
            .chain(folder.database_variable());
        for name in names {
            let path = variable(name).filter(|value| !value.is_empty());
            if let Some(path) = path {
                env = env.with_config_dir(name, PathBuf::from(path));
            }
        }
    }

    env
}

/// `parts` joined with the readers built into OTTY, without the agents
/// and providers whose ids `hidden` lists.
fn readers_of(
    parts: PluginParts,
    home: PathBuf,
    variable: ReadVariable,
    hidden: &[String],
) -> LoadedReaders {
    let readers = parts.into_readers(otty_agents::known_agents());
    let registry = without_entries(readers.registry, hidden);
    let agents = registry.listed_agents(readers.agents);
    let env = agent_env(home, &agents, variable);

    LoadedReaders {
        readers: AgentReaders {
            agents: Arc::from(agents),
            registry: Arc::new(registry),
            env,
        },
        providers: Arc::from(readers.providers),
        running: readers.running,
    }
}

/// `registry` without the agents and the providers whose ids `hidden`
/// lists.
fn without_entries(registry: Registry, hidden: &[String]) -> Registry {
    hidden.iter().fold(registry, |registry, id| {
        registry
            .without_agent(&AgentId::new(id))
            .without_provider(&ProviderId::new(id))
    })
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use otty_app_testing::temp_dir::TempDir;
    use otty_plugin_host::PartRef;

    use super::*;
    use crate::testing::run_task;

    fn no_variable(_: &str) -> Option<OsString> {
        None
    }

    fn template_folder() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../../../assets/plugins/example")
    }

    /// Issue 07, left for issue 09: the installed plugins load into the
    /// readers of Agents mode, with the health and log of their parts.
    #[test]
    fn the_installed_parts_join_the_built_in_readers() {
        let home = TempDir::new("plugin-readers-home");
        let cache = TempDir::new("plugin-readers-cache");
        let engine =
            plugin_engine(cache.path(), "/bin/sh").expect("the engine");
        let plugins = vec![InstalledParts {
            folder: template_folder(),
            on: vec![
                PartRef::agent("example-agent"),
                PartRef::provider("example-cloud"),
            ],
        }];

        let loaded = run_task(load_readers(
            Arc::new(engine),
            plugins,
            home.path().to_path_buf(),
            no_variable,
            Vec::new(),
        ));

        let registry = &loaded.readers.registry;
        assert!(
            registry
                .agent(&otty_agents::AgentId::new("example-agent"))
                .is_some()
        );
        assert_eq!(loaded.providers.len(), 1);
        assert_eq!(loaded.running.len(), 1);
        assert_eq!(loaded.running[0].plugin, "example");
        assert_eq!(loaded.running[0].parts.len(), 2);
        assert!(loaded.readers.agents.len() > 1);
    }

    /// `OTTY_HIDE_ENTRIES` of a debug build: the agents and providers it
    /// names leave the registry and the lists.
    #[test]
    fn the_hidden_entries_leave_the_registry_and_the_agents() {
        let cache = TempDir::new("plugin-readers-hidden-cache");
        let engine =
            plugin_engine(cache.path(), "/bin/sh").expect("the engine");

        let loaded = readers_now(
            &engine,
            &[],
            PathBuf::from("/home/user"),
            no_variable,
            &[String::from("deepseek"), String::from("kilo")],
        );

        let registry = &loaded.readers.registry;
        assert!(registry.provider(&ProviderId::new("deepseek")).is_none());
        assert!(registry.agent(&AgentId::new("kilo")).is_none());
        assert!(
            loaded.readers.agents.iter().all(|agent| agent
                .entry()
                .id()
                .as_str()
                != "kilo")
        );
    }

    #[test]
    fn the_agents_look_at_the_home_folder_and_their_variables() {
        let mut agents = vec![otty_agents::fixtures::codex_stand_in()];
        agents.extend(otty_agents::known_agents());
        let variable = |name: &str| match name {
            "CODEX_HOME" => Some(OsString::from("/data/codex")),
            "XDG_DATA_HOME" => Some(OsString::new()),
            _ => None,
        };

        let env = agent_env(PathBuf::from("/home/user"), &agents, variable);

        assert_eq!(env.home(), Path::new("/home/user"));
        assert_eq!(
            env.config_dir_of("CODEX_HOME"),
            Some(Path::new("/data/codex"))
        );
        assert_eq!(
            env.config_dir_of("XDG_DATA_HOME"),
            None,
            "an empty value is not set"
        );
    }

    #[test]
    fn the_agents_with_a_database_look_at_its_variable_too() {
        let agents = otty_agents::known_agents();
        let variable = |name: &str| match name {
            "XDG_DATA_HOME" => Some(OsString::from("/data")),
            "KILO_DB" => Some(OsString::from("/data/kilo-work.db")),
            _ => None,
        };

        let env = agent_env(PathBuf::from("/home/user"), &agents, variable);

        assert_eq!(
            env.config_dir_of("XDG_DATA_HOME"),
            Some(Path::new("/data"))
        );
        assert_eq!(
            env.config_dir_of("KILO_DB"),
            Some(Path::new("/data/kilo-work.db"))
        );
        assert_eq!(env.config_dir_of("OPENCODE_DB"), None);
    }
}
