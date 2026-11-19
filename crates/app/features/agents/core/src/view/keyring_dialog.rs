//! The dialog that asks the user to allow OTTY to read the system keyring:
//! Not now reads nothing, Allow saves the allowance and reads it.

use iced::Element;
use iced::widget::column;
use otty_app_i18n::{self as i18n, Key};
use otty_app_theme::theme::ThemeProps;
use otty_ui_kit::components::button::{Button, ButtonKind};
use otty_ui_kit::components::dialog::Dialog;
use otty_ui_kit::components::small_text::small_text;

use super::super::event::AgentsIntent;
use super::super::model::KeyringDialogViewModel;

/// The space between the text of the dialog and the services it reads.
const LINE_GAP: f32 = 8.0;

/// Props for the keyring dialog.
pub(crate) struct KeyringDialogProps<'a> {
    pub(crate) vm: KeyringDialogViewModel,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the modal keyring dialog.
pub(crate) fn view(props: KeyringDialogProps<'_>) -> Element<'_, AgentsIntent> {
    let palette = props.theme.theme.ui();
    let vm = props.vm;
    let body = column![
        small_text(vm.text, palette.text_muted()),
        small_text(vm.services, palette.text_muted()),
    ]
    .spacing(LINE_GAP);
    let not_now = Button::new(i18n::t(Key::AgentsNotNow), palette)
        .kind(ButtonKind::Ghost)
        .on_press(AgentsIntent::KeyringNotNow);
    let allow = Button::new(i18n::t(Key::AgentsAllow), palette)
        .kind(ButtonKind::Primary)
        .on_press(AgentsIntent::AllowKeyring);

    Dialog::new(vm.title, palette)
        .body(body)
        .action(not_now)
        .action(allow)
        .into()
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;
    use otty_app_theme::theme::ThemeManager;

    use super::*;

    #[test]
    fn the_dialog_says_what_it_reads_and_sends_its_answers() {
        let vm = KeyringDialogViewModel {
            title: "Read the system keyring?",
            text: String::from("Goose keeps its keys in the system keyring."),
            services: String::from("Keyring service: goose"),
        };
        let themes = ThemeManager::new();
        let mut ui = simulator(view(KeyringDialogProps {
            vm,
            theme: ThemeProps::new(themes.current()),
        }));

        ui.find("Read the system keyring?")
            .expect("title should show");
        ui.find("Keyring service: goose")
            .expect("the service should show");
        ui.click(i18n::t(Key::AgentsNotNow))
            .expect("Not now should show");
        ui.click(i18n::t(Key::AgentsAllow))
            .expect("Allow should show");

        let intents: Vec<AgentsIntent> = ui.into_messages().collect();
        assert_eq!(
            intents,
            vec![AgentsIntent::KeyringNotNow, AgentsIntent::AllowKeyring]
        );
    }
}
