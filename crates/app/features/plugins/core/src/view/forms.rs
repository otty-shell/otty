//! The small dialogs of Plugins mode: the GitHub link of Add plugin…, the
//! folder of Add plugin from folder…, the log of a plugin and the
//! question before Remove.

use iced::widget::{Id, column, text};
use iced::{Element, Length};
use otty_app_i18n::{self as i18n, Key};
use otty_ui_kit::components::button::{Button, ButtonKind};
use otty_ui_kit::components::dialog::Dialog;
use otty_ui_kit::components::small_text::small_text;
use otty_ui_kit::components::text_field::TextField;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{fonts, size, spacing};

use crate::event::PluginsIntent;

/// Widget id of the GitHub link field.
pub(crate) const URL_FIELD: Id = Id::new("plugins_url");
/// Widget id of the folder field.
pub(crate) const FOLDER_FIELD: Id = Id::new("plugins_folder");

/// Add plugin…: the GitHub link, Next and Cancel.
pub(super) fn add_url<'a>(
    url: &'a str,
    error: Option<&'a str>,
    is_busy: bool,
    palette: &UiPalette,
) -> Element<'a, PluginsIntent> {
    let mut field = TextField::new(i18n::t(Key::PluginsUrlLabel), url, palette)
        .id(URL_FIELD)
        .placeholder(i18n::t(Key::PluginsUrlHint))
        .on_escape(PluginsIntent::Back);
    if !is_busy {
        field = field
            .on_input(PluginsIntent::UrlChanged)
            .on_submit(PluginsIntent::Find);
    }

    find_dialog(
        i18n::t(Key::PluginsAddPluginTitle),
        field.into(),
        error,
        is_busy,
        !url.trim().is_empty(),
        palette,
    )
}

/// Add plugin from folder…: the folder, Next and Cancel.
pub(super) fn add_folder<'a>(
    folder: &'a str,
    error: Option<&'a str>,
    is_busy: bool,
    palette: &UiPalette,
) -> Element<'a, PluginsIntent> {
    let mut field =
        TextField::new(i18n::t(Key::PluginsFolderLabel), folder, palette)
            .id(FOLDER_FIELD)
            .placeholder(i18n::t(Key::PluginsFolderHint))
            .on_escape(PluginsIntent::Back);
    if !is_busy {
        field = field
            .on_input(PluginsIntent::FolderChanged)
            .on_submit(PluginsIntent::Find);
    }

    find_dialog(
        i18n::t(Key::PluginsAddFromFolderTitle),
        field.into(),
        error,
        is_busy,
        !folder.trim().is_empty(),
        palette,
    )
}

/// The log of a plugin, as it wrote it, and Back. A long log scrolls in
/// the body of the dialog.
pub(super) fn log<'a>(
    title: String,
    log: String,
    palette: &UiPalette,
) -> Element<'a, PluginsIntent> {
    let lines = text(log)
        .font(fonts::MONO)
        .size(12.0)
        .color(palette.text_body())
        .width(Length::Fill);

    Dialog::new(title, palette)
        .width(size::DIALOG_WIDE)
        .on_close(PluginsIntent::Back, i18n::t(Key::ButtonClose))
        .body(lines)
        .action(back(palette))
        .into()
}

/// The question before Remove: what goes and what stays, and how many of
/// OTTY's own logins go with the plugin, if any.
pub(super) fn remove<'a>(
    title: String,
    own_logins: Option<String>,
    error: Option<&'a str>,
    is_busy: bool,
    palette: &UiPalette,
) -> Element<'a, PluginsIntent> {
    let mut body = column![small_text(
        i18n::t(Key::PluginsRemoveText),
        palette.text_body()
    )]
    .spacing(spacing::SPACE_2);
    if let Some(own_logins) = own_logins {
        body = body.push(small_text(own_logins, palette.text_body()));
    }
    if let Some(error) = error {
        body = body.push(small_text(error, palette.status_error()));
    }
    let cancel = Button::new(i18n::t(Key::ButtonCancel), palette)
        .kind(ButtonKind::Ghost)
        .on_press(PluginsIntent::Back);
    let mut remove = Button::new(i18n::t(Key::PluginsRemove), palette)
        .kind(ButtonKind::Danger);
    if !is_busy {
        remove = remove.on_press(PluginsIntent::Remove);
    }

    Dialog::new(title, palette)
        .body(body)
        .on_close(PluginsIntent::Back, i18n::t(Key::ButtonClose))
        .action(cancel)
        .action(remove)
        .into()
}

