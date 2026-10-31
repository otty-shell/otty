//! `Button`: text button for forms, toolbars and panels.

use iced::widget::{button, row, text};
use iced::{Alignment, Border, Element, Length, Padding};

use super::{faded, kbd};
use crate::palette::UiPalette;
use crate::{fonts, radius, size, spacing};

/// Label size of a form button (`font-size: 13px`).
const LABEL_SIZE: f32 = 13.0;
/// Label size of a small button (`font-size: 12.5px`).
const LABEL_SIZE_SMALL: f32 = 12.5;
/// Horizontal padding of a form button (`padding: 0 14px`).
const PADDING_X: f32 = 14.0;
/// Horizontal padding of a small button (`padding: 0 10px`).
const PADDING_X_SMALL: f32 = 10.0;

/// Visual kind of a [`Button`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonKind {
    /// The one main action of a screen.
    Primary,
    /// Other actions next to the primary one.
    Secondary,
    /// Cancel and other low-stakes actions.
    Ghost,
    /// Actions that delete or change a protected target.
    Danger,
}

/// Height of a [`Button`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonSize {
    /// 34px, used in forms.
    Form,
    /// 28px, used in toolbars and inside blocks.
    Small,
}

/// Text button. A button without a message is shown disabled.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use otty_ui_kit::components::button::{Button, ButtonKind};
/// use otty_ui_kit::palette::UiPalette;
///
/// #[derive(Clone)]
/// enum Message {
///     Save,
/// }
///
/// let palette = UiPalette::otty_dark();
/// let save: Element<'_, Message> = Button::new("Save", &palette)
///     .kind(ButtonKind::Primary)
///     .shortcut("Ctrl S")
///     .on_press(Message::Save)
///     .into();
/// ```
pub struct Button<Message> {
    label: String,
    palette: UiPalette,
    kind: ButtonKind,
    size: ButtonSize,
    shortcut: Option<String>,
    on_press: Option<Message>,
}

impl<Message> Button<Message> {
    /// Creates a secondary, form-size button with `label`.
    pub fn new(label: impl Into<String>, palette: &UiPalette) -> Self {
        Self {
            label: label.into(),
            palette: palette.clone(),
            kind: ButtonKind::Secondary,
            size: ButtonSize::Form,
            shortcut: None,
            on_press: None,
        }
    }

    /// Sets the visual kind.
    pub fn kind(mut self, kind: ButtonKind) -> Self {
        self.kind = kind;
        self
    }

    /// Sets the height.
    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    /// Shows a shortcut hint (for example `Ctrl S`) inside the button.
    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    /// Sets the message sent on press and enables the button.
    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }
}

impl<'a, Message: Clone + 'a> From<Button<Message>> for Element<'a, Message> {
    fn from(value: Button<Message>) -> Self {
        let (height, label_size, padding_x) = match value.size {
            ButtonSize::Form => (size::CONTROL_LG, LABEL_SIZE, PADDING_X),
            ButtonSize::Small => {
                (size::CONTROL_MD, LABEL_SIZE_SMALL, PADDING_X_SMALL)
            },
        };
        let font = if value.kind == ButtonKind::Primary {
            fonts::UI_SEMIBOLD
        } else {
            fonts::UI
        };

        let mut content = row![text(value.label).size(label_size).font(font)]
            .spacing(spacing::SPACE_2)
            .height(Length::Fill)
            .align_y(Alignment::Center);
        if let Some(shortcut) = value.shortcut {
            content = content.push(kbd::kbd(shortcut, &value.palette));
        }

        let palette = value.palette;
        let kind = value.kind;
        button(content)
            .height(height)
            .padding(Padding::from([0.0, padding_x]))
            .on_press_maybe(value.on_press)
            .style(move |_, status| style(&palette, kind, status))
            .into()
    }
}

/// Resolves the button style of `kind` in `status`.
fn style(
    palette: &UiPalette,
    kind: ButtonKind,
    status: button::Status,
) -> button::Style {
    let hovered =
        matches!(status, button::Status::Hovered | button::Status::Pressed);

    let (background, text_color) = match kind {
        ButtonKind::Primary => (
            Some(if hovered {
                palette.accent_hover()
            } else {
                palette.accent()
            }),
            palette.on_accent(),
        ),
        ButtonKind::Secondary => (
            Some(if hovered {
                palette.surface_hover()
            } else {
                palette.surface_overlay()
            }),
            palette.text_strong(),
        ),
        ButtonKind::Ghost => (
            hovered.then(|| palette.surface_raised()),
            if hovered {
                palette.text_strong()
            } else {
                palette.text_muted()
            },
        ),
        ButtonKind::Danger => {
            (Some(palette.surface_danger()), palette.text_danger())
        },
    };

    let disabled = status == button::Status::Disabled;
    button::Style {
        background: background
            .map(|color| if disabled { faded(color) } else { color })
            .map(Into::into),
        text_color: if disabled {
            faded(text_color)
        } else {
            text_color
        },
        border: Border {
            radius: radius::RADIUS_SM.into(),
            ..Border::default()
        },
        ..button::Style::default()
    }
}
