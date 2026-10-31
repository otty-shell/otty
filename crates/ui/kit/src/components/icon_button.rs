//! `IconButton`: square icon-only button with a tooltip.

use iced::widget::{button, container, svg, text, tooltip};
use iced::{Border, Element, Length, Padding};

use crate::palette::UiPalette;
use crate::{fonts, radius, size, spacing, typography};

/// Icon edge length inside the button (`svg { width: 15px }`).
const ICON_SIZE: f32 = 15.0;

/// Size of an [`IconButton`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconButtonSize {
    /// 28px, the default.
    Medium,
    /// 24px, for dense headers.
    Small,
}

/// Icon color of an [`IconButton`] while idle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconTone {
    /// `text-muted`.
    Default,
    /// `accent`; only for the New session plus.
    Accent,
}

/// Square icon-only button. The label is shown as a tooltip.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use iced::widget::svg;
/// use otty_ui_kit::components::icon_button::IconButton;
/// use otty_ui_kit::palette::UiPalette;
///
/// #[derive(Clone)]
/// enum Message {
///     Close,
/// }
///
/// let palette = UiPalette::otty_dark();
/// let icon = svg::Handle::from_memory(&b"<svg/>"[..]);
/// let close: Element<'_, Message> = IconButton::new(icon, "Close", &palette)
///     .on_press(Message::Close)
///     .into();
/// ```
pub struct IconButton<Message> {
    icon: svg::Handle,
    label: String,
    palette: UiPalette,
    size: IconButtonSize,
    tone: IconTone,
    on_press: Option<Message>,
}

impl<Message> IconButton<Message> {
    /// Creates a medium default-tone button with `icon` and a tooltip
    /// `label`.
    pub fn new(
        icon: svg::Handle,
        label: impl Into<String>,
        palette: &UiPalette,
    ) -> Self {
        Self {
            icon,
            label: label.into(),
            palette: palette.clone(),
            size: IconButtonSize::Medium,
            tone: IconTone::Default,
            on_press: None,
        }
    }

    /// Sets the button size.
    pub fn size(mut self, size: IconButtonSize) -> Self {
        self.size = size;
        self
    }

    /// Sets the idle icon tone.
    pub fn tone(mut self, tone: IconTone) -> Self {
        self.tone = tone;
        self
    }

    /// Sets the message sent on press.
    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }
}

impl<'a, Message: Clone + 'a> From<IconButton<Message>>
    for Element<'a, Message>
{
    fn from(value: IconButton<Message>) -> Self {
        let edge = match value.size {
            IconButtonSize::Medium => size::CONTROL_MD,
            IconButtonSize::Small => size::CONTROL_SM,
        };
        let idle = match value.tone {
            IconTone::Default => value.palette.text_muted(),
            IconTone::Accent => value.palette.accent(),
        };
        let hover = value.palette.text_strong();
        let raised = value.palette.surface_raised();

        let icon = svg(value.icon).width(ICON_SIZE).height(ICON_SIZE).style(
            move |_, status| svg::Style {
                color: Some(match status {
                    svg::Status::Hovered => hover,
                    _ => idle,
                }),
            },
        );

        let control = button(
            container(icon)
                .width(Length::Fill)
                .height(Length::Fill)
                .center(Length::Fill),
        )
        .width(edge)
        .height(edge)
        .padding(0)
        .on_press_maybe(value.on_press)
        .style(move |_, status| button::Style {
            background: matches!(
                status,
                button::Status::Hovered | button::Status::Pressed
            )
            .then(|| raised.into()),
            border: Border {
                radius: radius::RADIUS_SM.into(),
                ..Border::default()
            },
            ..button::Style::default()
        });

        let hint = tooltip_hint(value.label, &value.palette);
        tooltip(control, hint, tooltip::Position::Bottom)
            .gap(spacing::SPACE_1)
            .into()
    }
}

/// Builds the tooltip bubble shown over icon buttons.
pub(super) fn tooltip_hint<'a, Message: 'a>(
    label: String,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let style = typography::SMALL;
    let background = palette.surface_overlay();
    let color = palette.text_body();
    let shadow = palette.shadow_popover();

    container(text(label).font(fonts::UI).size(style.size()).color(color))
        .padding(Padding::from([spacing::SPACE_1, spacing::SPACE_3]))
        .style(move |_| container::Style {
            background: Some(background.into()),
            border: Border {
                radius: radius::RADIUS_XS.into(),
                ..Border::default()
            },
            shadow,
            ..container::Style::default()
        })
        .into()
}
