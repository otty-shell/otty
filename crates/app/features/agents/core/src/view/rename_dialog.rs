//! The Rename dialog of an account or a profile (spec 2026-11-19,
//! Decision 7): a line that says what the name is for and one field.

use iced::Element;
use iced::widget::{Id, column};
use otty_app_i18n::{self as i18n, Key};
use otty_app_theme::theme::ThemeProps;
use otty_ui_kit::components::button::{Button, ButtonKind};
use otty_ui_kit::components::dialog::Dialog;
use otty_ui_kit::components::small_text::small_text;
use otty_ui_kit::components::text_field::TextField;
use otty_ui_kit::spacing;

use super::super::event::AgentsIntent;
use super::super::model::RenameDialogViewModel;
use super::super::rename_dialog::RenameIntent;

/// The id of the name field, which takes the keyboard when the dialog
/// opens.
pub(crate) const NAME_FIELD_ID: Id = Id::new("agents_rename_name");

/// Props for the Rename dialog.
pub(crate) struct RenameDialogProps<'a> {
    pub(crate) vm: RenameDialogViewModel<'a>,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the modal Rename dialog: Enter saves, Esc cancels.
pub(crate) fn view(props: RenameDialogProps<'_>) -> Element<'_, AgentsIntent> {
    let palette = props.theme.theme.ui();
    let vm = props.vm;
    let send = AgentsIntent::Rename;

    let mut field = TextField::new(i18n::t(Key::FieldName), vm.name, palette)
        .id(NAME_FIELD_ID)
        .on_input(move |name| send(RenameIntent::NameChanged(name)))
        .on_escape(send(RenameIntent::Cancelled));
    if vm.can_save {
        field = field.on_submit(send(RenameIntent::Submitted));
    }
    if let Some(error) = vm.error {
        field = field.error(error);
    }
    let body = column![small_text(vm.text, palette.text_muted()), field]
        .spacing(spacing::SPACE_3);

    let cancel = Button::new(i18n::t(Key::ButtonCancel), palette)
        .kind(ButtonKind::Ghost)
        .on_press(send(RenameIntent::Cancelled));
    let mut save = Button::new(i18n::t(Key::ButtonSave), palette)
        .kind(ButtonKind::Primary);
    if vm.can_save {
        save = save.on_press(send(RenameIntent::Submitted));
    }

    Dialog::new(vm.title, palette)
        .on_close(send(RenameIntent::Cancelled), i18n::t(Key::ButtonClose))
        .body(body)
        .action(cancel)
        .action(save)
        .into()
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;
    use otty_app_theme::theme::ThemeManager;

    use super::*;

    fn vm(can_save: bool) -> RenameDialogViewModel<'static> {
        RenameDialogViewModel {
            title: "Rename account",
            text: "A name of your own.",
            name: "Home",
            error: Some(String::from("Could not save")),
            can_save,
        }
    }

    #[test]
    fn the_dialog_shows_its_field_and_sends_its_answers() {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(RenameDialogProps {
            vm: vm(true),
            theme: ThemeProps::new(themes.current()),
        }));

        ui.find("Rename account").expect("the title should show");
        ui.find("A name of your own.")
            .expect("the text should show");
        ui.find("Could not save").expect("the error should show");
        ui.click(i18n::t(Key::ButtonCancel))
            .expect("Cancel should show");
        ui.click(i18n::t(Key::ButtonSave))
            .expect("Save should show");

        let intents: Vec<AgentsIntent> = ui.into_messages().collect();
        assert_eq!(
            intents,
            vec![
                AgentsIntent::Rename(RenameIntent::Cancelled),
                AgentsIntent::Rename(RenameIntent::Submitted),
            ]
        );
    }

    #[test]
    fn save_sends_nothing_while_it_is_off() {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(RenameDialogProps {
            vm: vm(false),
            theme: ThemeProps::new(themes.current()),
        }));

        ui.click(i18n::t(Key::ButtonSave))
            .expect("Save should show");

        assert_eq!(ui.into_messages().count(), 0);
    }
}
