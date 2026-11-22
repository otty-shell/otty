//! The views of Plugins mode: the left panel of the installed plugins
//! and the Add plugin menu under its chevron, the page of the selected
//! plugin in the main area, and the dialogs over
//! the window.

pub(crate) mod add_menu;
mod can_access;
pub(crate) mod details;
mod forms;
mod permissions;
pub(crate) mod side_panel;

pub(crate) use forms::{FOLDER_FIELD, URL_FIELD};
use iced::Element;
use otty_ui_kit::palette::UiPalette;

use crate::event::PluginsIntent;
use crate::model::DialogViewModel;

/// Render the open dialog as its view model shows it.
pub(crate) fn dialog<'a>(
    vm: DialogViewModel<'a>,
    palette: &UiPalette,
) -> Element<'a, PluginsIntent> {
    match vm {
        DialogViewModel::AddUrl {
            url,
            error,
            is_busy,
        } => forms::add_url(url, error, is_busy, palette),
        DialogViewModel::AddFolder {
            folder,
            error,
            is_busy,
        } => forms::add_folder(folder, error, is_busy, palette),
        DialogViewModel::Install(dialog) => {
            permissions::install(dialog, palette)
        },
        DialogViewModel::Update(dialog) => permissions::update(dialog, palette),
        DialogViewModel::Log { title, text } => {
            forms::log(title, text, palette)
        },
        DialogViewModel::Remove {
            title,
            error,
            is_busy,
        } => forms::remove(title, error, is_busy, palette),
    }
}
