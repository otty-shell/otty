use iced::widget::{Space, column, container, opaque, row, text, text_input};
use iced::{Element, Length, Theme, alignment};

use super::super::errors::secret_prompt_title;
use super::super::event::QuickLaunchIntent;
use super::super::state::SecretPromptState;
use super::super::types::SshSecretKind;
use super::wizard_form::editor_button;
use crate::i18n::{self, Key};
use crate::style::menu_panel_style;
use crate::theme::ThemeProps;

const DIALOG_WIDTH: f32 = 420.0;
const DIALOG_PADDING: f32 = 16.0;
const DIALOG_SPACING: f32 = 12.0;
const BUTTON_SPACING: f32 = 8.0;
const TITLE_SIZE: f32 = 15.0;
const TEXT_SIZE: f32 = 13.0;
const INPUT_PADDING_X: f32 = 8.0;
const INPUT_PADDING_Y: f32 = 6.0;
const BACKDROP_ALPHA: f32 = 0.5;

/// Props for the SSH secret dialog.
pub(crate) struct SecretPromptProps<'a> {
    pub(crate) prompt: &'a SecretPromptState,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the modal dialog that asks for an SSH key passphrase or password.
pub(crate) fn view(
    props: SecretPromptProps<'_>,
) -> Element<'_, QuickLaunchIntent, Theme, iced::Renderer> {
    let palette = props.theme.theme.iced_palette().clone();
    let request = props.prompt.request();

    let body = match request.kind() {
        SshSecretKind::KeyPassphrase => {
            i18n::secret_prompt_passphrase_body(request.target())
        },
        SshSecretKind::Password => {
            i18n::secret_prompt_password_body(request.target())
        },
    };

    let title_color = palette.bright_foreground;
    let title = text(secret_prompt_title(request.kind()))
        .size(TITLE_SIZE)
        .style(move |_| text::Style {
            color: Some(title_color),
        });

    let input = text_input("", props.prompt.value())
        .id(props.prompt.input_id().clone())
        .secure(true)
        .on_input(QuickLaunchIntent::SecretPromptChanged)
        .on_submit(QuickLaunchIntent::SecretPromptSubmit)
        .padding([INPUT_PADDING_Y, INPUT_PADDING_X])
        .size(TEXT_SIZE)
        .width(Length::Fill);

    let mut content = column![title, text(body).size(TEXT_SIZE), input]
        .spacing(DIALOG_SPACING)
        .width(Length::Fill);

    if request.is_retry() {
        let error = match request.kind() {
            SshSecretKind::KeyPassphrase => {
                i18n::t(Key::SecretPromptWrongPassphrase)
            },
            SshSecretKind::Password => {
                i18n::t(Key::SecretPromptPasswordRejected)
            },
        };
        let error_color = palette.red;
        content = content.push(text(error).size(TEXT_SIZE).style(move |_| {
            text::Style {
                color: Some(error_color),
            }
        }));
    }

    let buttons = row![
        Space::new().width(Length::Fill),
        editor_button(
            i18n::t(Key::ButtonCancel),
            QuickLaunchIntent::SecretPromptCancel,
            props.theme,
        ),
        editor_button(
            i18n::t(Key::ButtonConnect),
            QuickLaunchIntent::SecretPromptSubmit,
            props.theme,
        ),
    ]
    .spacing(BUTTON_SPACING)
    .width(Length::Fill);
    content = content.push(buttons);

    let dialog = container(content)
        .width(DIALOG_WIDTH)
        .padding(DIALOG_PADDING)
        .style(menu_panel_style(props.theme));

    let mut backdrop_color = palette.black;
    backdrop_color.a = BACKDROP_ALPHA;

    // `opaque` keeps clicks from reaching the UI behind the dialog.
    opaque(
        container(dialog)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(alignment::Horizontal::Center)
            .align_y(alignment::Vertical::Center)
            .style(move |_| container::Style {
                background: Some(backdrop_color.into()),
                ..Default::default()
            }),
    )
}
