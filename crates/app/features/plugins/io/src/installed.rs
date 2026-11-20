//! The installed plugins as the app holds them: one installer shared by
//! the work that runs on threads of its own, where the plugins live on
//! disk, and the plugins listed for the page.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use otty_agents::{Reader, Registry};
use otty_app_plugins::ListedPlugin;
use otty_plugin_host::PluginEngine;
use otty_plugin_manifest::{Manifest, PartKind};
use otty_plugin_releases::{
    InstallError, Installer, MANIFEST_FILE, Places, Served, UreqClient,
};

/// The file of the config folder that records the installed plugins.
const PLUGINS_FILE: &str = "plugins.json";
/// The folder of the data folder that holds the installed plugins.
const PLUGINS_FOLDER: &str = "plugins";

/// The installed plugins and everything that changes them, shared by the
/// work that runs on threads of its own; one change runs at a time. When
/// `plugins.json` could not be read, every change answers why, and the
/// file is left as it is.
#[derive(Clone)]
pub struct SharedInstaller {
    installer: Arc<Mutex<Result<Installer, InstallError>>>,
}

impl SharedInstaller {
    /// The installed plugins as `places` records them. The installer
    /// reaches GitHub with OTTY's client, checks and compiles plugins with
    /// `engine`, and counts the ids and hosts the readers built into OTTY
    /// serve, `built_in_readers`, as served.
    pub fn open(
        places: Places,
        engine: Arc<PluginEngine>,
        built_in_readers: Vec<Served>,
    ) -> Self {
        let opened = Installer::open(
            places,
            Box::new(UreqClient::new()),
            Box::new(engine),
        )
        .map(|installer| installer.with_built_in_readers(built_in_readers));
        if let Err(error) = &opened {
            log::warn!("the installed plugins can not be read: {error}");
        }

        Self {
            installer: Arc::new(Mutex::new(opened)),
        }
    }

    /// The installed plugins as the page lists them; none when
    /// `plugins.json` could not be read.
    pub fn plugins(&self) -> Vec<ListedPlugin> {
        self.with(|installer| Ok(listed(installer)))
            .unwrap_or_default()
    }

    /// Runs `work` on the installer, waiting for a change that runs.
    pub(crate) fn with<T>(
        &self,
        work: impl FnOnce(&mut Installer) -> Result<T, InstallError>,
    ) -> Result<T, InstallError> {
        let mut opened = self
            .installer
            .lock()
            .unwrap_or_else(PoisonError::into_inner);

        match opened.as_mut() {
            Ok(installer) => work(installer),
            Err(error) => Err(error.clone()),
        }
    }
}

/// Where the installed plugins live: their files in
/// `<data_dir>/plugins/`, such as `~/.local/share/otty/plugins/`, and
/// `plugins.json` in the config folder `config_dir`.
pub fn places(config_dir: &Path, data_dir: &Path) -> Places {
    Places {
        plugins_folder: data_dir.join(PLUGINS_FOLDER),
        plugins_file: config_dir.join(PLUGINS_FILE),
    }
}

/// The agent ids, provider ids and hosts the readers built into OTTY
/// serve in `registry`: a plugin part that serves one of them asks the
/// user which one serves it.
pub fn built_in_served(registry: &Registry) -> Vec<Served> {
    let agents = registry
        .agents()
        .iter()
        .filter(|entry| *entry.reader() == Reader::BuiltIn)
        .map(|entry| Served::Agent(entry.id().as_str().to_string()));
    let providers = registry
        .providers()
        .iter()
        .filter(|entry| *entry.reader() == Reader::BuiltIn)
        .flat_map(|entry| {
            std::iter::once(Served::Provider(entry.id().as_str().to_string()))
                .chain(entry.hosts().iter().cloned().map(Served::Host))
        });

    agents.chain(providers).collect()
}

/// Each installed plugin of `installer` with the manifest and the icon of
/// its installed version.
pub(crate) fn listed(installer: &Installer) -> Vec<ListedPlugin> {
    installer
        .plugins()
        .iter()
        .map(|plugin| {
            let folder = installer.folder_of(plugin);
            let manifest = read_manifest(&folder);
            let icon = manifest
                .as_ref()
                .and_then(|manifest| first_icon(manifest, &folder));

            ListedPlugin {
                installed: plugin.clone(),
                manifest,
                folder,
                icon,
            }
        })
        .collect()
}

/// The manifest in `folder`; `None`, with a warning, when it can not be
/// read.
fn read_manifest(folder: &Path) -> Option<Manifest> {
    let path = folder.join(MANIFEST_FILE);

    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) => {
            log::warn!("{} can not be read: {error}", path.display());
            return None;
        },
    };
    Manifest::parse(&text)
        .inspect_err(|error| {
            log::warn!("{} is not a valid manifest: {error}", path.display());
        })
        .ok()
}

/// The SVG of the icon of the first part of `manifest`, in `folder`.
fn first_icon(manifest: &Manifest, folder: &Path) -> Option<Vec<u8>> {
    let part = manifest.parts().into_iter().next()?;
    let icon = match part.kind() {
        PartKind::Agent => manifest
            .agents()
            .iter()
            .find(|agent| agent.id() == part.id())
            .map(|agent| agent.icon().to_string()),
        PartKind::Provider => manifest
            .providers()
            .iter()
            .find(|provider| provider.id() == part.id())
            .map(|provider| provider.icon().to_string()),
    }?;

    std::fs::read(folder.join(PathBuf::from(icon))).ok()
}

#[cfg(test)]
mod tests {
    use otty_agents::known_agents;
    use otty_app_testing::temp_dir::TempDir;

    use super::*;

    #[test]
    fn the_plugins_live_in_the_data_folder_and_plugins_json_in_the_config_folder()
     {
        let places = places(
            Path::new("/home/user/.config/otty"),
            Path::new("/home/user/.local/share/otty"),
        );

        assert_eq!(
            places.plugins_folder,
            PathBuf::from("/home/user/.local/share/otty/plugins")
        );
        assert_eq!(
            places.plugins_file,
            PathBuf::from("/home/user/.config/otty/plugins.json")
        );
    }

    /// Issue 07: a part that serves what a reader built into OTTY serves
    /// is a conflict the user settles. No agent reader is built in any
    /// more.
    #[test]
    fn the_built_in_readers_serve_their_agents_providers_and_hosts() {
        let registry = Registry::built_in(&known_agents());

        let served = built_in_served(&registry);

        assert!(served.contains(&Served::Provider(String::from("moonshot"))));
        assert!(served.contains(&Served::Host(String::from("moonshot.ai"))));
        assert!(
            !served
                .iter()
                .any(|served| matches!(served, Served::Agent(_))),
            "every agent is read by a plugin"
        );
    }

    #[test]
    fn a_plugins_json_that_can_not_be_read_lists_nothing_and_refuses_changes() {
        let folder = TempDir::new("plugins-io-broken");
        let config = folder.path().join("config");
        std::fs::create_dir_all(&config).expect("the folder");
        std::fs::write(config.join(PLUGINS_FILE), "not json").expect("written");
        let engine =
            PluginEngine::new(folder.path().join("cache")).expect("the engine");

        let shared = SharedInstaller::open(
            places(&config, &folder.path().join("data")),
            Arc::new(engine),
            Vec::new(),
        );

        assert!(shared.plugins().is_empty());
        assert!(shared.with(|installer| installer.remove("x")).is_err());
        assert_eq!(
            std::fs::read_to_string(config.join(PLUGINS_FILE)).expect("kept"),
            "not json"
        );
    }
}
