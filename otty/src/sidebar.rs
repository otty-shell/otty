use iced::alignment;
use iced::widget::{Space, button, column, container, scrollable, svg};
use iced::{Element, Length, Theme};

use crate::main_window::{Event, SidebarView};

const SIDEBAR_WIDTH: f32 = 48.0;
const ICON_SIZE: f32 = 32.0;

/// Renders the vertical sidebar with scrollable and fixed sections.
pub fn view_sidebar<'a>(
    active_view: SidebarView,
) -> Element<'a, Event, Theme, iced::Renderer> {
    let scrollable_icons = view_scrollable_icons();
    let fixed_area = view_fixed_area(active_view);

    let content = column![
        scrollable_icons,
        Space::with_height(Length::Fill),
        fixed_area
    ]
    .width(Length::Fill)
    .height(Length::Fill);

    container(content)
        .width(Length::Fixed(SIDEBAR_WIDTH))
        .height(Length::Fill)
        .style(sidebar_container_style)
        .into()
}

fn view_scrollable_icons<'a>() -> Element<'a, Event, Theme, iced::Renderer> {
    let column = column![].spacing(8);

    let scroll = scrollable::Scrollable::with_direction(
        column,
        scrollable::Direction::Vertical(
            scrollable::Scrollbar::new()
                .width(0)
                .scroller_width(0)
                .margin(0),
        ),
    )
    .width(Length::Fill)
    .height(Length::Fill);

    scroll.into()
}

fn view_fixed_area<'a>(
    active_view: SidebarView,
) -> Element<'a, Event, Theme, iced::Renderer> {
    let is_active = matches!(active_view, SidebarView::Settings);

    let handle = svg::Handle::from_memory(include_bytes!(
        "../../assets/svg/settings.svg"
    ));
    let icon = svg::Svg::new(handle)
        .width(Length::Fixed(ICON_SIZE))
        .height(Length::Fixed(ICON_SIZE))
        .style(move |theme, status| {
            sidebar_icon_style(theme, status, is_active)
        });

    let icon_container = container(icon)
        .width(Length::Fill)
        .height(Length::Fixed(ICON_SIZE))
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center);

    button(icon_container)
        .on_press(Event::SidebarViewChanged(SidebarView::Settings))
        .padding([8, 0])
        .width(Length::Fill)
        .style(move |theme, _| sidebar_button_style(theme, is_active))
        .into()
}

fn sidebar_container_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();

    container::Style {
        background: Some(palette.background.weak.color.into()),
        text_color: Some(palette.background.weak.text),
        ..container::Style::default()
    }
}

fn sidebar_button_style(theme: &Theme, is_active: bool) -> button::Style {
    let palette = theme.extended_palette();

    if is_active {
        button::Style {
            background: Some(palette.primary.weak.color.into()),
            text_color: palette.primary.weak.text,
            ..button::Style::default()
        }
    } else {
        button::Style {
            background: Some(palette.background.weak.color.into()),
            text_color: palette.background.weak.text,
            ..button::Style::default()
        }
    }
}

fn sidebar_icon_style(
    theme: &Theme,
    status: svg::Status,
    is_active: bool,
) -> svg::Style {
    let palette = theme.extended_palette();

    let base = palette.background.strong.text;
    let highlight = palette.background.weak.text;

    let color = if is_active || matches!(status, svg::Status::Hovered) {
        highlight
    } else {
        base
    };

    svg::Style {
        color: Some(color),
        ..svg::Style::default()
    }
}
