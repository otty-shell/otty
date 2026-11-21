//! The language of OTTY's interface as the plugins see it (spec
//! 2026-11-19-agent-plugins, issue 20). The engine gives each plugin the
//! language at every call; when the user picks another, the plugins that
//! export the language receiver are told, and OTTY reads Agents mode again
//! when one of them answers that its texts depend on the language. A
//! plugin without the receiver is not called.

use std::sync::Arc;

use iced::Task;
use otty_plugin_host::{LoadedPlugin, PluginEngine};

use crate::background::wait_for_thread;

/// The loaded plugins whose parts are on and that export the language
/// receiver, shared by the calls that tell them a new language.
pub type LanguageReceivers = Arc<[Arc<LoadedPlugin>]>;

/// The language `tag`, such as `ru`, is the one of OTTY's interface from
/// now on: the engine gives it to every plugin at its next call, and each
/// plugin of `receivers` is told, on a thread of its own. The answer is
/// whether one of them said its texts depend on the language, so Agents
/// mode reads its parts again; a thread that stops before it answers
/// gives `false`.
pub fn tell_language(
    engine: Arc<PluginEngine>,
    receivers: LanguageReceivers,
    tag: String,
) -> Task<bool> {
    let work = move || told_language(&engine, &receivers, &tag);

    Task::perform(wait_for_thread(work), |changed| changed.unwrap_or(false))
}

/// [`tell_language`], blocking: every receiver is told, even after one
/// answered true. A receiver that fails counts as false, with a warning
/// in the log.
fn told_language(
    engine: &PluginEngine,
    receivers: &[Arc<LoadedPlugin>],
    tag: &str,
) -> bool {
    engine.set_language(tag);

    let mut changed = false;
    for plugin in receivers {
        match plugin.language_changed(tag) {
            Ok(depends) => changed |= depends,
            Err(error) => {
                let id = plugin.manifest().plugin().id();
                log::warn!(
                    "the plugin {id} did not take the language: {error}"
                );
            },
        }
    }

    changed
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use otty_agents::{AgentId, Profile, ProfileStatus};
    use otty_app_testing::temp_dir::TempDir;
    use otty_plugin_host::PartRef;

    use super::*;
    use crate::plugin_parts::{InstalledParts, PluginParts};

    /// The template plugin as OTTY carries it built in.
    fn template_folder() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../../../assets/plugins/example")
    }

    /// A plugin whose texts are only in English: the smallest test plugin
    /// of the host, which exports no language receiver.
    fn english_only_folder(home: &TempDir) -> PathBuf {
        let folder = home.path().join("english-only");
        let wasm = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../../plugins/host/tests/fixtures/minimal.wasm");
        let manifest = "[plugin]\nid = \"english-only\"\nname = \"English \
                        only\"\nversion = \"1.0.0\"\nwasm = \
                        \"plugin.wasm\"\n\n[api]\nagent = \"0.1\"\n\n\
                        [[agent]]\nid = \"test-agent\"\nname = \"Test \
                        agent\"\nicon = \"test.svg\"\n";

        std::fs::create_dir_all(&folder).expect("the folder");
        std::fs::copy(wasm, folder.join("plugin.wasm")).expect("the wasm");
        std::fs::write(folder.join("otty-plugin.toml"), manifest)
            .expect("the manifest");
        folder
    }

    /// A profile of the template's agent nobody is logged in with.
    fn nobody(home: &TempDir) -> Profile {
        let folder = home.path().join(".example-agent-nobody");
        std::fs::create_dir_all(&folder).expect("the folder");
        std::fs::write(folder.join("state.json"), "{}").expect("a file");

        Profile::found(AgentId::new("example-agent"), "Example", folder)
    }

    /// Issue 20: a new language reaches the plugin that exports the
    /// receiver, which asks for a read again, and its needs read again
    /// come in the new language, without loading it again.
    #[test]
    fn a_new_language_reaches_the_receiver_and_its_needs_follow() {
        let home = TempDir::new("plugin-language-home");
        let cache = TempDir::new("plugin-language-cache");
        let engine = Arc::new(
            PluginEngine::new(cache.path().to_path_buf()).expect("the engine"),
        );
        let installed = [InstalledParts {
            folder: template_folder(),
            on: vec![PartRef::agent("example-agent")],
        }];
        let parts = PluginParts::load(
            &engine,
            &installed,
            home.path().to_path_buf(),
            |_| None,
        );
        let readers = parts.into_readers();
        let profile = nobody(&home);
        let title = |agents: &[Box<dyn otty_agents::Agent>]| {
            let ProfileStatus::NotLoggedIn(Some(need)) =
                agents[0].detect(&profile)
            else {
                panic!("nobody is logged in");
            };
            need.title().to_string()
        };

        let before = title(&readers.agents);
        let changed = told_language(&engine, &readers.language_receivers, "ru");
        let after = title(&readers.agents);

        assert_eq!(readers.language_receivers.len(), 1);
        assert!(changed);
        assert_eq!(before, "Needs login");
        assert_eq!(after, "Нужен вход");
    }

    /// Issue 20: a plugin that does not export the receiver is not told;
    /// nothing asks for a read again.
    #[test]
    fn a_plugin_without_the_receiver_is_not_told() {
        let home = TempDir::new("plugin-language-english-home");
        let cache = TempDir::new("plugin-language-english-cache");
        let engine = Arc::new(
            PluginEngine::new(cache.path().to_path_buf()).expect("the engine"),
        );
        let installed = [InstalledParts {
            folder: english_only_folder(&home),
            on: vec![PartRef::agent("test-agent")],
        }];
        let parts = PluginParts::load(
            &engine,
            &installed,
            home.path().to_path_buf(),
            |_| None,
        );
        let readers = parts.into_readers();

        let changed = told_language(&engine, &readers.language_receivers, "ru");

        assert_eq!(readers.agents.len(), 1);
        assert!(readers.language_receivers.is_empty());
        assert!(!changed);
    }
}
