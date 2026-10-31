use iced::widget::{Space, column, container, opaque, row, text};
use iced::{Element, Length, Theme, alignment};
use otty_ui_kit::components::button::{Button, ButtonKind};
use otty_ui_kit::components::text_field::TextField;
use otty_ui_kit::{fonts, radius, spacing, typography};

use super::super::errors::secret_prompt_title;
use super::super::event::QuickLaunchIntent;
use super::super::state::SecretPromptState;
use super::super::types::SshSecretKind;
use crate::i18n::{self, Key};
use crate::theme::ThemeProps;

const DIALOG_WIDTH: f32 = 420.0;
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

    let title = text(secret_prompt_title(request.kind()))
        .size(typography::HEADING.size())
        .font(fonts::UI)
        .color(palette.text_strong());

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
    let buttons = row![Space::new().width(Length::Fill), cancel, connect]
        .spacing(spacing::SPACE_3)
        .width(Length::Fill);

    let content = column![title, Element::from(input), buttons]
        .spacing(spacing::SPACE_5)
        .width(Length::Fill);

    let dialog_background = palette.surface_overlay();
    let dialog_shadow = palette.shadow_popover();
    let dialog = container(content)
        .width(DIALOG_WIDTH)
        .padding(spacing::SPACE_5)
        .style(move |_| container::Style {
            background: Some(dialog_background.into()),
            border: iced::Border {
                radius: radius::RADIUS_LG.into(),
                ..Default::default()
            },
            shadow: dialog_shadow,
            ..Default::default()
        });

    let mut backdrop_color = props.theme.theme.iced_palette().black;
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
