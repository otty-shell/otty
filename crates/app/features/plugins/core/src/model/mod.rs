//! What Plugins mode shows, built from its state: the list of the left
//! panel and the menu of a plugin in it, the page of the selected plugin
//! in the main area, and the dialog open over the window.

mod details;
mod dialogs;
pub(crate) mod permissions;
mod plugin_menu;
mod side_panel;

#[cfg(test)]
pub(crate) use details::LastFailureViewModel;
#[cfg(test)]
pub(crate) use details::TrustTagViewModel;
pub(crate) use details::{
    MainViewModel, PartRowViewModel, PluginPageViewModel, SourceViewModel,
    failed,
};
pub(crate) use dialogs::{
    CanAccessDialogViewModel, InstallDialogViewModel, PartPermissionsViewModel,
    UpdateDialogViewModel,
};
use otty_app_i18n::{self as i18n, Key};
pub(crate) use plugin_menu::PluginMenuViewModel;
#[cfg(test)]
pub(crate) use side_panel::PluginSign;
pub(crate) use side_panel::{PluginListRowViewModel, SidePanelViewModel};

use crate::state::{Dialog, PluginsState};

/// What the open dialog shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DialogViewModel<'a> {
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
    /// What one part can access.
    CanAccess(CanAccessDialogViewModel),
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
        /// "OTTY's own logins of this plugin are removed too: 2", only
        /// when it made any.
        own_logins: Option<String>,
        /// Why the removal failed.
        error: Option<&'a str>,
        /// Whether the removal runs.
        is_busy: bool,
    },
}

impl<'a> DialogViewModel<'a> {
    /// What the open dialog shows; `None` while none is open, or when the
    /// plugin of the dialog is gone, as after a removal elsewhere.
    pub(crate) fn from_state(state: &'a PluginsState) -> Option<Self> {
        let error = state.error();
        let is_busy = state.is_busy();

        let dialog = match state.dialog()? {
            Dialog::AddUrl { url } => Self::AddUrl {
                url,
                error,
                is_busy,
            },
            Dialog::AddFolder { folder } => Self::AddFolder {
                folder,
                error,
                is_busy,
            },
            Dialog::Install { plan } => {
                Self::Install(dialogs::install_dialog(state, plan))
            },
            Dialog::Update { plugin } => {
                let listed = state.plugin(plugin)?;
                let update = state.waiting_update(plugin)?;
                Self::Update(dialogs::update_dialog(state, listed, update))
            },
            Dialog::CanAccess { plugin, part } => Self::CanAccess(
                dialogs::can_access_dialog(state.plugin(plugin)?, part)?,
            ),
            Dialog::Log(plugin) => {
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
            Dialog::Remove { plugin, own_logins } => Self::Remove {
                title: i18n::filled(
                    Key::TplPluginsRemoveTitle,
                    &[("name", state.plugin(plugin)?.installed.name())],
                ),
                own_logins: (*own_logins > 0).then(|| {
                    i18n::filled(
                        Key::TplPluginsRemoveOwnLogins,
                        &[("count", &own_logins.to_string())],
                    )
                }),
                error,
                is_busy,
            },
        };

        Some(dialog)
    }
}
