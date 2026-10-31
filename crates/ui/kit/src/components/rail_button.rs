//! `RailButton`: icon button of the widget rail.

use iced::widget::{button, column, container, row, svg, text, tooltip};
use iced::{Border, Element, Length, Padding};

use crate::palette::UiPalette;
use crate::{fonts, radius, size, spacing, typography};

/// Icon edge length (`.ot-rail svg { width: 20px }`).
const ICON_SIZE: f32 = 20.0;
/// Width of the active line on the left edge (`inset 2px 0 0`).
const ACTIVE_LINE: f32 = 2.0;
/// Vertical margin of a separator (`margin: 6px 12px`).
const SEPARATOR_MARGIN_Y: f32 = 6.0;
/// Horizontal margin of a separator.
const SEPARATOR_MARGIN_X: f32 = 12.0;

/// Rail button. The label is shown as a tooltip. A button without a message
/// is not interactive.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use iced::widget::svg;
/// use otty_ui_kit::components::rail_button::RailButton;
/// use otty_ui_kit::palette::UiPalette;
///
/// #[derive(Clone)]
/// enum Message {
///     Open,
/// }
///
/// let palette = UiPalette::otty_dark();
/// let icon = svg::Handle::from_memory(&b"<svg/>"[..]);
/// let button: Element<'_, Message> =
///     RailButton::new(icon, "Sessions", &palette)
///         .active(true)
///         .on_press(Message::Open)
///         .into();
/// ```
pub struct RailButton<Message> {
    icon: svg::Handle,
    label: String,
    palette: UiPalette,
    is_active: bool,
    on_press: Option<Message>,
}

impl<Message> RailButton<Message> {
    /// Creates an idle button with `icon` and a tooltip `label`.
    pub fn new(
        icon: svg::Handle,
        label: impl Into<String>,
        palette: &UiPalette,
    ) -> Self {
        Self {
            icon,
            label: label.into(),
            palette: palette.clone(),
            is_active: false,
            on_press: None,
        }
    }

    /// Marks the widget as open: accent icon and a line on the left edge.
    pub fn active(mut self, is_active: bool) -> Self {
        self.is_active = is_active;
        self
    }

    /// Sets the message sent on press.
    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }
}

impl<'a, Message: Clone + 'a> From<RailButton<Message>>
    for Element<'a, Message>
{
    fn from(value: RailButton<Message>) -> Self {
        let palette = value.palette;
        let idle = if value.is_active {
            palette.accent()
        } else {
            palette.text_faint()
        };
        let hover = palette.accent();
        let line = if value.is_active {
            palette.accent()
        } else {
            iced::Color::TRANSPARENT
        };

        let icon = svg(value.icon).width(ICON_SIZE).height(ICON_SIZE).style(
            move |_, status| svg::Style {
                color: Some(match status {
                    svg::Status::Hovered => hover,
                    _ => idle,
                }),
            },
        );
        let edge = container("").width(ACTIVE_LINE).height(Length::Fill).style(
            move |_| container::Style {
                background: Some(line.into()),
                ..container::Style::default()
            },
        );
        let content = row![
            edge,
            container(icon)
                .width(Length::Fill)
                .height(Length::Fill)
                .center(Length::Fill),
        ];

        let control = button(content)
            .width(size::RAIL)
            .height(size::RAIL_BUTTON)
            .padding(0)
            .on_press_maybe(value.on_press)
            .style(|_, _| button::Style::default());

        let style = typography::SMALL;
        let background = palette.surface_overlay();
        let color = palette.text_body();
        let shadow = palette.shadow_popover();
        let hint = container(
            text(value.label)
                .font(fonts::UI)
                .size(style.size())
                .color(color),
        )
        .padding(Padding::from([spacing::SPACE_1, spacing::SPACE_3]))
        .style(move |_| container::Style {
            background: Some(background.into()),
            border: Border {
                radius: radius::RADIUS_XS.into(),
                ..Border::default()
            },
            shadow,
            ..container::Style::default()
        });

        tooltip(control, hint, tooltip::Position::Right)
            .gap(spacing::SPACE_1)
            .into()
    }
}

/// Returns the 1px line between groups of rail buttons.
pub fn rail_separator<'a, Message: 'a>(
    palette: &UiPalette,
) -> Element<'a, Message> {
    let line = palette.line();

    column![container("").width(Length::Fill).height(1).style(move |_| {
        container::Style {
            background: Some(line.into()),
            ..container::Style::default()
        }
    })]
    .padding(Padding::from([SEPARATOR_MARGIN_Y, SEPARATOR_MARGIN_X]))
    .into()
}