/// Back: the dialog closes.
fn back<'a>(palette: &UiPalette) -> Element<'a, PluginsIntent> {
    Button::new(i18n::t(Key::PluginsBack), palette)
        .kind(ButtonKind::Ghost)
        .on_press(PluginsIntent::Back)
        .into()
}

/// A dialog with one field, the line that it is looked for or why it was
/// not found, and Cancel and Next.
fn find_dialog<'a>(
    title: &str,
    field: Element<'a, PluginsIntent>,
    error: Option<&'a str>,
    is_busy: bool,
    has_text: bool,
    palette: &UiPalette,
) -> Element<'a, PluginsIntent> {
    let mut body = column![field].spacing(spacing::SPACE_2);
    if is_busy {
        body = body.push(small_text(
            i18n::t(Key::PluginsLooking),
            palette.text_muted(),
        ));
    }
    if let Some(error) = error {
        body = body.push(small_text(error, palette.status_error()));
    }
    let cancel = Button::new(i18n::t(Key::ButtonCancel), palette)
        .kind(ButtonKind::Ghost)
        .on_press(PluginsIntent::Back);
    let mut next = Button::new(i18n::t(Key::PluginsNext), palette)
        .kind(ButtonKind::Primary);
    if has_text && !is_busy {
        next = next.on_press(PluginsIntent::Find);
    }

    Dialog::new(title, palette)
        .width(size::DIALOG_WIDE)
        .body(body)
        .on_close(PluginsIntent::Back, i18n::t(Key::ButtonClose))
        .action(cancel)
        .action(next)
        .into()
}

#[cfg(test)]
mod tests {
    use iced_test::selector::Candidate;
    use iced_test::{Simulator, simulator};

    use super::*;

    #[test]
    fn next_under_a_typed_link_looks_for_the_plugin() {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(add_url(
            "https://github.com/someone/plugin",
            None,
            false,
            &palette,
        ));

        let _ = ui.click(i18n::t(Key::PluginsNext));

        let sent: Vec<PluginsIntent> = ui.into_messages().collect();
        assert!(matches!(sent.as_slice(), [PluginsIntent::Find]));
    }

    #[test]
    fn a_folder_that_holds_no_plugin_says_why() {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(add_folder(
            "/work/nothing",
            Some("The plugin has no file `otty-plugin.toml`"),
            false,
            &palette,
        ));

        assert!(ui.find("The plugin has no file `otty-plugin.toml`").is_ok());
    }

    /// Issue 33: a long log scrolls in the body of the dialog, the only
    /// area that scrolls, and Back stays in view.
    #[test]
    fn a_long_log_scrolls_once_and_back_stays_in_view() {
        let palette = UiPalette::otty_dark();
        let log_text: String =
            (0..200).map(|line| format!("line {line}\n")).collect();
        let mut ui = Simulator::with_size(
            iced::Settings::default(),
            (1024.0, 600.0),
            log(String::from("Log of Example"), log_text, &palette),
        );

        let mut scrollers = 0;
        let _ = ui.find(|candidate: Candidate<'_>| {
            if matches!(candidate, Candidate::Scrollable { .. }) {
                scrollers += 1;
            }
            None::<()>
        });
        ui.click(i18n::t(Key::PluginsBack))
            .expect("Back should be in view");

        let sent: Vec<PluginsIntent> = ui.into_messages().collect();
        assert_eq!(scrollers, 1);
        assert!(matches!(sent.as_slice(), [PluginsIntent::Back]));
    }
}
