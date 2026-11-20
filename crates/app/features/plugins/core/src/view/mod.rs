//! The views of the Plugins page: one wide dialog over the window that
//! shows the list of the installed plugins or one of its dialogs.

mod forms;
mod list;
mod permissions;

pub(crate) use forms::{FOLDER_FIELD, URL_FIELD};
use iced::Element;
use otty_app_theme::theme::ThemeProps;

use crate::event::PluginsIntent;
use crate::model::PageViewModel;

/// Props for the page.
pub(crate) struct PageProps<'a> {
    pub(crate) vm: PageViewModel<'a>,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the page as what its view model shows.
pub(crate) fn view(props: PageProps<'_>) -> Element<'_, PluginsIntent> {
    let palette = props.theme.theme.ui();

    match props.vm {
        PageViewModel::List {
            rows,
            error,
            is_busy,
        } => list::view(rows, error, is_busy, palette),
        PageViewModel::AddUrl {
            url,
            error,
            is_busy,
        } => forms::add_url(url, error, is_busy, palette),
        PageViewModel::AddFolder {
            folder,
            error,
            is_busy,
        } => forms::add_folder(folder, error, is_busy, palette),
        PageViewModel::Install(dialog) => permissions::install(dialog, palette),
        PageViewModel::Update(dialog) => permissions::update(dialog, palette),
        PageViewModel::Permissions(dialog) => {
            permissions::granted(dialog, palette)
        },
        PageViewModel::Log { title, text } => forms::log(title, text, palette),
        PageViewModel::Remove {
            title,
            error,
            is_busy,
        } => forms::remove(title, error, is_busy, palette),
    }
}
