use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use otty_plugin_manifest::PartRef;
use otty_plugin_releases::{InstallPlan, PendingUpdate};

use crate::types::{ListedPlugin, RunningPlugin};

/// A dialog of Plugins mode, over the window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Dialog {
    /// Add plugin…: the GitHub link.
    AddUrl {
        /// The link typed so far.
        url: String,
    },
    /// Add plugin from folder…: the folder.
    AddFolder {
        /// The folder typed so far.
        folder: String,
    },
    /// The install dialog of a plugin found.
    Install {
        /// What was found.
        plan: Box<InstallPlan>,
    },
    /// The update dialog of the plugin with this id.
    Update {
        /// The plugin id.
        plugin: String,
    },
    /// What one part of an installed plugin can access.
    CanAccess {
        /// The plugin id.
        plugin: String,
        /// The part.
        part: PartRef,
    },
    /// The log of the plugin with this id.
    Log(String),
    /// The dialog that asks before the plugin with this id is removed.
    Remove(String),
}

/// What the latest update check or reload left to show about one plugin.
/// What it did, such as "Plugin Example updated to 0.5.0", is a
/// notification instead (issue 35).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PluginNews {
    /// A newer version waits for the user's OK: Review on its page.
    Waiting(Box<PendingUpdate>),
    /// Why the check or the reload failed, in plain words: its row in the
    /// left panel is signed failed.
    Failed(String),
}

/// The state of Plugins mode: the installed plugins, the parts that run,
/// what the checks said, the selected plugin, the search and the open
/// dialog.
pub(crate) struct PluginsState {
    dialog: Option<Dialog>,
    plugins: Vec<ListedPlugin>,
    running: Vec<RunningPlugin>,
    news: BTreeMap<String, PluginNews>,
    too_new: BTreeMap<String, String>,
    selected: Option<String>,
    search: String,
    error: Option<String>,
    is_busy: bool,
    home: PathBuf,
}

impl PluginsState {
    /// The open dialog, if any.
    pub(crate) fn dialog(&self) -> Option<&Dialog> {
        self.dialog.as_ref()
    }

    /// The installed plugins.
    pub(crate) fn plugins(&self) -> &[ListedPlugin] {
        &self.plugins
    }

    /// The installed plugin `id`.
    pub(crate) fn plugin(&self, id: &str) -> Option<&ListedPlugin> {
        self.plugins.iter().find(|plugin| plugin.id() == id)
    }

    /// The running plugin `id`.
    pub(crate) fn running_plugin(&self, id: &str) -> Option<&RunningPlugin> {
        self.running.iter().find(|running| running.plugin == id)
    }

    /// What the checks said about the plugin `id`.
    pub(crate) fn news_of(&self, id: &str) -> Option<&PluginNews> {
        self.news.get(id)
    }

    /// The newer version of the plugin `id` that needs a newer OTTY.
    pub(crate) fn too_new_of(&self, id: &str) -> Option<&str> {
        self.too_new.get(id).map(String::as_str)
    }

    /// The waiting update of the plugin `id`.
    pub(crate) fn waiting_update(&self, id: &str) -> Option<&PendingUpdate> {
        match self.news.get(id)? {
            PluginNews::Waiting(update) => Some(update),
            _ => None,
        }
    }

    /// The plugin whose page the main area shows; never one that is not
    /// installed.
    pub(crate) fn selected(&self) -> Option<&str> {
        self.selected.as_deref()
    }

    /// The text typed in the search field of the left panel.
    pub(crate) fn search(&self) -> &str {
        &self.search
    }

    /// Why the latest action failed: in the open dialog, or on the page
    /// of the selected plugin.
    pub(crate) fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Whether the mode waits for its I/O: its buttons do nothing.
    pub(crate) fn is_busy(&self) -> bool {
        self.is_busy
    }

    /// The user's home folder, which `~` in a typed folder stands for.
    pub(crate) fn home(&self) -> &Path {
        &self.home
    }

    /// No dialog, nothing selected, with the installed plugins `plugins`.
    pub(crate) fn new(plugins: Vec<ListedPlugin>) -> Self {
        Self {
            dialog: None,
            plugins,
            running: Vec::new(),
            news: BTreeMap::new(),
            too_new: BTreeMap::new(),
            selected: None,
            search: String::new(),
            error: None,
            is_busy: false,
            home: PathBuf::from("/"),
        }
    }

    /// The same state, with `~` in a typed folder standing for `home`.
    pub(crate) fn with_home(mut self, home: PathBuf) -> Self {
        self.home = home;
        self
    }

    /// Shows `dialog`, with no error and nothing waited for.
    pub(crate) fn show(&mut self, dialog: Dialog) {
        self.dialog = Some(dialog);
        self.error = None;
        self.is_busy = false;
    }

    /// Closes the open dialog, with no error and nothing waited for.
    pub(crate) fn close_dialog(&mut self) {
        self.dialog = None;
        self.error = None;
        self.is_busy = false;
    }

    pub(crate) fn dialog_mut(&mut self) -> Option<&mut Dialog> {
        self.dialog.as_mut()
    }

    /// Keeps `plugins`; the selection goes when its plugin is gone.
    pub(crate) fn set_plugins(&mut self, plugins: Vec<ListedPlugin>) {
        self.plugins = plugins;

        let selected = self.selected.take();
        self.select(selected);
    }

    pub(crate) fn set_running(&mut self, running: Vec<RunningPlugin>) {
        self.running = running;
    }

    pub(crate) fn set_news(&mut self, plugin: &str, news: PluginNews) {
        self.news.insert(plugin.to_string(), news);
    }

    pub(crate) fn forget_news(&mut self, plugin: &str) {
        self.news.remove(plugin);
    }

    pub(crate) fn set_too_new(
        &mut self,
        plugin: &str,
        version: Option<String>,
    ) {
        match version {
            Some(version) => self.too_new.insert(plugin.to_string(), version),
            None => self.too_new.remove(plugin),
        };
    }

    /// Selects the plugin `plugin`, or nothing when it is not installed.
    /// A new selection drops the error of the page shown before.
    pub(crate) fn select(&mut self, plugin: Option<String>) {
        let plugin = plugin.filter(|id| self.plugin(id).is_some());

        if plugin != self.selected && self.dialog.is_none() {
            self.error = None;
        }
        self.selected = plugin;
    }

    pub(crate) fn set_search(&mut self, search: String) {
        self.search = search;
    }

    pub(crate) fn set_error(&mut self, error: Option<String>) {
        self.error = error;
    }

    pub(crate) fn set_busy(&mut self, is_busy: bool) {
        self.is_busy = is_busy;
    }
}
