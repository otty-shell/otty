//! The dialogs that show permissions: the install dialog, the update
//! dialog with what changed, and the permissions of an installed plugin.
//! Each part is a title and its lines.

use iced::Element;
use iced::widget::{Column, column, row, text};
use otty_app_i18n::{self as i18n, Key};
use otty_ui_kit::components::button::{Button, ButtonKind};
use otty_ui_kit::components::dialog::Dialog;
use otty_ui_kit::components::small_text::small_text;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{fonts, size, spacing};

use super::forms::back;
use crate::event::PluginsIntent;
use crate::model::{
    InstallDialogViewModel, PartPermissionsViewModel, PermissionsViewModel,
    UpdateDialogViewModel,
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
    body = body.push(parts(dialog.parts, false, palette));
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
        parts(dialog.parts, true, palette),
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

/// The permissions the user granted to each part of a plugin, and Back.
pub(super) fn granted<'a>(
    dialog: PermissionsViewModel,
    palette: &UiPalette,
) -> Element<'a, PluginsIntent> {
    Dialog::new(dialog.title, palette)
        .width(size::DIALOG_WIDE)
        .on_close(PluginsIntent::Back, i18n::t(Key::ButtonClose))
        .body(parts(dialog.parts, false, palette))
        .action(back(palette))
        .into()
}

/// Each part: its title with its mark, its lines, marked new in the
/// update dialog (`marks_new`), and the lines it no longer asks for.
fn parts<'a>(
    parts: Vec<PartPermissionsViewModel>,
    marks_new: bool,
    palette: &UiPalette,
) -> Element<'a, PluginsIntent> {
    let mut list = Column::new().spacing(spacing::SPACE_3);

    for part in parts {
        let mut title = row![
            text(part.title)
                .font(fonts::UI_SEMIBOLD)
                .color(palette.text_strong())
        ]
        .spacing(spacing::SPACE_2);
        if let Some(mark) = part.mark {
            title = title.push(small_text(mark, palette.status_warn()));
        }

        let mut lines = column![title].spacing(spacing::SPACE_1);
        if part.lines.is_empty()
            && part.removed.is_empty()
            && part.mark.is_none()
        {
            lines = lines.push(small_text(
                i18n::t(Key::PluginsNoPermissions),
                palette.text_muted(),
            ));
        }
        for line in part.lines {
            let line = if marks_new {
                i18n::filled(
                    Key::TplPluginsNewPermission,
                    &[("permission", &line)],
                )
            } else {
                format!("• {line}")
            };
            lines = lines.push(small_text(line, palette.text_body()));
        }
        for line in part.removed {
            let removed =
                i18n::filled(Key::TplPluginsNoLonger, &[("permission", &line)]);
            lines = lines.push(small_text(removed, palette.text_muted()));
        }
        list = list.push(lines);
    }

    list.into()
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;

    use super::*;

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
