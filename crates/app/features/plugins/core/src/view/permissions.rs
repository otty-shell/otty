//! The dialogs that show permissions: the install dialog and the update
//! dialog with what changed. Each part is its name with the tag of its
//! kind, then what it can access, grouped as on the page of a plugin
//! (issue 35).

use iced::widget::{Column, column, row, text};
use iced::{Alignment, Element};
use otty_app_i18n::{self as i18n, Key};
use otty_ui_kit::components::button::{Button, ButtonKind};
use otty_ui_kit::components::dialog::Dialog;
use otty_ui_kit::components::small_text::small_text;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{fonts, size, spacing};

use super::can_access::{can_access, kind_tag};
use crate::event::PluginsIntent;
use crate::model::{
    InstallDialogViewModel, PartPermissionsViewModel, UpdateDialogViewModel,
};

/// The install dialog: who made the plugin and who reviewed it, each part
/// with its permissions, then Cancel and Install.
pub(super) fn install<'a>(
    dialog: InstallDialogViewModel,
    palette: &UiPalette,
) -> Element<'a, PluginsIntent> {
    let mut body = Column::new().spacing(spacing::SPACE_4);
    let mut about = column![small_text(dialog.trust, palette.text_body())]
        .spacing(spacing::SPACE_1);
    if let Some(authors) = dialog.authors {
        about = about.push(small_text(authors, palette.text_muted()));
    }
    if let Some(repository) = dialog.repository {
        about = about.push(small_text(repository, palette.text_muted()));
    }
    body = body.push(about);
    body = body.push(parts(dialog.parts, palette));
    if let Some(error) = dialog.error {
        body = body.push(small_text(error, palette.status_error()));
    }

    let cancel = Button::new(i18n::t(Key::ButtonCancel), palette)
        .kind(ButtonKind::Ghost)
        .on_press(PluginsIntent::Back);
    let mut install = Button::new(i18n::t(Key::PluginsInstall), palette)
        .kind(ButtonKind::Primary);
    if !dialog.is_busy {
        install = install.on_press(PluginsIntent::Install);
    }

    Dialog::new(dialog.title, palette)
        .width(size::DIALOG_WIDE)
        .body(body)
        .action(cancel)
        .action(install)
        .into()
}

/// The update dialog: what changed part by part, then Not now and
/// Update.
pub(super) fn update<'a>(
    dialog: UpdateDialogViewModel,
    palette: &UiPalette,
) -> Element<'a, PluginsIntent> {
    let body = column![
        small_text(i18n::t(Key::PluginsUpdateAsks), palette.text_body()),
        parts(dialog.parts, palette),
    ]
    .spacing(spacing::SPACE_4);
    let body = match dialog.error {
        Some(error) => body.push(small_text(error, palette.status_error())),
        None => body,
    };

    let not_now = Button::new(i18n::t(Key::PluginsNotNow), palette)
        .kind(ButtonKind::Ghost)
        .on_press(PluginsIntent::Back);
    let mut update = Button::new(i18n::t(Key::PluginsUpdate), palette)
        .kind(ButtonKind::Primary);
    if !dialog.is_busy {
        update = update.on_press(PluginsIntent::ApplyUpdate);
    }

    Dialog::new(dialog.title, palette)
        .width(size::DIALOG_WIDE)
        .body(body)
        .action(not_now)
        .action(update)
        .into()
}

/// Each part: its name, the tag of its kind and in the update dialog its
/// mark, then what it can access, unless the new version no longer has
/// it.
fn parts<'a>(
    parts: Vec<PartPermissionsViewModel>,
    palette: &UiPalette,
) -> Element<'a, PluginsIntent> {
    let mut list = Column::new().spacing(spacing::SPACE_5);

    for part in parts {
        let mut title = row![
            text(part.name)
                .font(fonts::UI_SEMIBOLD)
                .color(palette.text_strong()),
            kind_tag(part.kind, palette),
        ]
        .spacing(spacing::SPACE_3)
        .align_y(Alignment::Center);
        if let Some(mark) = part.mark {
            title = title.push(small_text(mark, palette.status_warn()));
        }

        let mut block = column![title].spacing(spacing::SPACE_3);
        if !part.is_removed {
            block = block.push(can_access(part.groups, palette));
        }
        list = list.push(block);
    }

    list.into()
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;

    use super::*;
    use crate::model::permissions::{
        PermissionGroup, PermissionGroupViewModel, PlaceViewModel,
    };

    /// Issue 35: the install dialog shows each part with the tag of its
    /// kind and what it can access, grouped as on the page.
    #[test]
    fn the_install_dialog_shows_the_same_groups() {
        let palette = UiPalette::otty_dark();
        let dialog = InstallDialogViewModel {
            title: String::from("Install Example 0.1.0"),
            authors: None,
            repository: None,
            trust: "Community plugin: reviewed by its author only",
            parts: vec![PartPermissionsViewModel {
                name: String::from("Example Cloud"),
                kind: "Provider",
                mark: None,
                is_removed: false,
                groups: vec![PermissionGroupViewModel {
                    label: "Sites",
                    places: vec![PlaceViewModel {
                        group: PermissionGroup::Sites,
                        text: String::from("api.example.com"),
                        is_words: false,
                        note: None,
                        mark: None,
                    }],
                }],
            }],
            error: None,
            is_busy: false,
        };
        let mut ui = simulator(install(dialog, &palette));

        assert!(ui.find("Example Cloud").is_ok());
        assert!(ui.find("Provider").is_ok());
        assert!(ui.find(i18n::t(Key::PluginsCanAccess)).is_ok());
        assert!(ui.find("Sites").is_ok());
        assert!(ui.find("api.example.com").is_ok());
    }

    #[test]
    fn install_of_the_install_dialog_installs() {
        let palette = UiPalette::otty_dark();
        let dialog = InstallDialogViewModel {
            title: String::from("Install Example Fork 0.1.0"),
            authors: None,
            repository: None,
            trust: "Community plugin: reviewed by its author only",
            parts: Vec::new(),
            error: None,
            is_busy: false,
        };
        let mut ui = simulator(install(dialog, &palette));

        let _ = ui.click(i18n::t(Key::PluginsInstall));

        let sent: Vec<PluginsIntent> = ui.into_messages().collect();
        assert!(matches!(sent.as_slice(), [PluginsIntent::Install]));
    }
}
