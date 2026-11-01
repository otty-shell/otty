//! The dialog a destructive action asks first with: what goes, an optional
//! switch ("Stop it first", "Force"), Cancel and the action.

use iced::Element;
use iced::widget::{column, text};
use otty_ui_kit::components::button::{Button, ButtonKind};
use otty_ui_kit::components::dialog::Dialog;
use otty_ui_kit::components::switch::Switch;
use otty_ui_kit::{fonts, spacing, typography};

use super::super::action_model::ConfirmViewModel;
use super::super::event::DockerIntent;
use crate::i18n::{self, Key};
use crate::theme::ThemeProps;

/// Props for the confirmation dialog.
pub(crate) struct ConfirmDialogProps<'a> {
    pub(crate) vm: ConfirmViewModel,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the modal confirmation dialog.
pub(crate) fn view(props: ConfirmDialogProps<'_>) -> Element<'_, DockerIntent> {
    let palette = props.theme.theme.ui();
    let vm = props.vm;
    let hint = typography::SMALL;

    let mut body = column![
        text(vm.text)
            .size(hint.size())
            .line_height(iced::Pixels(hint.line_height()))
            .font(fonts::UI)
            .color(palette.text_muted())
    ]
    .spacing(spacing::SPACE_4);
    if let Some((label, is_on)) = vm.switch {
        body = body.push(
            Switch::new(is_on, palette)
                .label(label)
                .on_toggle(DockerIntent::ConfirmSwitchToggled),
        );
    }

    let cancel = Button::new(i18n::t(Key::ButtonCancel), palette)
        .kind(ButtonKind::Ghost)
        .on_press(DockerIntent::ConfirmCancelled);
    let confirm = Button::new(vm.confirm_label, palette)
        .kind(ButtonKind::Danger)
        .on_press(DockerIntent::ConfirmAccepted);

    Dialog::new(vm.title, palette)
        .body(body)
        .action(cancel)
        .action(confirm)
        .into()
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;

    use super::*;
    use crate::i18n::{self, Key};
    use crate::theme::ThemeManager;

    fn remove_dialog(switch: Option<bool>) -> ConfirmViewModel {
        ConfirmViewModel {
            title: String::from("Remove container web-1?"),
            text: String::from("The container is deleted."),
            switch: switch.map(|on| (i18n::t(Key::DockerStopItFirst), on)),
            confirm_label: i18n::t(Key::DockerActionRemove),
        }
    }

    #[test]
    fn the_dialog_shows_its_title_text_and_switch() {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(ConfirmDialogProps {
            vm: remove_dialog(Some(true)),
            theme: ThemeProps::new(themes.current()),
        }));

        ui.find("Remove container web-1?")
            .expect("title should show");
        ui.find("The container is deleted.")
            .expect("text should show");
        ui.find(i18n::t(Key::DockerStopItFirst))
            .expect("switch label should show");
    }

    #[test]
    fn cancel_and_the_action_send_their_intents() {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(ConfirmDialogProps {
            vm: remove_dialog(None),
            theme: ThemeProps::new(themes.current()),
        }));

        ui.click(i18n::t(Key::ButtonCancel))
            .expect("cancel should show");
        ui.click(i18n::t(Key::DockerActionRemove))
            .expect("remove should show");

        let intents: Vec<DockerIntent> = ui.into_messages().collect();
        assert_eq!(
            intents,
            vec![
                DockerIntent::ConfirmCancelled,
                DockerIntent::ConfirmAccepted
            ]
        );
    }

    #[test]
    fn a_dialog_without_a_switch_shows_none() {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(ConfirmDialogProps {
            vm: remove_dialog(None),
            theme: ThemeProps::new(themes.current()),
        }));

        assert!(ui.find(i18n::t(Key::DockerStopItFirst)).is_err());
    }

    #[test]
    fn the_switch_sends_its_new_state() {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(ConfirmDialogProps {
            vm: remove_dialog(Some(true)),
            theme: ThemeProps::new(themes.current()),
        }));

        ui.click(i18n::t(Key::DockerStopItFirst))
            .expect("switch should show");

        let intents: Vec<DockerIntent> = ui.into_messages().collect();
        assert_eq!(intents, vec![DockerIntent::ConfirmSwitchToggled(false)]);
    }
}
