//! What the Plugins page is given: the installed plugins as
//! `otty-app-plugins-io` reads them, the parts of them that run, and what
//! the page asks the binary to load.

use std::path::PathBuf;

use otty_agents::PartHealth;
use otty_plugin_host::PluginLog;
use otty_plugin_manifest::{Manifest, PartKind, PartRef};
use otty_plugin_releases::{InstalledPlugin, Source};

/// One installed plugin as the page lists it: its record in
/// `plugins.json`, the manifest of its installed version, the folder of
/// that version and the icon of its first part. A plain bag the io crate
/// fills.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListedPlugin {
    /// The record in `plugins.json`.
    pub installed: InstalledPlugin,
    /// The manifest of the installed version; `None` when its folder can
    /// not be read.
    pub manifest: Option<Manifest>,
    /// The folder of the installed version.
    pub folder: PathBuf,
    /// The SVG of the icon of its first part, when it can be read.
    pub icon: Option<Vec<u8>>,
}

impl ListedPlugin {
    /// The plugin id.
    pub fn id(&self) -> &str {
        self.installed.id()
    }

    /// The name of the part `part` from the manifest, such as "Codex";
    /// its id when the manifest does not name it.
    pub fn part_name(&self, part: &PartRef) -> String {
        part_name(self.manifest.as_ref(), part)
    }

    /// The repository of the plugin: as its manifest names it, or the one
    /// it was installed from.
    pub fn repository(&self) -> Option<&str> {
        let named = self
            .manifest
            .as_ref()
            .and_then(|manifest| manifest.plugin().repository());

        match (named, self.installed.source()) {
            (Some(url), _) => Some(url),
            (None, Source::Release { url }) => Some(url),
            _ => None,
        }
    }

    /// The parts that are on, to load from the folder.
    pub(crate) fn parts_to_load(&self) -> PartsToLoad {
        let on = self
            .installed
            .parts()
            .iter()
            .filter(|part| part.is_on())
            .map(|part| part.part())
            .collect();

        PartsToLoad {
            folder: self.folder.clone(),
            on,
        }
    }
}

/// The parts of one plugin that are on, in the folder of its installed
/// version: what the binary loads into Agents mode. A plain bag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartsToLoad {
    /// The folder of the installed version.
    pub folder: PathBuf,
    /// The parts that are on.
    pub on: Vec<PartRef>,
}

/// One plugin as it runs: its log and the health of each of its parts
/// that is loaded. A plain bag of shared handles: the page reads them when
/// it draws, and Try again starts a paused part again.
#[derive(Debug, Clone)]
pub struct RunningPlugin {
    /// The plugin id.
    pub plugin: String,
    /// What the plugin wrote to its log.
    pub log: PluginLog,
    /// Each loaded part with its health.
    pub parts: Vec<(PartRef, PartHealth)>,
}

/// The name of `part` in `manifest`, or its id.
pub(crate) fn part_name(manifest: Option<&Manifest>, part: &PartRef) -> String {
    let name = manifest.and_then(|manifest| match part.kind() {
        PartKind::Agent => manifest
            .agents()
            .iter()
            .find(|agent| agent.id() == part.id())
            .map(|agent| agent.name().to_string()),
        PartKind::Provider => manifest
            .providers()
            .iter()
            .find(|provider| provider.id() == part.id())
            .map(|provider| provider.name().to_string()),
    });

    name.unwrap_or_else(|| part.id().to_string())
}
