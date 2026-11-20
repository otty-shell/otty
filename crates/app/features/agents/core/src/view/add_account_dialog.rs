//! The Add account dialog over the window (spec 2026-11-19, Decision 7):
//! the provider as a drop-down with what OTTY reads of each, where the key
//! is kept (a file or an environment variable) and its path or name; Cancel
//! and Add. Edit key… shows the same dialog with its provider fixed and
//! Save.

use iced::Element;
use iced::widget::column;
use otty_app_i18n::{self as i18n, Key};
use otty_app_theme::theme::ThemeProps;
use otty_ui_kit::components::button::{Button, ButtonKind};
use otty_ui_kit::components::dialog::Dialog;
use otty_ui_kit::components::note_select::{NoteSelect, NoteSelectOption};
use otty_ui_kit::components::segmented::segmented;
use otty_ui_kit::components::small_text::small_text;
use otty_ui_kit::components::text_field::TextField;
use otty_ui_kit::spacing;

use super::super::add_account_dialog::AddAccountIntent;
use super::super::event::AgentsIntent;
use super::super::model::AddAccountDialogViewModel;
use super::super::profile_dialog::{AdminKeyChoice, AdminKeyChoiceOption};

/// The places of the key, in their order.
const KEY_CHOICES: [AdminKeyChoiceOption; 2] = [
    AdminKeyChoiceOption(AdminKeyChoice::File),
    AdminKeyChoiceOption(AdminKeyChoice::Variable),
];

/// Props for the Add account dialog.
pub(crate) struct AddAccountDialogProps<'a> {
    pub(crate) vm: AddAccountDialogViewModel<'a>,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the modal Add account dialog: Enter adds, Esc cancels.
pub(crate) fn view(
    props: AddAccountDialogProps<'_>,
) -> Element<'_, AgentsIntent> {
    let palette = props.theme.theme.ui();
    let vm = props.vm;
    let send = AgentsIntent::AddAccount;

    let mut body = column![small_text(vm.text, palette.text_muted())]
        .spacing(spacing::SPACE_4);

    let provider = if vm.providers.is_empty() {
        column![
            small_text(
                i18n::t(Key::AgentsColumnProvider),
                palette.text_muted()
            ),
            small_text(vm.provider.name.clone(), palette.text_body()),
        ]
    } else {
        let picked = vm
            .providers
            .iter()
            .position(|option| option.provider == vm.provider.provider);
        let options = vm
            .providers
            .iter()
            .map(|option| {
                NoteSelectOption::new(option.name.clone())
                    .with_note(option.note)
            })
            .collect();
        let providers: Vec<_> = vm
            .providers
            .iter()
            .map(|option| option.provider.clone())
            .collect();
        column![
            small_text(
                i18n::t(Key::AgentsColumnProvider),
                palette.text_muted()
            ),
            NoteSelect::new(options, picked, palette)
                .open(vm.is_provider_list_open)
                .on_toggle(send(AddAccountIntent::ProviderListToggled))
                .on_pick(move |index| {
                    send(AddAccountIntent::ProviderPicked(
                        providers[index].clone(),
                    ))
                }),
        ]
    };
    body = body.push(provider.spacing(spacing::SPACE_2));

    body = body.push(
        column![
            small_text(i18n::t(Key::AgentsFieldKey), palette.text_muted()),
            segmented(
                &KEY_CHOICES,
                &AdminKeyChoiceOption(vm.key_choice),
                move |option| send(AddAccountIntent::KeyChoicePicked(option.0)),
                palette,
            ),
        ]
        .spacing(spacing::SPACE_2),
    );

    let label = match vm.key_choice {
        AdminKeyChoice::File => Key::AgentsAdminKeyFile,
        AdminKeyChoice::Variable => Key::AgentsAdminKeyVariable,
    };
    let mut key = TextField::new(i18n::t(label), vm.key, palette)
        .placeholder(vm.key_placeholder)
        .on_input(move |text| send(AddAccountIntent::KeyChanged(text)))
        .on_escape(send(AddAccountIntent::Cancelled));
    if vm.can_submit {
        key = key.on_submit(send(AddAccountIntent::Submitted));
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
        .on_press(send(AddAccountIntent::Cancelled));
    let mut submit =
        Button::new(vm.submit_label, palette).kind(ButtonKind::Primary);
    if vm.can_submit {
        submit = submit.on_press(send(AddAccountIntent::Submitted));
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
    use otty_agents::ProviderRef;
    use otty_app_theme::theme::ThemeManager;

    use super::*;
    use crate::model::ProviderOption;

    fn deepseek() -> ProviderOption {
        ProviderOption {
            provider: ProviderRef::of_id("deepseek"),
            name: String::from("DeepSeek"),
            note: "Money left",
        }
    }

    fn vm(can_submit: bool) -> AddAccountDialogViewModel<'static> {
        AddAccountDialogViewModel {
            title: "Add account",
            text: "A key no agent uses.",
            providers: vec![deepseek()],
            provider: deepseek(),
            is_provider_list_open: false,
            key_choice: AdminKeyChoice::File,
            key: "~/.config/deepseek-key",
            key_placeholder: String::from("~/.config/deepseek-api-key"),
            key_error: None,
            error: Some(String::from("Could not save")),
            submit_label: "Add",
            can_submit,
        }
    }

    #[test]
    fn the_dialog_shows_its_fields_and_sends_its_answers() {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(AddAccountDialogProps {
            vm: vm(true),
            theme: ThemeProps::new(themes.current()),
        }));

        ui.find("Add account").expect("the title should show");
        ui.find("A key no agent uses.")
            .expect("the text should show");
        ui.find("Could not save").expect("the error should show");
        ui.click(i18n::t(Key::AgentsAdminKeyVariable))
            .expect("the variable choice should show");
        ui.click(i18n::t(Key::ButtonCancel))
            .expect("Cancel should show");
        ui.click("Add").expect("Add should show");

        let intents: Vec<AgentsIntent> = ui.into_messages().collect();
        assert_eq!(
            intents,
            vec![
                AgentsIntent::AddAccount(AddAccountIntent::KeyChoicePicked(
                    AdminKeyChoice::Variable
                )),
                AgentsIntent::AddAccount(AddAccountIntent::Cancelled),
                AgentsIntent::AddAccount(AddAccountIntent::Submitted),
            ]
        );
    }

    #[test]
    fn add_sends_nothing_while_it_is_off() {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(AddAccountDialogProps {
            vm: vm(false),
            theme: ThemeProps::new(themes.current()),
        }));

        ui.click("Add").expect("Add should show");

        assert_eq!(ui.into_messages().count(), 0);
    }
}
