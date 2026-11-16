//! The dialog that asks before an added profile is removed: its config
//! folder and its sessions stay on disk.

use iced::Element;
use iced::widget::text;
use otty_app_i18n::{self as i18n, Key};
use otty_app_theme::theme::ThemeProps;
use otty_ui_kit::components::button::{Button, ButtonKind};
use otty_ui_kit::components::dialog::Dialog;
use otty_ui_kit::{fonts, typography};

use super::super::event::AgentsIntent;
use super::super::model::RemoveDialogViewModel;

/// Props for the remove dialog.
pub(crate) struct RemoveDialogProps<'a> {
    pub(crate) vm: RemoveDialogViewModel,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the modal remove dialog.
pub(crate) fn view(props: RemoveDialogProps<'_>) -> Element<'_, AgentsIntent> {
    let palette = props.theme.theme.ui();
    let vm = props.vm;
    let hint = typography::SMALL;

    let body = text(vm.text)
        .size(hint.size())
        .line_height(iced::Pixels(hint.line_height()))
        .font(fonts::UI)
        .color(palette.text_muted());
    let cancel = Button::new(i18n::t(Key::ButtonCancel), palette)
        .kind(ButtonKind::Ghost)
        .on_press(AgentsIntent::RemoveCancelled);
    let confirm = Button::new(vm.confirm_label, palette)
        .kind(ButtonKind::Danger)
        .on_press(AgentsIntent::RemoveConfirmed);

    Dialog::new(vm.title, palette)
        .body(body)
        .action(cancel)
        .action(confirm)
        .into()
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;
    use otty_app_theme::theme::ThemeManager;

    use super::*;

    #[test]
    fn the_dialog_says_the_folder_stays_and_sends_its_answers() {
        let vm = RemoveDialogViewModel {
            title: String::from("Remove Claude Code · Work?"),
            text: String::from("The folder stays on disk."),
            confirm_label: i18n::t(Key::ButtonRemove),
        };
        let themes = ThemeManager::new();
        let mut ui = simulator(view(RemoveDialogProps {
            vm,
            theme: ThemeProps::new(themes.current()),
        }));

        ui.find("Remove Claude Code · Work?")
            .expect("title should show");
        ui.find("The folder stays on disk.")
            .expect("text should show");
        ui.click(i18n::t(Key::ButtonCancel))
            .expect("cancel should show");
        ui.click(i18n::t(Key::ButtonRemove))
            .expect("remove should show");

        let intents: Vec<AgentsIntent> = ui.into_messages().collect();
        assert_eq!(
            intents,
            vec![AgentsIntent::RemoveCancelled, AgentsIntent::RemoveConfirmed]
        );
    }
}
