use std::path::PathBuf;

use iced::widget::Id;
use otty_app_domain::plugins::{OffPart, PluginNotes, PluginsPlace};
use otty_plugin_manifest::PartRef;
use otty_plugin_releases::{
    InstallError, InstallPlan, PendingUpdate, UpdateOutcome, UpdateReport,
};

use crate::types::{ListedPlugin, PartsToLoad, RunningPlugin};

/// The installed plugins after a change, as `otty-app-plugins-io` read
/// them again, or why the change did not happen.
pub type ChangeAnswer = Result<Vec<ListedPlugin>, InstallError>;

/// What Plugins mode is asked to do: a press in its views, the answer of
/// its I/O, or news from the binary.
#[derive(Debug, Clone)]
pub enum PluginsIntent {
    /// Show `PluginsPlace` in the mode: from Settings, from "Read by" or a
    /// need of Agents mode. The binary switches to the mode first.
    Open(PluginsPlace),
    /// A press on a plugin of the left panel: its page.
    Select(String),
    /// The search field of the left panel was edited or cleared.
    SearchChanged(String),
    /// Cancel or the close cross of a dialog, or Esc: the dialog closes.
    Back,
    /// Add plugin…: the field for a GitHub link.
    AddPlugin,
    /// Add plugin from folder…: the field for a folder.
    AddFromFolder,
    /// The GitHub link was edited.
    UrlChanged(String),
    /// The folder was edited.
    FolderChanged(String),
    /// Next under the link or the folder: find the plugin there.
    Find,
    /// The plugin was found and checked, or why not.
    Found(Result<Box<InstallPlan>, InstallError>),
    /// Install of the install dialog.
    Install,
    /// The switch of a part was flipped.
    SetPartOn {
        /// The plugin id.
        plugin: String,
        /// The part.
        part: PartRef,
        /// Whether the part is to be on.
        on: bool,
    },
    /// Turn on of the need "<Part> of the <Plugin> plugin is off".
    TurnOn(OffPart),
    /// Try again of a paused part.
    TryAgain {
        /// The plugin id.
        plugin: String,
        /// The part.
        part: PartRef,
    },
    /// Copy report of the last failed call of a part: the report.
    CopyReport(String),
    /// Check for updates of a plugin.
    CheckForUpdates(String),
    /// Reload of a local plugin.
    Reload(String),
    /// Review of a plugin whose update waits: the update dialog.
    Review(String),
    /// Update of the update dialog.
    ApplyUpdate,
    /// Show log of a plugin.
    ShowLog(String),
    /// Open repository of a plugin.
    OpenRepository(String),
    /// Remove of a plugin: the dialog that asks first.
    AskRemove(String),
    /// Remove of that dialog.
    Remove,
    /// A change finished: an install, a switch, a removal or an update the
    /// user agreed to.
    Changed(ChangeAnswer),
    /// Update checks finished, the daily one or Check for updates, with
    /// the plugins as they are after them.
    Checked {
        /// What each check did.
        reports: Vec<UpdateReport>,
        /// The installed plugins after the checks.
        plugins: Vec<ListedPlugin>,
        /// Whether the user asked for the check: "Up to date" shows too.
        asked: bool,
    },
    /// Reload of a local plugin finished.
    Reloaded {
        /// The plugin id.
        plugin: String,
        /// What it did, or why not.
        outcome: Result<UpdateOutcome, InstallError>,
        /// The installed plugins after it.
        plugins: Vec<ListedPlugin>,
    },
    /// The timer of the daily update check.
    UpdateTick,
    /// The parts that run now, after the binary loaded them.
    Running(Vec<RunningPlugin>),
}

/// What Plugins mode reports to the binary.
#[derive(Debug, Clone, PartialEq)]
pub enum PluginsOutcome {
    /// Move the keyboard focus to this field, such as the GitHub link of
    /// Add plugin….
    FocusField(Id),
    /// Find the newest release of the GitHub repository at this link;
    /// answer [`PluginsIntent::Found`].
    FindRelease(String),
    /// Find the plugin in this folder; answer [`PluginsIntent::Found`].
    FindLocal(PathBuf),
    /// Install the plugin of the plan; answer [`PluginsIntent::Changed`].
    Install {
        /// What the install dialog showed.
        plan: Box<InstallPlan>,
    },
    /// Turn a part on or off; answer [`PluginsIntent::Changed`].
    SetPartOn {
        /// The plugin id.
        plugin: String,
        /// The part.
        part: PartRef,
        /// Whether it is to be on.
        on: bool,
    },
    /// Remove a plugin; answer [`PluginsIntent::Changed`].
    Remove(String),
    /// Check the releases of a plugin; answer [`PluginsIntent::Checked`].
    CheckUpdate(String),
    /// Run the daily update check when a day has passed since the last
    /// one; answer [`PluginsIntent::Checked`].
    CheckUpdatesIfDue,
    /// Read the folder of a local plugin again; answer
    /// [`PluginsIntent::Reloaded`].
    Reload(String),
    /// Install a waiting update the user agreed to; answer
    /// [`PluginsIntent::Changed`].
    ApplyUpdate {
        /// The update.
        update: Box<PendingUpdate>,
    },
    /// Open this page in the user's browser, such as a repository.
    OpenPage(String),
    /// Put this text on the clipboard, such as the report of a failed
    /// call.
    CopyText(String),
    /// The parts that are on changed: load these into Agents mode.
    LoadParts(Vec<PartsToLoad>),
    /// What Agents mode shows of the plugins changed: the parts that are
    /// off and the updates that wait.
    Notes(PluginNotes),
    /// A paused part runs again: Agents mode reads again.
    ReadAgain,
    /// Tell the user what an action did, such as "Plugin Example
    /// reloaded": a neutral notification (issue 35).
    Done(String),
    /// Tell the user why an action failed, such as "Plugin Example could
    /// not be reloaded: …": an error notification (issue 35).
    Failed(String),
}
