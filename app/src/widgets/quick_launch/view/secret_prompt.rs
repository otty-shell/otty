use iced::{Element, Theme};
use otty_ui_kit::components::button::{Button, ButtonKind};
use otty_ui_kit::components::dialog::Dialog;
use otty_ui_kit::components::text_field::TextField;

use super::super::errors::secret_prompt_title;
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

/// Render the modal dialog that asks for an SSH key passphrase or password.
pub(crate) fn view(
    props: SecretPromptProps<'_>,
) -> Element<'_, QuickLaunchIntent, Theme, iced::Renderer> {
    let palette = props.theme.theme.ui();
    let request = props.prompt.request();

    let body = match request.kind() {
        SshSecretKind::KeyPassphrase => {
            i18n::secret_prompt_passphrase_body(request.target())
        },
        SshSecretKind::Password => {
            i18n::secret_prompt_password_body(request.target())
        },
    };

    let mut input = TextField::new(body, props.prompt.value(), palette)
        .id(props.prompt.input_id().clone())
        .secure(true)
        .on_input(QuickLaunchIntent::SecretPromptChanged)
        .on_submit(QuickLaunchIntent::SecretPromptSubmit);

    if request.is_retry() {
        let error = match request.kind() {
            SshSecretKind::KeyPassphrase => {
                i18n::t(Key::SecretPromptWrongPassphrase)
            },
            SshSecretKind::Password => {
                i18n::t(Key::SecretPromptPasswordRejected)
            },
        };
        input = input.error(error);
    }

    let cancel = Button::new(i18n::t(Key::ButtonCancel), palette)
        .kind(ButtonKind::Ghost)
        .on_press(QuickLaunchIntent::SecretPromptCancel);
    let connect = Button::new(i18n::t(Key::ButtonConnect), palette)
        .kind(ButtonKind::Primary)
        .on_press(QuickLaunchIntent::SecretPromptSubmit);

    Dialog::new(secret_prompt_title(request.kind()), palette)
        .body(input)
        .action(cancel)
        .action(connect)
        .into()
}
