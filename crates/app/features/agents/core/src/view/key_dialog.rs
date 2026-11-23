//! The dialog of a key a plugin part asks for, over the window: named by
//! the part's label, with the plugin that asks, its hint, what the key
//! starts with and a button to its page, the provider of the account,
//! where the key is kept (a file or an environment variable) and its path
//! or name; Cancel and Save.

use iced::Element;
use iced::widget::column;
use otty_app_i18n::{self as i18n, Key};
use otty_app_theme::theme::ThemeProps;
use otty_ui_kit::components::button::{Button, ButtonKind};
use otty_ui_kit::components::dialog::Dialog;
use otty_ui_kit::components::segmented::segmented;
use otty_ui_kit::components::small_text::small_text;
use otty_ui_kit::components::text_field::TextField;
use otty_ui_kit::spacing;

use super::super::event::AgentsIntent;
use super::super::key_dialog::KeyDialogIntent;
use super::super::key_place_field::{KeyPlaceChoice, KeyPlaceChoiceOption};
use super::super::model::KeyDialogViewModel;

/// The places of the key, in their order.
const KEY_CHOICES: [KeyPlaceChoiceOption; 2] = [
    KeyPlaceChoiceOption(KeyPlaceChoice::File),
    KeyPlaceChoiceOption(KeyPlaceChoice::Variable),
];

/// Props for the key dialog.
pub(crate) struct KeyDialogProps<'a> {
    pub(crate) vm: KeyDialogViewModel<'a>,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the modal key dialog: Enter saves, Esc cancels.
pub(crate) fn view(props: KeyDialogProps<'_>) -> Element<'_, AgentsIntent> {
    let palette = props.theme.theme.ui();
    let vm = props.vm;
    let send = AgentsIntent::KeyDialog;

    let mut body = column![
        small_text(vm.plugin, palette.text_muted()),
        small_text(vm.text, palette.text_muted()),
    ]
    .spacing(spacing::SPACE_4);
    if let Some(prefix) = vm.prefix {
        body = body.push(small_text(prefix, palette.text_muted()));
    }
    if let Some(page) = vm.page {
        body = body.push(
            Button::new(page, palette)
                .kind(ButtonKind::Secondary)
                .on_press(send(KeyDialogIntent::PageOpened)),
        );
    }

    body = body.push(
        column![
            small_text(
                i18n::t(Key::AgentsColumnProvider),
                palette.text_muted()
            ),
            small_text(vm.provider, palette.text_body()),
        ]
        .spacing(spacing::SPACE_2),
    );
    body = body.push(
        column![
            small_text(i18n::t(Key::AgentsFieldKey), palette.text_muted()),
            segmented(
                &KEY_CHOICES,
                &KeyPlaceChoiceOption(vm.key_choice),
                move |option| send(KeyDialogIntent::KeyChoicePicked(option.0)),
                palette,
            ),
        ]
        .spacing(spacing::SPACE_2),
    );

    let label = match vm.key_choice {
        KeyPlaceChoice::File => Key::AgentsKeyPlaceFile,
        KeyPlaceChoice::Variable => Key::AgentsKeyPlaceVariable,
    };
    let mut key = TextField::new(i18n::t(label), vm.key, palette)
        .placeholder(vm.key_placeholder)
        .on_input(move |text| send(KeyDialogIntent::KeyChanged(text)))
        .on_escape(send(KeyDialogIntent::Cancelled));
    if vm.can_submit {
        key = key.on_submit(send(KeyDialogIntent::Submitted));
    }
    if let Some(error) = vm.key_error {
        key = key.error(error);
    }
    body = body.push(key);

    if let Some(error) = vm.error {
        body = body.push(small_text(error, palette.status_error()));
    }

    let cancel = Button::new(i18n::t(Key::ButtonCancel), palette)
        .kind(ButtonKind::Ghost)
        .on_press(send(KeyDialogIntent::Cancelled));
    let mut submit = Button::new(i18n::t(Key::ButtonSave), palette)
        .kind(ButtonKind::Primary);
    if vm.can_submit {
        submit = submit.on_press(send(KeyDialogIntent::Submitted));
    }

    Dialog::new(vm.title, palette)
        .body(body)
        .action(cancel)
        .action(submit)
        .into()
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;
    use otty_app_theme::theme::ThemeManager;

    use super::*;

    fn vm(can_submit: bool) -> KeyDialogViewModel<'static> {
        KeyDialogViewModel {
            title: String::from("Admin key"),
            text: String::from("Reads the real bill."),
            plugin: String::from("Example plugin"),
            prefix: Some(String::from("The key starts with ex-admin-.")),
            page: Some(String::from("Open example.com")),
            provider: String::from("Example Cloud"),
            key_choice: KeyPlaceChoice::File,
            key: "~/.config/example-admin.key",
            key_placeholder: String::from("~/.config/example-admin.key"),
            key_error: None,
            error: Some(String::from("Could not save")),
            can_submit,
        }
    }

    #[test]
    fn the_dialog_shows_its_fields_and_sends_its_answers() {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(KeyDialogProps {
            vm: vm(true),
            theme: ThemeProps::new(themes.current()),
        }));

        ui.find("Admin key").expect("the label names the dialog");
        ui.find("Example plugin").expect("the plugin should show");
        ui.find("Reads the real bill.")
            .expect("the hint should show");
        ui.find("The key starts with ex-admin-.")
            .expect("the prefix should show");
        ui.find("Example Cloud").expect("the provider should show");
        ui.find("Could not save").expect("the error should show");
        ui.click("Open example.com").expect("the page should show");
        ui.click(i18n::t(Key::AgentsKeyPlaceVariable))
            .expect("the variable choice should show");
        ui.click(i18n::t(Key::ButtonCancel))
            .expect("Cancel should show");
        ui.click(i18n::t(Key::ButtonSave))
            .expect("Save should show");

        let intents: Vec<AgentsIntent> = ui.into_messages().collect();
        assert_eq!(
            intents,
            vec![
                AgentsIntent::KeyDialog(KeyDialogIntent::PageOpened),
                AgentsIntent::KeyDialog(KeyDialogIntent::KeyChoicePicked(
                    KeyPlaceChoice::Variable
                )),
                AgentsIntent::KeyDialog(KeyDialogIntent::Cancelled),
                AgentsIntent::KeyDialog(KeyDialogIntent::Submitted),
            ]
        );
    }

    #[test]
    fn save_sends_nothing_while_it_is_off() {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(KeyDialogProps {
            vm: vm(false),
            theme: ThemeProps::new(themes.current()),
        }));

        ui.click(i18n::t(Key::ButtonSave))
            .expect("Save should show");

        assert_eq!(ui.into_messages().count(), 0);
    }
}
