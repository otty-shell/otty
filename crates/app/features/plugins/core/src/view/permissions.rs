//! The dialogs that show permissions: the install dialog and the update
//! dialog with what changed. Each part is a compact card with its name,
//! the tag of its kind and its description, then what it can access under
//! "Can access", as the Can access dialog of the page shows it (issues 35,
//! 36). A community plugin's note is a warning (owner, 2026-11-22).

use iced::Element;
use iced::widget::{Column, column, svg};
use otty_app_i18n::{self as i18n, Key};
use otty_app_theme::icons;
use otty_ui_kit::components::button::{Button, ButtonKind};
use otty_ui_kit::components::dialog::Dialog;
use otty_ui_kit::components::notice::{Notice, NoticeTone};
use otty_ui_kit::components::small_text::small_text;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{size, spacing};

use super::can_access::part_card;
use crate::event::PluginsIntent;
use crate::model::{
    InstallDialogViewModel, PartPermissionsViewModel, UpdateDialogViewModel,
};

/// The install dialog: who reviewed the plugin and who made it, each part
/// with its permissions, then Cancel and Install.
pub(super) fn install<'a>(
    dialog: InstallDialogViewModel,
    palette: &UiPalette,
) -> Element<'a, PluginsIntent> {
    let trust: Element<'a, PluginsIntent> = if dialog.is_trust_warn {
        Notice::new(
            dialog.trust,
            svg::Handle::from_memory(icons::NOTICE_WARN),
            palette,
        )
        .tone(NoticeTone::Warn)
        .into()
    } else {
        small_text(dialog.trust, palette.text_body()).into()
    };
    let mut about = column![trust].spacing(spacing::SPACE_1);
    if let Some(authors) = dialog.authors {
        about = about.push(small_text(authors, palette.text_muted()));
    }
    if let Some(repository) = dialog.repository {
        about = about.push(small_text(repository, palette.text_muted()));
    }

    let mut body =
        column![about, parts(dialog.parts, palette)].spacing(spacing::SPACE_5);
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
    let mut body = column![
        small_text(i18n::t(Key::PluginsUpdateAsks), palette.text_body()),
        parts(dialog.parts, palette),
    ]
    .spacing(spacing::SPACE_5);
    if let Some(error) = dialog.error {
        body = body.push(small_text(error, palette.status_error()));
    }

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

/// Each part as its card, "Can access" over its groups.
fn parts<'a>(
    parts: Vec<PartPermissionsViewModel>,
    palette: &UiPalette,
) -> Element<'a, PluginsIntent> {
    let mut list = Column::new().spacing(spacing::SPACE_5);
    for part in parts {
        list = list.push(part_card(part, palette));
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

    fn install_dialog(
        trust: &'static str,
        is_trust_warn: bool,
        parts: Vec<PartPermissionsViewModel>,
    ) -> InstallDialogViewModel {
        InstallDialogViewModel {
            title: String::from("Install Example 0.1.0"),
            authors: None,
            repository: None,
            trust,
            is_trust_warn,
            parts,
            error: None,
            is_busy: false,
        }
    }

    fn cloud() -> PartPermissionsViewModel {
        PartPermissionsViewModel {
            name: String::from("Example Cloud"),
            kind: "Provider",
            description: Some(String::from(
                "Reads the limits of Example Cloud accounts.",
            )),
            mark: None,
            is_removed: false,
            groups: vec![PermissionGroupViewModel {
                label: String::from("Sites"),
                meaning: Some(String::from("Sends requests only to these")),
                places: vec![PlaceViewModel {
                    group: PermissionGroup::Sites,
                    text: String::from("api.example.com"),
                    is_words: false,
                    lead: None,
                    note: None,
                    profiles_of: None,
                    mark: None,
                }],
            }],
        }
    }

    /// Issue 35, 36: the install dialog shows each part as its card, with
    /// its kind, description and what it can access under "Can access",
    /// each group with what it means; the line that everything else is
    /// refused is left to the Can access dialog of the page.
    #[test]
    fn the_install_dialog_shows_each_part_and_its_groups() {
        let palette = UiPalette::otty_dark();
        let dialog =
            install_dialog("Official plugin", false, vec![cloud(), cloud()]);
        let mut ui = simulator(install(dialog, &palette));

        assert!(ui.find("Provider").is_ok());
        assert!(
            ui.find("Reads the limits of Example Cloud accounts.")
                .is_ok()
        );
        assert!(ui.find(i18n::t(Key::PluginsCanAccess)).is_ok());
        assert!(ui.find("Sites").is_ok());
        assert!(ui.find("Sends requests only to these").is_ok());
        assert!(ui.find("api.example.com").is_ok());
        assert!(ui.find(i18n::t(Key::PluginsCanAccessNote)).is_err());
    }

    /// Owner, 2026-11-22: the note of a community plugin, reviewed by its
    /// author only, is a warning; it shows as the line it is.
    #[test]
    fn the_note_of_a_community_plugin_is_a_warning() {
        let palette = UiPalette::otty_dark();
        let dialog = install_dialog(
            "Community plugin: reviewed by its author only",
            true,
            Vec::new(),
        );
        let mut ui = simulator(install(dialog, &palette));

        assert!(
            ui.find("Community plugin: reviewed by its author only")
                .is_ok()
        );
    }

    #[test]
    fn install_of_the_install_dialog_installs() {
        let palette = UiPalette::otty_dark();
        let dialog = install_dialog("Community plugin", true, Vec::new());
        let mut ui = simulator(install(dialog, &palette));

        let _ = ui.click(i18n::t(Key::PluginsInstall));

        let sent: Vec<PluginsIntent> = ui.into_messages().collect();
        assert!(matches!(sent.as_slice(), [PluginsIntent::Install]));
    }

    /// Issue 36: the update dialog shows the parts as cards too, without
    /// the line that everything else is refused.
    #[test]
    fn the_update_dialog_shows_each_part_as_its_card() {
        let palette = UiPalette::otty_dark();
        let dialog = UpdateDialogViewModel {
            title: String::from("Update Example 0.1.0 → 0.2.0"),
            parts: vec![cloud()],
            error: None,
            is_busy: false,
        };
        let mut ui = simulator(update(dialog, &palette));

        assert!(ui.find(i18n::t(Key::PluginsUpdateAsks)).is_ok());
        assert!(
            ui.find("Reads the limits of Example Cloud accounts.")
                .is_ok()
        );
        assert!(ui.find(i18n::t(Key::PluginsCanAccess)).is_ok());
        assert!(ui.find(i18n::t(Key::PluginsCanAccessNote)).is_err());
    }
}
