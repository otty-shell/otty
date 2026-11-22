use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use otty_plugin_releases::{InstallPlan, PendingUpdate};

use crate::types::{ListedPlugin, RunningPlugin};

/// What the page shows in its dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Screen {
    /// The installed plugins.
    List,
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
    /// The permissions of the plugin with this id.
    Permissions(String),
    /// The log of the plugin with this id.
    Log(String),
    /// The dialog that asks before the plugin with this id is removed.
    Remove(String),
}

/// What the latest update check, reload or change said about one plugin,
/// on its row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PluginNews {
    /// Check for updates found nothing newer: "Up to date".
    UpToDate,
    /// "Updated to 0.5.0".
    Updated(String),
    /// A newer version waits for the user's OK.
    Waiting(Box<PendingUpdate>),
    /// Why the check or the change failed, in plain words.
    Failed(String),
}

/// The state of the Plugins page: the installed plugins, the parts that
/// run, what the checks said, and the open dialog.
pub(crate) struct PluginsState {
    is_open: bool,
    screen: Screen,
    plugins: Vec<ListedPlugin>,
    running: Vec<RunningPlugin>,
    news: BTreeMap<String, PluginNews>,
    too_new: BTreeMap<String, String>,
    actions_open: Option<String>,
    highlighted: Option<String>,
    error: Option<String>,
    is_busy: bool,
    home: PathBuf,
}

impl PluginsState {
    /// Whether the page is open.
    pub(crate) fn is_open(&self) -> bool {
        self.is_open
    }

    /// The open dialog of the page.
    pub(crate) fn screen(&self) -> &Screen {
        &self.screen
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

    /// The plugin whose actions ("⋯") are shown.
    pub(crate) fn actions_open(&self) -> Option<&str> {
        self.actions_open.as_deref()
    }

    /// The plugin "Read by" opened the page at.
    pub(crate) fn highlighted(&self) -> Option<&str> {
        self.highlighted.as_deref()
    }

    /// Why the latest action of the open dialog failed.
    pub(crate) fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Whether the page waits for its I/O: its buttons do nothing.
    pub(crate) fn is_busy(&self) -> bool {
        self.is_busy
    }

    /// The user's home folder, which `~` in a typed folder stands for.
    pub(crate) fn home(&self) -> &Path {
        &self.home
    }

    /// The page closed, with the installed plugins `plugins`.
    pub(crate) fn new(plugins: Vec<ListedPlugin>) -> Self {
        Self {
            is_open: false,
            screen: Screen::List,
            plugins,
            running: Vec::new(),
            news: BTreeMap::new(),
            too_new: BTreeMap::new(),
            actions_open: None,
            highlighted: None,
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

    pub(crate) fn set_open(&mut self, is_open: bool) {
        self.is_open = is_open;
    }

    /// Shows `screen`, with no error and nothing waited for.
    pub(crate) fn show(&mut self, screen: Screen) {
        self.screen = screen;
        self.error = None;
        self.is_busy = false;
        self.actions_open = None;
    }

    pub(crate) fn screen_mut(&mut self) -> &mut Screen {
        &mut self.screen
    }

    pub(crate) fn set_plugins(&mut self, plugins: Vec<ListedPlugin>) {
        self.plugins = plugins;
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

    pub(crate) fn set_actions_open(&mut self, plugin: Option<String>) {
        self.actions_open = plugin;
    }

    pub(crate) fn set_highlighted(&mut self, plugin: Option<String>) {
        self.highlighted = plugin;
    }

    pub(crate) fn set_error(&mut self, error: Option<String>) {
        self.error = error;
    }

    pub(crate) fn set_busy(&mut self, is_busy: bool) {
        self.is_busy = is_busy;
    }
}
