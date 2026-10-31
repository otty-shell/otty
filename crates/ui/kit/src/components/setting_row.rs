//! `SettingRow`: name and description on the left, a control on the right.

use iced::widget::{column, container, row, text};
use iced::{Alignment, Element, Length, Padding};

use crate::palette::UiPalette;
use crate::{fonts, spacing, typography};

/// Width of the control column (`width: 280px`).
const CONTROL_WIDTH: f32 = 280.0;
/// Vertical padding of the row (`padding: 14px 0`).
const PADDING_Y: f32 = 14.0;
/// Gap between name and description (`gap: 2px`).
const TEXT_GAP: f32 = 2.0;

/// Row of a settings page with a 1px line under it.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use otty_ui_kit::components::setting_row::SettingRow;
/// use otty_ui_kit::components::switch::Switch;
/// use otty_ui_kit::palette::UiPalette;
///
/// #[derive(Clone)]
/// enum Message {
///     Toggled(bool),
/// }
///
/// let palette = UiPalette::otty_dark();
/// let setting: Element<'_, Message> =
///     SettingRow::new("Command blocks", &palette)
///         .description("Group output by command.")
///         .control(Switch::new(true, &palette).on_toggle(Message::Toggled))
///         .into();
/// ```
pub struct SettingRow<'a, Message> {
    title: String,
    palette: UiPalette,
    description: Option<String>,
    control: Option<Element<'a, Message>>,
}

impl<'a, Message> SettingRow<'a, Message> {
    /// Creates a row with a name only.
    pub fn new(title: impl Into<String>, palette: &UiPalette) -> Self {
        Self {
            title: title.into(),
            palette: palette.clone(),
            description: None,
            control: None,
        }
    }

    /// Sets the line under the name that says what the setting does.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the control shown in the 280px column on the right, aligned to
    /// its right edge.
    pub fn control(mut self, control: impl Into<Element<'a, Message>>) -> Self {
        self.control = Some(control.into());
        self
    }
}

impl<'a, Message: 'a> From<SettingRow<'a, Message>> for Element<'a, Message> {
    fn from(value: SettingRow<'a, Message>) -> Self {
        let palette = value.palette;
        let body = typography::BODY;
        let small = typography::SMALL;

        let mut texts = column![
            text(value.title)
                .size(body.size())
                .font(fonts::UI)
                .color(palette.text_strong())
        ]
        .spacing(TEXT_GAP)
        .width(Length::Fill);
        if let Some(description) = value.description {
            texts = texts.push(
                text(description)
                    .size(small.size())
                    .font(fonts::UI)
                    .color(palette.text_muted()),
            );
        }

        let mut line = row![texts]
            .spacing(spacing::SPACE_6)
            .align_y(Alignment::Center);
        if let Some(control) = value.control {
            line = line.push(
                container(control)
                    .width(CONTROL_WIDTH)
                    .align_x(Alignment::End),
            );
        }

        let rule = palette.line();
        column![
            container(line)
                .padding(Padding::from([PADDING_Y, 0.0]))
                .width(Length::Fill),
            container("").width(Length::Fill).height(1).style(move |_| {
                container::Style {
                    background: Some(rule.into()),
                    ..container::Style::default()
                }
            }),
        ]
        .into()
    }
}
