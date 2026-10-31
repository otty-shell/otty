//! `SplitButton`: primary icon button with a narrow chevron for a menu.

use iced::widget::{button, container, row, svg, tooltip};
use iced::{Alignment, Border, Element, Length};

use super::glyph;
use super::icon_button::{IconButton, IconButtonSize, IconTone, tooltip_hint};
use crate::palette::UiPalette;
use crate::{radius, size, spacing};

/// Edge of the chevron (`width: 11px; height: 11px`).
const CHEVRON_SIZE: f32 = 11.0;

/// Primary icon button and a chevron button side by side in one
/// `radius-sm` frame. The frame shows `surface-raised` while the menu opened
/// by the chevron is open.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use iced::widget::svg;
/// use otty_ui_kit::components::split_button::SplitButton;
/// use otty_ui_kit::palette::UiPalette;
///
/// #[derive(Clone)]
/// enum Message {
///     NewSession,
///     ShowMenu,
/// }
///
/// let palette = UiPalette::otty_dark();
/// let plus = svg::Handle::from_memory(&b"<svg/>"[..]);
/// let split: Element<'_, Message> = SplitButton::new(
///     plus,
///     "New terminal",
///     "More ways to start a session",
///     &palette,
/// )
/// .on_press(Message::NewSession)
/// .on_menu(Message::ShowMenu)
/// .into();
/// ```
pub struct SplitButton<Message> {
    icon: svg::Handle,
    label: String,
    menu_label: String,
    palette: UiPalette,
    is_menu_open: bool,
    on_press: Option<Message>,
    on_menu: Option<Message>,
}

impl<Message> SplitButton<Message> {
    /// Creates a split button; `label` and `menu_label` are the tooltips of
    /// the primary and the chevron part.
    pub fn new(
        icon: svg::Handle,
        label: impl Into<String>,
        menu_label: impl Into<String>,
        palette: &UiPalette,
    ) -> Self {
        Self {
            icon,
            label: label.into(),
            menu_label: menu_label.into(),
            palette: palette.clone(),
            is_menu_open: false,
            on_press: None,
            on_menu: None,
        }
    }

    /// Shows the frame background while the menu is open.
    pub fn menu_open(mut self, is_menu_open: bool) -> Self {
        self.is_menu_open = is_menu_open;
        self
    }

    /// Sets the message sent when the primary part is pressed.
    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }

    /// Sets the message sent when the chevron is pressed.
    pub fn on_menu(mut self, message: Message) -> Self {
        self.on_menu = Some(message);
        self
    }
}

impl<'a, Message: Clone + 'a> From<SplitButton<Message>>
    for Element<'a, Message>
{
    fn from(value: SplitButton<Message>) -> Self {
        let palette = value.palette;
        let frame = palette.surface_raised();
        let is_menu_open = value.is_menu_open;

        let mut primary = IconButton::new(value.icon, value.label, &palette)
            .size(IconButtonSize::Small)
            .tone(IconTone::Accent);
        if let Some(message) = value.on_press {
            primary = primary.on_press(message);
        }

        let muted = palette.text_muted();
        let strong = palette.text_strong();
        let raised = palette.surface_raised();
        let chevron = button(
            container(
                svg(glyph::caret(true))
                    .width(CHEVRON_SIZE)
                    .height(CHEVRON_SIZE)
                    .style(move |_, status| svg::Style {
                        color: Some(match status {
                            svg::Status::Hovered => strong,
                            _ => muted,
                        }),
                    }),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .center(Length::Fill),
        )
        .width(size::SPLIT_CHEVRON)
        .height(size::CONTROL_SM)
        .padding(0)
        .on_press_maybe(value.on_menu)
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
        let chevron = tooltip(
            chevron,
            tooltip_hint(value.menu_label, &palette),
            tooltip::Position::Bottom,
        )
        .gap(spacing::SPACE_1);

        container(row![primary, chevron].align_y(Alignment::Center))
            .style(move |_| container::Style {
                background: is_menu_open.then(|| frame.into()),
                border: Border {
                    radius: radius::RADIUS_SM.into(),
                    ..Border::default()
                },
                ..container::Style::default()
            })
            .into()
    }
}

#[cfg(test)]
mod tests {
    use iced::Point;
    use iced_test::simulator;

    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Message {
        New,
        Menu,
    }

    fn click_at(x: f32) -> Vec<Message> {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(
            SplitButton::new(glyph::plus(), "New", "More", &palette)
                .on_press(Message::New)
                .on_menu(Message::Menu),
        );

        ui.point_at(Point::new(x, size::CONTROL_SM / 2.0));
        let _ = ui.simulate(iced_test::simulator::click());

        ui.into_messages().collect()
    }

    #[test]
    fn given_primary_part_when_pressed_then_only_new_is_sent() {
        assert_eq!(click_at(size::CONTROL_SM / 2.0), vec![Message::New]);
    }

    #[test]
    fn given_chevron_part_when_pressed_then_only_menu_is_sent() {
        let x = size::CONTROL_SM + size::SPLIT_CHEVRON / 2.0;

        assert_eq!(click_at(x), vec![Message::Menu]);
    }
}
