use iced::widget::{column, text};
use iced::{Element, Theme};
use otty_ui_kit::components::button::{Button, ButtonKind};
use otty_ui_kit::components::dialog::Dialog;
use otty_ui_kit::components::text_field::TextField;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{fonts, spacing, typography};

use super::super::event::QuickLaunchIntent;
use super::super::state::SecretPromptState;
use super::super::types::SshSecretKind;
use crate::i18n::{self, Key};
use crate::theme::ThemeProps;

/// Props for the SSH secret dialog.
pub(crate) struct SecretPromptProps<'a> {
    pub(crate) prompt: &'a SecretPromptState,
    pub(crate) theme: ThemeProps<'a>,
}

/// Texts of the SSH secret dialog for one prompt.
struct SecretPromptTexts {
    title: String,
    hint: String,
    label: &'static str,
    error: Option<&'static str>,
}

impl SecretPromptTexts {
    fn new(prompt: &SecretPromptState) -> Self {
        let request = prompt.request();

        let (title, hint, label, retry_error) = match request.kind() {
            SshSecretKind::KeyPassphrase => (
                i18n::secret_prompt_passphrase_title(request.target()),
                i18n::secret_prompt_passphrase_hint(
                    prompt.launch_title(),
                    &prompt.account(),
                ),
                i18n::t(Key::SecretPromptPassphraseLabel),
                i18n::t(Key::SecretPromptWrongPassphrase),
            ),
            SshSecretKind::Password => (
                i18n::secret_prompt_password_title(request.target()),
                i18n::secret_prompt_password_hint(
                    prompt.launch_title(),
                    &prompt.account(),
                ),
                i18n::t(Key::SecretPromptPasswordLabel),
                i18n::t(Key::SecretPromptPasswordRejected),
            ),
        };

        Self {
            title,
            hint,
            label,
            error: request.is_retry().then_some(retry_error),
        }
    }
}

/// Render the modal dialog that asks for an SSH key passphrase or password.
pub(crate) fn view(
    props: SecretPromptProps<'_>,
) -> Element<'_, QuickLaunchIntent, Theme, iced::Renderer> {
    dialog(props.prompt, props.theme.theme.ui())
}

fn dialog<'a>(
    prompt: &'a SecretPromptState,
    palette: &UiPalette,
) -> Element<'a, QuickLaunchIntent, Theme, iced::Renderer> {
    let texts = SecretPromptTexts::new(prompt);

    let hint = text(texts.hint)
        .size(typography::SMALL.size())
        .font(fonts::UI)
        .color(palette.text_muted());

    let mut input = TextField::new(texts.label, prompt.value(), palette)
        .id(prompt.input_id().clone())
        .secure(true)
        .on_input(QuickLaunchIntent::SecretPromptChanged)
        .on_submit(QuickLaunchIntent::SecretPromptSubmit);
    if let Some(error) = texts.error {
        input = input.error(error);
    }

    let cancel = Button::new(i18n::t(Key::ButtonCancel), palette)
        .kind(ButtonKind::Ghost)
        .on_press(QuickLaunchIntent::SecretPromptCancel);
    let connect = Button::new(i18n::t(Key::ButtonConnect), palette)
        .kind(ButtonKind::Primary)
        .on_press(QuickLaunchIntent::SecretPromptSubmit);

    Dialog::new(texts.title, palette)
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
    use crate::widgets::quick_launch::types::{
        CommandSpec, QuickLaunch, SshCommand, SshSecretRequest,
    };

    fn ssh_launch() -> QuickLaunch {
        QuickLaunch {
            title: String::from("laptop"),
            spec: CommandSpec::Ssh {
                ssh: SshCommand {
                    host: String::from("192.168.1.20"),
                    port: 22,
                    user: Some(String::from("ubuntu")),
                    identity_file: None,
                    extra_args: Vec::new(),
                },
            },
            exit_behavior: ExitBehavior::default(),
        }
    }

    fn prompt(
        kind: SshSecretKind,
        target: &str,
        is_retry: bool,
    ) -> SecretPromptState {
        SecretPromptState::new(
            vec![String::from("laptop")],
            Box::new(ssh_launch()),
            SshSecretRequest::new(kind, target.to_string(), is_retry),
        )
    }

    #[test]
    fn given_key_passphrase_when_rendered_then_title_names_the_key_file() {
        let prompt =
            prompt(SshSecretKind::KeyPassphrase, "~/.ssh/id_ed25519", false);
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(dialog(&prompt, &palette));

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
        let mut ui = simulator(dialog(&prompt, &palette));

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
        let mut ui = simulator(dialog(&prompt, &palette));

        let error = i18n::t(Key::SecretPromptWrongPassphrase);
        assert!(ui.find(error).is_err());
    }

    #[test]
    fn given_retry_when_rendered_then_error_shows_under_the_field() {
        let palette = UiPalette::otty_dark();

        let passphrase = prompt(SshSecretKind::KeyPassphrase, "key", true);
        let mut ui = simulator(dialog(&passphrase, &palette));
        let error = i18n::t(Key::SecretPromptWrongPassphrase);
        assert!(ui.find(error).is_ok());

        let password = prompt(SshSecretKind::Password, "u@h:22", true);
        let mut ui = simulator(dialog(&password, &palette));
        let error = i18n::t(Key::SecretPromptPasswordRejected);
        assert!(ui.find(error).is_ok());
    }

    #[test]
    fn given_dialog_when_connect_clicked_then_submit_intent_is_sent() {
        let prompt = prompt(SshSecretKind::KeyPassphrase, "key", false);
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(dialog(&prompt, &palette));

        ui.click(i18n::t(Key::ButtonConnect))
            .expect("connect button should be visible");

        let messages: Vec<QuickLaunchIntent> = ui.into_messages().collect();
        assert!(matches!(
            messages.as_slice(),
            [QuickLaunchIntent::SecretPromptSubmit]
        ));
    }

    #[test]
    fn given_dialog_when_cancel_clicked_then_cancel_intent_is_sent() {
        let prompt = prompt(SshSecretKind::Password, "u@h:22", false);
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(dialog(&prompt, &palette));

        ui.click(i18n::t(Key::ButtonCancel))
            .expect("cancel button should be visible");

        let messages: Vec<QuickLaunchIntent> = ui.into_messages().collect();
        assert!(matches!(
            messages.as_slice(),
            [QuickLaunchIntent::SecretPromptCancel]
        ));
    }
}
