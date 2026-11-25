//! What Agents mode reads with once the plugins are loaded (spec
//! 2026-11-19-agent-plugins, "How plugins talk to OTTY"): the one plugin
//! engine, the parts that are on of the installed plugins, and what the
//! agents look at in the environment. It is built at start and again on a thread of its own
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
use crate::plugin_language::LanguageReceivers;
use crate::plugin_parts::{
    FailedPlugin, InstalledParts, PluginParts, ProviderList, RunningParts,
};

/// The folder of `cache_dir` the compiled plugins are kept in.
const COMPILED_FOLDER: &str = "plugins";

/// The folder in OTTY's cache folder that an agent's program started in a
/// hidden terminal runs in, so it never shows up as a session of a real
/// project.
const PROBE_FOLDER: &str = "agent-probe";

/// How a variable of the environment is read: the binary gives
/// `std::env::var_os`, a test a made-up one.
pub type ReadVariable = fn(&str) -> Option<OsString>;

/// What Agents mode reads with: the agents and the registry, the provider
/// parts that read the numbers of the accounts, the plugins that run
/// with their logs and the health of their parts, and the plugins that did
/// not load. A plain bag of shared values.
#[derive(Clone)]
pub struct LoadedReaders {
    /// The agents, the registry and the environment of Agents mode.
    pub readers: AgentReaders,
    /// The provider parts that are on.
    pub providers: ProviderList,
    /// The plugins that run.
    pub running: Vec<RunningParts>,
    /// The plugins told when the language of OTTY's interface changes.
    pub language_receivers: LanguageReceivers,
    /// The plugins that did not load, with why.
    pub failed: Vec<FailedPlugin>,
}

impl std::fmt::Debug for LoadedReaders {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LoadedReaders")
            .field("readers", &self.readers)
            .field("providers", &self.providers.len())
            .field("running", &self.running)
            .field("language_receivers", &self.language_receivers.len())
            .field("failed", &self.failed)
            .finish()
    }
}

/// The one engine of the app's plugins: its compiled plugins are kept in
/// `cache_dir`, and it starts their programs and hidden terminals through
/// the user's login shell `shell`.
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
/// loaded by `engine`. The agents look
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
/// change. When the thread stops before it answers, no part serves.
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
/// `CODER_HOME`; `variable` gives the value of a variable. An empty value
/// counts as not set.
pub fn agent_env(
    home: PathBuf,
    agents: &[Box<dyn Agent>],
    variable: impl Fn(&str) -> Option<OsString>,
) -> AgentEnv {
    let mut env = AgentEnv::new(home);

    for agent in agents {
        let folder = agent.config_folder();
        let name = folder.variable();
        let path = variable(name).filter(|value| !value.is_empty());
        if let Some(path) = path {
            env = env.with_config_dir(name, PathBuf::from(path));
        }
    }

    env
}

/// The readers of `parts`, without the agents and providers whose ids
/// `hidden` lists.
fn readers_of(
    parts: PluginParts,
    home: PathBuf,
    variable: ReadVariable,
    hidden: &[String],
) -> LoadedReaders {
    let readers = parts.into_readers();
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
        language_receivers: readers.language_receivers,
        failed: readers.failed,
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
    use crate::testing::{example_copy, run_task};

    fn no_variable(_: &str) -> Option<OsString> {
        None
    }

    /// Issue 07, left for issue 09: the installed plugins load into the
    /// readers of Agents mode, with the health and log of their parts.
    #[test]
    fn the_installed_parts_are_the_readers() {
        let home = TempDir::new("plugin-readers-home");
        let cache = TempDir::new("plugin-readers-cache");
        let engine =
            plugin_engine(cache.path(), "/bin/sh").expect("the engine");
        let plugins = vec![InstalledParts {
            folder: example_copy(),
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
        assert_eq!(loaded.readers.agents.len(), 1);
    }

    /// `OTTY_HIDE_ENTRIES` of a debug build: the agents and providers it
    /// names leave the registry and the lists.
    #[test]
    fn the_hidden_entries_leave_the_registry_and_the_agents() {
        let cache = TempDir::new("plugin-readers-hidden-cache");
        let engine =
            plugin_engine(cache.path(), "/bin/sh").expect("the engine");

        let plugins = [InstalledParts {
            folder: example_copy(),
            on: vec![PartRef::agent("example-agent")],
        }];

        let loaded = readers_now(
            &engine,
            &plugins,
            PathBuf::from("/home/user"),
            no_variable,
            &[String::from("example-cloud"), String::from("example-agent")],
        );

        let registry = &loaded.readers.registry;
        assert!(
            registry
                .provider(&ProviderId::new("example-cloud"))
                .is_none()
        );
        assert!(registry.agent(&AgentId::new("example-agent")).is_none());
        assert!(loaded.readers.agents.is_empty());
    }

    /// Issue 17: a plugin whose manifest declares no entry of the system
    /// keyring, as the official plugins do, gives its parts none, so with
    /// only such plugins installed Agents mode never asks to allow the
    /// keyring. The example plugin stands for them.
    #[test]
    fn a_plugin_that_declares_no_keyring_entry_gives_its_parts_none() {
        let cache = TempDir::new("plugin-readers-keyring-cache");
        let engine =
            plugin_engine(cache.path(), "/bin/sh").expect("the engine");
        let plugins = [InstalledParts {
            folder: example_copy(),
            on: vec![
                PartRef::agent("example-agent"),
                PartRef::provider("example-cloud"),
            ],
        }];

        let loaded = readers_now(
            &engine,
            &plugins,
            PathBuf::from("/no/such/home"),
            no_variable,
            &[],
        );

        let registry = &loaded.readers.registry;
        assert_eq!(registry.agents().len(), 1);
        assert_eq!(registry.providers().len(), 1);
        assert!(
            registry
                .agents()
                .iter()
                .all(|entry| entry.keyring_services().is_empty())
        );
        assert!(
            registry
                .providers()
                .iter()
                .all(|entry| entry.keyring_services().is_empty())
        );
    }

    #[test]
    fn the_agents_look_at_the_home_folder_and_their_variables() {
        let agents = vec![otty_agents::fixtures::codex_stand_in()];
        let set = |name: &str| {
            (name == "CODEX_HOME").then(|| OsString::from("/data/codex"))
        };
        let empty = |name: &str| (name == "CODEX_HOME").then(OsString::new);

        let env = agent_env(PathBuf::from("/home/user"), &agents, set);
        let empty_env = agent_env(PathBuf::from("/home/user"), &agents, empty);

        assert_eq!(env.home(), Path::new("/home/user"));
        assert_eq!(
            env.config_dir_of("CODEX_HOME"),
            Some(Path::new("/data/codex"))
        );
        assert_eq!(
            empty_env.config_dir_of("CODEX_HOME"),
            None,
            "an empty value is not set"
        );
    }
}
