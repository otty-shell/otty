use iced::widget::button::Status as ButtonStatus;
use iced::widget::{button, text};
use iced::{Element, Length, alignment};

use crate::app::theme::ThemeProps;

/// UI events emitted by a menu item.
#[derive(Debug, Clone)]
pub(crate) enum MenuItemEvent {
    Pressed,
}

/// Layout metrics for a menu item.
#[derive(Debug, Clone, Copy)]
struct MenuItemMetrics {
    height: f32,
    font_size: f32,
    horizontal_padding: f32,
}

impl Default for MenuItemMetrics {
    fn default() -> Self {
        Self {
            height: 24.0,
            font_size: 13.0,
            horizontal_padding: 10.0,
        }
    }
}

/// Props for rendering a menu item.
#[derive(Debug, Clone, Copy)]
pub(crate) struct MenuItemProps<'a> {
    pub(crate) label: &'a str,
    pub(crate) theme: ThemeProps<'a>,
}

/// A single menu row used in context menus.
pub(crate) struct MenuItem<'a> {
    props: MenuItemProps<'a>,
    metrics: MenuItemMetrics,
}

impl<'a> MenuItem<'a> {
    pub fn new(props: MenuItemProps<'a>) -> Self {
        Self {
            props,
            metrics: MenuItemMetrics::default(),
        }
    }

    pub fn view(self) -> Element<'a, MenuItemEvent> {
        let palette = self.props.theme.theme.iced_palette();

        let label = text(self.props.label)
            .size(self.metrics.font_size)
            .width(Length::Fill)
            .align_x(alignment::Horizontal::Left)
            .align_y(alignment::Vertical::Center);

        button(label)
            .padding([1.0, self.metrics.horizontal_padding])
            .width(Length::Fill)
            .height(Length::Fixed(self.metrics.height))
            .style(move |_, status| menu_button_style(palette, status))
            .on_press(MenuItemEvent::Pressed)
            .into()
    }
}

fn menu_button_style(
    palette: &crate::app::theme::IcedColorPalette,
    status: ButtonStatus,
) -> button::Style {
    let background = match status {
        ButtonStatus::Hovered | ButtonStatus::Pressed => {
            Some(palette.dim_blue.into())
        },
        _ => Some(palette.overlay.into()),
    };

    let text_color = match status {
        ButtonStatus::Hovered | ButtonStatus::Pressed => palette.dim_black,
        _ => palette.foreground,
    };

    button::Style {
        background,
        text_color,
        border: iced::Border {
            width: 0.0,
            ..Default::default()
        },
        ..Default::default()
    }
}
