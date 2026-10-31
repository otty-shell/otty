//! `Switch`: on or off setting that applies right away.

use iced::widget::{button, container, row, text};
use iced::{Alignment, Border, Element, Length, Padding, border};

use crate::palette::UiPalette;
use crate::{fonts, spacing, typography};

/// Track width (`width: 34px`).
const TRACK_WIDTH: f32 = 34.0;
/// Track height (`height: 20px`).
const TRACK_HEIGHT: f32 = 20.0;
/// Knob diameter (`width: 14px`).
const KNOB_SIZE: f32 = 14.0;
/// Gap between track edge and knob (`top: 3px; left: 3px`).
const KNOB_INSET: f32 = 3.0;

/// Switch with an optional label. Without a message it is not interactive.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use otty_ui_kit::components::switch::Switch;
/// use otty_ui_kit::palette::UiPalette;
///
/// #[derive(Clone)]
/// enum Message {
///     Toggled(bool),
/// }
///
/// let palette = UiPalette::otty_dark();
/// let switch: Element<'_, Message> = Switch::new(true, &palette)
///     .label("Show command blocks")
///     .on_toggle(Message::Toggled)
///     .into();
/// ```
pub struct Switch<'a, Message> {
    is_on: bool,
    palette: UiPalette,
    label: Option<String>,
    on_toggle: Option<Box<dyn Fn(bool) -> Message + 'a>>,
}

impl<'a, Message> Switch<'a, Message> {
    /// Creates a switch in the given state.
    pub fn new(is_on: bool, palette: &UiPalette) -> Self {
        Self {
            is_on,
            palette: palette.clone(),
            label: None,
            on_toggle: None,
        }
    }

    /// Sets the label that says what is on.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets the function that maps the new state to a message.
    pub fn on_toggle(
        mut self,
        on_toggle: impl Fn(bool) -> Message + 'a,
    ) -> Self {
        self.on_toggle = Some(Box::new(on_toggle));
        self
    }
}

impl<'a, Message: Clone + 'a> From<Switch<'a, Message>>
    for Element<'a, Message>
{
    fn from(value: Switch<'a, Message>) -> Self {
        let palette = value.palette;
        let is_on = value.is_on;

        let track_color = if is_on {
            palette.accent()
        } else {
            palette.surface_hover()
        };
        let knob_color = if is_on {
            palette.on_accent()
        } else {
            palette.text_muted()
        };
        let knob =
            container("")
                .width(KNOB_SIZE)
                .height(KNOB_SIZE)
                .style(move |_| container::Style {
                    background: Some(knob_color.into()),
                    border: Border {
                        radius: border::radius(KNOB_SIZE / 2.0),
                        ..Border::default()
                    },
                    ..container::Style::default()
                });
        let track = container(knob)
            .width(TRACK_WIDTH)
            .height(TRACK_HEIGHT)
            .padding(KNOB_INSET)
            .align_x(if is_on {
                Alignment::End
            } else {
                Alignment::Start
            })
            .align_y(Alignment::Center)
            .style(move |_| container::Style {
                background: Some(track_color.into()),
                border: Border {
                    radius: border::radius(TRACK_HEIGHT / 2.0),
                    ..Border::default()
                },
                ..container::Style::default()
            });

        let mut content = row![track]
            .spacing(spacing::SPACE_3)
            .align_y(Alignment::Center);
        if let Some(label) = value.label {
            let style = typography::BODY;
            content = content.push(
                text(label)
                    .font(fonts::UI)
                    .size(style.size())
                    .color(palette.text_body()),
            );
        }

        let message = value.on_toggle.map(|on_toggle| on_toggle(!is_on));
        button(content)
            .padding(Padding::ZERO)
            .width(Length::Shrink)
            .on_press_maybe(message)
            .style(|_, _| button::Style::default())
            .into()
    }
}
