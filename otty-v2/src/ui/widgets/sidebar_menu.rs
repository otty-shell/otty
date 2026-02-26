use iced::widget::{
    Space, button, column, container, pane_grid, row, scrollable, svg,
};
use iced::{Border, Element, Length, alignment};

use crate::icons;
use crate::theme::ThemeProps;

const MENU_BUTTON_SIZE: f32 = 44.0;
const MENU_ICON_SIZE: f32 = 20.0;
const MENU_BUTTON_PADDING: f32 = 8.0;
const MENU_META_SPACING: f32 = 0.0;
const ACTIVE_BORDER_WIDTH: f32 = 2.0;

/// Sidebar destinations displayed in the menu rail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SidebarMenuItem {
    Terminal,
    Explorer,
}

/// UI events emitted by the sidebar menu.
#[derive(Debug, Clone)]
pub(crate) enum SidebarMenuEvent {
    SelectItem(SidebarMenuItem),
    OpenSettings,
    ToggleWorkspace,
    Resized(pane_grid::ResizeEvent),
}

/// Props for rendering the sidebar menu rail.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SidebarMenuProps<'a> {
    pub(crate) active_item: SidebarMenuItem,
    pub(crate) workspace_open: bool,
    pub(crate) menu_width: f32,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the sidebar menu rail with scrollable primary items and fixed meta.
pub(crate) fn view<'a>(
    props: SidebarMenuProps<'a>,
) -> Element<'a, SidebarMenuEvent> {
    let palette = props.theme.theme.iced_palette();

    let terminal_button = sidebar_button(
        icons::SIDEBAR_TERMINAL,
        props.active_item == SidebarMenuItem::Terminal,
        props.theme,
        SidebarMenuEvent::SelectItem(SidebarMenuItem::Terminal),
    );

    let explorer_button = sidebar_button(
        icons::SIDEBAR_EXPLORER,
        props.active_item == SidebarMenuItem::Explorer,
        props.theme,
        SidebarMenuEvent::SelectItem(SidebarMenuItem::Explorer),
    );

    let main_menu = column![terminal_button, explorer_button]
        .spacing(0)
        .width(Length::Fill);

    let main_scroll = scrollable::Scrollable::with_direction(
        main_menu,
        scrollable::Direction::Vertical(
            scrollable::Scrollbar::new()
                .width(0)
                .scroller_width(0)
                .margin(0),
        ),
    )
    .width(Length::Fill)
    .height(Length::Fill);

    let settings_button = sidebar_button(
        icons::SIDEBAR_SETTINGS,
        false,
        props.theme,
        SidebarMenuEvent::OpenSettings,
    );

    let toggle_icon = if props.workspace_open {
        icons::SIDEBAR_COLLAPSE
    } else {
        icons::SIDEBAR_EXPAND
    };

    let toggle_button = sidebar_button(
        toggle_icon,
        false,
        props.theme,
        SidebarMenuEvent::ToggleWorkspace,
    );

    let meta_menu =
        column![settings_button, toggle_button].spacing(MENU_META_SPACING);

    let content = column![main_scroll, meta_menu]
        .width(Length::Fill)
        .height(Length::Fill);

    container(content)
        .width(Length::Fixed(props.menu_width))
        .height(Length::Fill)
        .style(move |_| iced::widget::container::Style {
            background: Some(palette.dim_black.into()),
            ..Default::default()
        })
        .into()
}

fn sidebar_button<'a>(
    icon: &'static [u8],
    is_active: bool,
    theme: ThemeProps<'a>,
    on_press: SidebarMenuEvent,
) -> Element<'a, SidebarMenuEvent> {
    let palette = theme.theme.iced_palette();
    let base_color = palette.dim_foreground;
    let hover_color = palette.blue;
    let active_color = palette.blue;

    let icon = svg::Svg::new(svg::Handle::from_memory(icon))
        .width(Length::Fixed(MENU_ICON_SIZE))
        .height(Length::Fixed(MENU_ICON_SIZE))
        .style(move |_, status| {
            let color = if is_active {
                active_color
            } else if status == svg::Status::Hovered {
                hover_color
            } else {
                base_color
            };

            svg::Style { color: Some(color) }
        });

    let icon_container = container(icon)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .padding(MENU_BUTTON_PADDING);

    let border_color = if is_active {
        palette.blue
    } else {
        iced::Color::TRANSPARENT
    };

    let border_strip = container(Space::new())
        .width(Length::Fixed(ACTIVE_BORDER_WIDTH))
        .height(Length::Fill)
        .style(move |_| iced::widget::container::Style {
            background: Some(border_color.into()),
            ..Default::default()
        });

    let content = row![border_strip, icon_container]
        .spacing(0)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_y(alignment::Vertical::Center);

    button(content)
        .on_press(on_press)
        .padding(0)
        .width(Length::Fill)
        .height(Length::Fixed(MENU_BUTTON_SIZE))
        .style(|_, _| iced::widget::button::Style {
            background: None,
            border: Border::default(),
            ..Default::default()
        })
        .into()
}
