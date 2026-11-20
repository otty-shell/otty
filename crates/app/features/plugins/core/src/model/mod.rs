//! What the page shows, built from its state: the list of the installed
//! plugins or the dialog open over it.

mod choices;
mod dialogs;
mod list;

use std::time::SystemTime;

pub(crate) use choices::{ChoiceViewModel, given_to_new};
pub(crate) use dialogs::{
    InstallDialogViewModel, PartPermissionsViewModel, PermissionsViewModel,
    UpdateDialogViewModel,
};
#[cfg(test)]
pub(crate) use list::LastFailureViewModel;
pub(crate) use list::{PartRowViewModel, PluginRowViewModel, failed};
use otty_app_i18n::{self as i18n, Key};

use crate::state::{PluginsState, Screen};

/// What the open page shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PageViewModel<'a> {
    /// The installed plugins.
    List {
        /// One row per plugin.
        rows: Vec<PluginRowViewModel>,
        /// Why the latest switch or removal failed.
        error: Option<&'a str>,
        /// Whether a change runs.
        is_busy: bool,
    },
    /// Add plugin…: the GitHub link.
    AddUrl {
        /// The link typed so far.
        url: &'a str,
        /// Why no plugin was found there.
        error: Option<&'a str>,
        /// Whether the plugin is looked for.
        is_busy: bool,
    },
    /// Add plugin from folder…: the folder.
    AddFolder {
        /// The folder typed so far.
        folder: &'a str,
        /// Why no plugin was found there.
        error: Option<&'a str>,
        /// Whether the folder is read.
        is_busy: bool,
    },
    /// The install dialog.
    Install(InstallDialogViewModel),
    /// The update dialog.
    Update(UpdateDialogViewModel),
    /// The permissions of a plugin.
    Permissions(PermissionsViewModel),
    /// The log of a plugin.
    Log {
        /// "Log of Example".
        title: String,
        /// What the plugin wrote, or that it wrote nothing.
        text: String,
    },
    /// The dialog that asks before a plugin is removed.
    Remove {
        /// "Remove Example?".
        title: String,
        /// Why the removal failed.
        error: Option<&'a str>,
        /// Whether the removal runs.
        is_busy: bool,
    },
}

impl<'a> PageViewModel<'a> {
    /// What the page shows; `None` while it is closed, or when the plugin
    /// of its dialog is gone, as after a removal elsewhere.
    pub(crate) fn from_state(state: &'a PluginsState) -> Option<Self> {
        if !state.is_open() {
            return None;
        }
        let error = state.error();
        let is_busy = state.is_busy();

        let page = match state.screen() {
            Screen::List => Self::List {
                rows: list::rows(state, SystemTime::now()),
                error,
                is_busy,
            },
            Screen::AddUrl { url } => Self::AddUrl {
                url,
                error,
                is_busy,
            },
            Screen::AddFolder { folder } => Self::AddFolder {
                folder,
                error,
                is_busy,
            },
            Screen::Install { plan, use_new } => {
                Self::Install(dialogs::install_dialog(state, plan, use_new))
            },
            Screen::Update { plugin, use_new } => {
                let listed = state.plugin(plugin)?;
                let update = state.waiting_update(plugin)?;
                Self::Update(dialogs::update_dialog(
                    state, listed, update, use_new,
                ))
            },
            Screen::Permissions(plugin) => {
                Self::Permissions(dialogs::permissions(state.plugin(plugin)?))
            },
            Screen::Log(plugin) => {
                let name = state.plugin(plugin)?.installed.name();
                let text = state
                    .running_plugin(plugin)
                    .map(|running| running.log.text())
                    .filter(|text| !text.is_empty())
                    .unwrap_or_else(|| {
                        i18n::t(Key::PluginsLogEmpty).to_string()
                    });
                Self::Log {
                    title: i18n::filled(
                        Key::TplPluginsLogTitle,
                        &[("name", name)],
                    ),
                    text,
                }
            },
            Screen::Remove(plugin) => Self::Remove {
                title: i18n::filled(
                    Key::TplPluginsRemoveTitle,
                    &[("name", state.plugin(plugin)?.installed.name())],
                ),
                error,
                is_busy,
            },
        };

        Some(page)
    }
}
