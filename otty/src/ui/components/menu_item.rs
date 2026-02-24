use iced::widget::button::Status as ButtonStatus;
use iced::widget::{button, text};
use iced::{Element, Length, alignment};

use crate::theme::{IcedColorPalette, ThemeProps};

const MENU_ITEM_HEIGHT: f32 = 24.0;
const MENU_ITEM_FONT_SIZE: f32 = 13.0;
const MENU_ITEM_HORIZONTAL_PADDING: f32 = 10.0;
const MENU_ITEM_VERTICAL_PADDING: f32 = 1.0;

/// UI events emitted by a menu item.
#[derive(Debug, Clone)]
pub(crate) enum MenuItemEvent {
    Pressed,
}

/// Props for rendering a menu item.
#[derive(Debug, Clone, Copy)]
pub(crate) struct MenuItemProps<'a> {
    pub(crate) label: &'a str,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render a single menu row used in context menus.
pub(crate) fn view<'a>(props: MenuItemProps<'a>) -> Element<'a, MenuItemEvent> {
    let palette = props.theme.theme.iced_palette();

    let label = text(props.label)
        .size(MENU_ITEM_FONT_SIZE)
        .width(Length::Fill)
        .align_x(alignment::Horizontal::Left)
        .align_y(alignment::Vertical::Center);

    button(label)
        .padding([MENU_ITEM_VERTICAL_PADDING, MENU_ITEM_HORIZONTAL_PADDING])
        .width(Length::Fill)
        .height(Length::Fixed(MENU_ITEM_HEIGHT))
        .style(move |_, status| menu_button_style(palette, status))
        .on_press(MenuItemEvent::Pressed)
        .into()
}

fn menu_button_style(
    palette: &IcedColorPalette,
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

#[cfg(test)]
mod tests {
    use iced::Background;
    use iced::widget::button::Status as ButtonStatus;

    use super::menu_button_style;

    #[test]
    fn given_hovered_status_when_building_style_then_uses_hover_colors() {
        let theme = crate::theme::AppTheme::default();
        let palette = theme.iced_palette();

        let style = menu_button_style(palette, ButtonStatus::Hovered);

        assert_eq!(style.text_color, palette.dim_black);
        assert_eq!(style.background, Some(Background::Color(palette.dim_blue)),);
    }

    #[test]
    fn given_active_status_when_building_style_then_uses_idle_colors() {
        let theme = crate::theme::AppTheme::default();
        let palette = theme.iced_palette();

        let style = menu_button_style(palette, ButtonStatus::Active);

        assert_eq!(style.text_color, palette.foreground);
        assert_eq!(style.background, Some(Background::Color(palette.overlay)),);
    }
}
