use iced::Element;
use iced::widget::text;
use otty_app_i18n::{self as i18n, Key};
use otty_app_theme::theme::ThemeProps;
use otty_ui_kit::components::button::{Button, ButtonKind};
use otty_ui_kit::components::dialog::Dialog;
use otty_ui_kit::{fonts, typography};

use crate::event::ExplorerIntent;
use crate::model::DeleteDialogViewModel;

/// Render the dialog that asks before a file or a folder is deleted for
/// good: what goes, Cancel and Delete.
pub(crate) fn view<'a>(
    vm: DeleteDialogViewModel<'_>,
    theme: ThemeProps<'a>,
) -> Element<'a, ExplorerIntent> {
    let palette = theme.theme.ui();
    let hint = typography::SMALL;

    let key = if vm.is_folder {
        Key::FilesDeleteFolderText
    } else {
        Key::FilesDeleteFileText
    };
    let body = text(i18n::t(key))
        .size(hint.size())
        .line_height(iced::Pixels(hint.line_height()))
        .font(fonts::UI)
        .color(palette.text_muted());

    let cancel = Button::new(i18n::t(Key::ButtonCancel), palette)
        .kind(ButtonKind::Ghost)
        .on_press(ExplorerIntent::DeleteCancelled);
    let delete = Button::new(i18n::t(Key::Delete), palette)
        .kind(ButtonKind::Danger)
        .on_press(ExplorerIntent::DeleteConfirmed);

    Dialog::new(i18n::files_delete_title(vm.name), palette)
        .body(body)
        .action(cancel)
        .action(delete)
        .into()
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;
    use otty_app_i18n::{self as i18n, Key};
    use otty_app_theme::theme::{ThemeManager, ThemeProps};

    use super::view;
    use crate::event::ExplorerIntent;
    use crate::model::DeleteDialogViewModel;

    #[test]
    fn the_dialog_names_the_folder_and_says_what_goes() {
        let themes = ThemeManager::new();
        let vm = DeleteDialogViewModel {
            name: "src",
            is_folder: true,
        };
        let mut ui = simulator(view(vm, ThemeProps::new(themes.current())));

        assert!(ui.find(i18n::files_delete_title("src")).is_ok());
        assert!(ui.find(i18n::t(Key::FilesDeleteFolderText)).is_ok());
    }

    #[test]
    fn cancel_and_delete_send_their_answers() {
        let themes = ThemeManager::new();
        let vm = DeleteDialogViewModel {
            name: "main.rs",
            is_folder: false,
        };
        let mut ui = simulator(view(vm, ThemeProps::new(themes.current())));

        assert!(ui.find(i18n::t(Key::FilesDeleteFileText)).is_ok());
        ui.click(i18n::t(Key::ButtonCancel))
            .expect("cancel should show");
        ui.click(i18n::t(Key::Delete)).expect("delete should show");

        let intents: Vec<ExplorerIntent> = ui.into_messages().collect();
        assert!(matches!(
            intents.as_slice(),
            [
                ExplorerIntent::DeleteCancelled,
                ExplorerIntent::DeleteConfirmed
            ]
        ));
    }
}
