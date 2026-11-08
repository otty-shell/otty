use iced::Element;
use iced::widget::{column, text};
use otty_ui_kit::components::button::{Button, ButtonKind};
use otty_ui_kit::components::dialog::Dialog;
use otty_ui_kit::components::text_field::TextField;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{fonts, spacing, typography};

use super::super::event::QuickLaunchIntent;
use super::super::model::SecretPromptViewModel;
use crate::i18n::{self, Key};
use crate::theme::ThemeProps;

/// Props for the SSH secret dialog.
pub(crate) struct SecretPromptProps<'a> {
    pub(crate) vm: SecretPromptViewModel<'a>,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the modal dialog that asks for an SSH key passphrase or password.
pub(crate) fn view(
    props: SecretPromptProps<'_>,
) -> Element<'_, QuickLaunchIntent> {
    dialog(props.vm, props.theme.theme.ui())
}

fn dialog<'a>(
    vm: SecretPromptViewModel<'a>,
    palette: &UiPalette,
) -> Element<'a, QuickLaunchIntent> {
    let hint = text(vm.hint)
        .size(typography::SMALL.size())
        .font(fonts::UI)
        .color(palette.text_muted());

    let mut input = TextField::new(vm.label, vm.value, palette)
        .id(vm.input_id.clone())
        .secure(true)
        .on_input(QuickLaunchIntent::SecretPromptChanged)
        .on_submit(QuickLaunchIntent::SecretPromptSubmit)
        .on_escape(QuickLaunchIntent::SecretPromptCancel);
    if let Some(error) = vm.error {
        input = input.error(error);
    }

    let cancel = Button::new(i18n::t(Key::ButtonCancel), palette)
        .kind(ButtonKind::Ghost)
        .on_press(QuickLaunchIntent::SecretPromptCancel);
    let connect = Button::new(i18n::t(Key::ButtonConnect), palette)
        .kind(ButtonKind::Primary)
        .on_press(QuickLaunchIntent::SecretPromptSubmit);

    Dialog::new(vm.title, palette)
        .body(column![hint, input].spacing(spacing::SPACE_4))
        .action(cancel)
        .action(connect)
        .into()
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;

    use super::*;
    use crate::domain::exit_behavior::ExitBehavior;
    use crate::domain::quick_launch::{CommandSpec, QuickLaunch, SshCommand};
    use crate::widgets::quick_launch::state::{
        QuickLaunchState, SecretPromptState,
    };
    use crate::widgets::quick_launch::types::{
        SshSecretKind, SshSecretRequest,
    };

    fn ssh_launch() -> QuickLaunch {
        QuickLaunch::new(
            String::from("laptop"),
            CommandSpec::Ssh {
                ssh: SshCommand::new(
                    String::from("192.168.1.20"),
                    22,
                    Some(String::from("ubuntu")),
                    None,
                    Vec::new(),
                ),
            },
            ExitBehavior::default(),
        )
    }

    /// The state of a launch that asks for a secret of `kind` for `target`.
    fn prompt(
        kind: SshSecretKind,
        target: &str,
        is_retry: bool,
    ) -> QuickLaunchState {
        let mut state = QuickLaunchState::default();
        state.open_secret_prompt(SecretPromptState::new(
            vec![String::from("laptop")],
            Box::new(ssh_launch()),
            SshSecretRequest::new(kind, target.to_string(), is_retry),
        ));

        state
    }

    /// The open dialog of `state`.
    fn vm(state: &QuickLaunchState) -> SecretPromptViewModel<'_> {
        SecretPromptViewModel::from_state(state)
            .expect("the prompt should be open")
    }

    #[test]
    fn given_key_passphrase_when_rendered_then_title_names_the_key_file() {
        let prompt =
            prompt(SshSecretKind::KeyPassphrase, "~/.ssh/id_ed25519", false);
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(dialog(vm(&prompt), &palette));

        let title = i18n::secret_prompt_passphrase_title("~/.ssh/id_ed25519");
        let hint = i18n::secret_prompt_passphrase_hint(
            "laptop",
            "ubuntu@192.168.1.20",
        );
        assert!(ui.find(title.as_str()).is_ok());
        assert!(ui.find(hint.as_str()).is_ok());
    }

    #[test]
    fn given_password_when_rendered_then_title_names_the_account() {
        let prompt =
            prompt(SshSecretKind::Password, "ubuntu@192.168.1.20:22", false);
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(dialog(vm(&prompt), &palette));

        let title =
            i18n::secret_prompt_password_title("ubuntu@192.168.1.20:22");
        let hint =
            i18n::secret_prompt_password_hint("laptop", "ubuntu@192.168.1.20");
        assert!(ui.find(title.as_str()).is_ok());
        assert!(ui.find(hint.as_str()).is_ok());
    }

    #[test]
    fn given_first_attempt_when_rendered_then_no_error_shows() {
        let prompt = prompt(SshSecretKind::KeyPassphrase, "key", false);
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(dialog(vm(&prompt), &palette));

        let error = i18n::t(Key::SecretPromptWrongPassphrase);
        assert!(ui.find(error).is_err());
    }

    #[test]
    fn given_retry_when_rendered_then_error_shows_under_the_field() {
        let palette = UiPalette::otty_dark();

        let passphrase = prompt(SshSecretKind::KeyPassphrase, "key", true);
        let mut ui = simulator(dialog(vm(&passphrase), &palette));
        let error = i18n::t(Key::SecretPromptWrongPassphrase);
        assert!(ui.find(error).is_ok());

        let password = prompt(SshSecretKind::Password, "u@h:22", true);
        let mut ui = simulator(dialog(vm(&password), &palette));
        let error = i18n::t(Key::SecretPromptPasswordRejected);
        assert!(ui.find(error).is_ok());
    }

    #[test]
    fn given_dialog_when_connect_clicked_then_submit_intent_is_sent() {
        let prompt = prompt(SshSecretKind::KeyPassphrase, "key", false);
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(dialog(vm(&prompt), &palette));

        ui.click(i18n::t(Key::ButtonConnect))
            .expect("connect button should be visible");

        let messages: Vec<QuickLaunchIntent> = ui.into_messages().collect();
        assert!(matches!(
            messages.as_slice(),
            [QuickLaunchIntent::SecretPromptSubmit]
        ));
    }

    /// Scenario 6: Esc in the focused field means Cancel. Without it the
    /// field would only lose its focus and the prompt would stay.
    #[test]
    fn esc_in_the_secret_field_cancels() {
        let prompt = prompt(SshSecretKind::Password, "u@h:22", false);
        let palette = UiPalette::otty_dark();
        let vm = vm(&prompt);
        let input_id = vm.input_id.clone();
        let mut ui = simulator(dialog(vm, &palette));

        ui.click(input_id).expect("the field should show");
        ui.tap_key(iced::keyboard::key::Named::Escape);

        let messages: Vec<QuickLaunchIntent> = ui.into_messages().collect();
        assert!(matches!(
            messages.as_slice(),
            [QuickLaunchIntent::SecretPromptCancel]
        ));
    }

    #[test]
    fn given_dialog_when_cancel_clicked_then_cancel_intent_is_sent() {
        let prompt = prompt(SshSecretKind::Password, "u@h:22", false);
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(dialog(vm(&prompt), &palette));

        ui.click(i18n::t(Key::ButtonCancel))
            .expect("cancel button should be visible");

        let messages: Vec<QuickLaunchIntent> = ui.into_messages().collect();
        assert!(matches!(
            messages.as_slice(),
            [QuickLaunchIntent::SecretPromptCancel]
        ));
    }
}
