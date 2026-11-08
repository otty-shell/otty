//! The create dialog of the open list: Pull image (image and tag, then the
//! progress line of the pull), Create volume and Create network (name and
//! driver).

use iced::Element;
use iced::widget::{Id, column};
use otty_ui_kit::components::button::{Button, ButtonKind};
use otty_ui_kit::components::dialog::Dialog;
use otty_ui_kit::components::text_field::TextField;
use otty_ui_kit::spacing;

use super::super::event::CreateIntent;
use super::super::model::FormDialogViewModel;
use super::super::types::FormField;
use crate::components::pull_line::pull_line;
use crate::i18n::{self, Key};
use crate::theme::ThemeProps;

/// Props for the create dialog.
pub(crate) struct FormDialogProps<'a> {
    pub(crate) vm: FormDialogViewModel<'a>,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the modal create dialog: its fields, the progress line of a
/// running pull, Cancel and the action button.
pub(crate) fn view(props: FormDialogProps<'_>) -> Element<'_, CreateIntent> {
    let palette = props.theme.theme.ui();
    let vm = props.vm;

    let mut body = column![].spacing(spacing::SPACE_4);
    for field in vm.fields {
        let mut input = TextField::new(field.label, field.value, palette)
            .id(field_id(field.field));
        if !field.placeholder.is_empty() {
            input = input.placeholder(field.placeholder);
        }
        if vm.is_editable {
            let changed = field.field;
            input = input
                .on_input(move |value| CreateIntent::Changed(changed, value))
                .on_submit(CreateIntent::Submitted);
        }
        body = body.push(input);
    }
    if let Some(progress) = vm.progress {
        body = body.push(pull_line(progress, palette));
    }

    let cancel = Button::new(i18n::t(Key::ButtonCancel), palette)
        .kind(ButtonKind::Ghost)
        .on_press(CreateIntent::Cancelled);
    let mut submit =
        Button::new(vm.submit_label, palette).kind(ButtonKind::Primary);
    if vm.can_submit {
        submit = submit.on_press(CreateIntent::Submitted);
    }

    Dialog::new(vm.title, palette)
        .body(body)
        .action(cancel)
        .action(submit)
        .into()
}

/// Widget id of the text field of `field`.
pub(crate) fn field_id(field: FormField) -> Id {
    match field {
        FormField::Name => Id::new("containers_form_name"),
        FormField::Tag => Id::new("containers_form_tag"),
        FormField::Driver => Id::new("containers_form_driver"),
    }
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;

    use super::*;
    use crate::components::pull_line::PullLineViewModel;
    use crate::i18n::{self, Key};
    use crate::theme::ThemeManager;
    use crate::widgets::containers::create::model::FormDialogFieldViewModel;

    fn pull_dialog(can_submit: bool) -> FormDialogViewModel<'static> {
        FormDialogViewModel {
            title: i18n::t(Key::ContainersPullImage),
            fields: vec![
                FormDialogFieldViewModel {
                    field: FormField::Name,
                    label: i18n::t(Key::ContainersFieldImage),
                    value: "",
                    placeholder: "nginx",
                },
                FormDialogFieldViewModel {
                    field: FormField::Tag,
                    label: i18n::t(Key::ContainersFieldTag),
                    value: "latest",
                    placeholder: "",
                },
            ],
            progress: None,
            submit_label: i18n::t(Key::ContainersActionPull),
            can_submit,
            is_editable: true,
        }
    }

    #[test]
    fn typing_in_a_field_sends_its_new_value() {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(FormDialogProps {
            vm: pull_dialog(false),
            theme: ThemeProps::new(themes.current()),
        }));

        ui.click(field_id(FormField::Name))
            .expect("image field should show");
        let _ = ui.typewrite("n");

        let intents: Vec<CreateIntent> = ui.into_messages().collect();
        assert_eq!(
            intents,
            vec![CreateIntent::Changed(FormField::Name, String::from("n"))]
        );
    }

    #[test]
    fn cancel_and_the_action_send_their_intents() {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(FormDialogProps {
            vm: pull_dialog(true),
            theme: ThemeProps::new(themes.current()),
        }));

        ui.click(i18n::t(Key::ButtonCancel))
            .expect("cancel should show");
        ui.click(i18n::t(Key::ContainersActionPull))
            .expect("pull should show");

        let intents: Vec<CreateIntent> = ui.into_messages().collect();
        assert_eq!(
            intents,
            vec![CreateIntent::Cancelled, CreateIntent::Submitted]
        );
    }

    #[test]
    fn a_form_that_cannot_be_sent_has_a_disabled_action() {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(FormDialogProps {
            vm: pull_dialog(false),
            theme: ThemeProps::new(themes.current()),
        }));

        ui.click(i18n::t(Key::ContainersActionPull))
            .expect("pull should show");

        let intents: Vec<CreateIntent> = ui.into_messages().collect();
        assert!(intents.is_empty());
    }

    #[test]
    fn a_running_pull_shows_its_progress_line_and_can_be_cancelled() {
        let themes = ThemeManager::new();
        let vm = FormDialogViewModel {
            is_editable: false,
            progress: Some(PullLineViewModel {
                text: String::from("Pulling nginx:latest"),
                fraction: 0.3,
                amount: Some(String::from("15 MB of 47 MB")),
            }),
            ..pull_dialog(false)
        };
        let mut ui = simulator(view(FormDialogProps {
            vm,
            theme: ThemeProps::new(themes.current()),
        }));

        ui.find("Pulling nginx:latest")
            .expect("progress text should show");
        ui.find("15 MB of 47 MB").expect("amount should show");
        ui.click(field_id(FormField::Name))
            .expect("image field should show");
        let _ = ui.typewrite("x");
        ui.click(i18n::t(Key::ButtonCancel))
            .expect("cancel should show");

        let intents: Vec<CreateIntent> = ui.into_messages().collect();
        assert_eq!(intents, vec![CreateIntent::Cancelled]);
    }
}
