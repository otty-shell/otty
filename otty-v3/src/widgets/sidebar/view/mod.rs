use iced::widget::{Space, button, column, container, row, scrollable, svg};
use iced::{Border, Element, Length, Theme, alignment};

use super::event::SidebarUiEvent;
use super::model::{SIDEBAR_MENU_WIDTH, SidebarItem, SidebarViewModel};
use crate::shared::ui::icons;
use crate::shared::ui::theme::ThemeProps;

const MENU_BUTTON_SIZE: f32 = 44.0;
const MENU_ICON_SIZE: f32 = 20.0;
const MENU_BUTTON_PADDING: f32 = 8.0;
const ACTIVE_BORDER_WIDTH: f32 = 2.0;

/// Props for the sidebar view aggregator.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SidebarViewProps<'a> {
    pub(crate) vm: SidebarViewModel,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the sidebar menu rail. Workspace content is composed externally.
pub(crate) fn view(
    props: SidebarViewProps<'_>,
) -> Element<'_, SidebarUiEvent, Theme, iced::Renderer> {
    if props.vm.is_hidden {
        return container(Space::new())
            .width(Length::Shrink)
            .height(Length::Fill)
            .into();
    }

    let palette = props.theme.theme.iced_palette();

    let terminal_button = sidebar_button(
        icons::SIDEBAR_TERMINAL,
        props.vm.active_item == SidebarItem::Terminal,
        props.theme,
        SidebarUiEvent::SelectTerminal,
    );

    let explorer_button = sidebar_button(
        icons::SIDEBAR_EXPLORER,
        props.vm.active_item == SidebarItem::Explorer,
        props.theme,
        SidebarUiEvent::SelectExplorer,
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
        SidebarUiEvent::OpenSettings,
    );

    let toggle_icon = if props.vm.is_workspace_open {
        icons::SIDEBAR_COLLAPSE
    } else {
        icons::SIDEBAR_EXPAND
    };

    let toggle_button = sidebar_button(
        toggle_icon,
        false,
        props.theme,
        SidebarUiEvent::ToggleWorkspace,
    );

    let meta_menu = column![settings_button, toggle_button].spacing(0);

    let content = column![main_scroll, meta_menu]
        .width(Length::Fill)
        .height(Length::Fill);

    container(content)
        .width(Length::Fixed(SIDEBAR_MENU_WIDTH))
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
    on_press: SidebarUiEvent,
) -> Element<'a, SidebarUiEvent, Theme, iced::Renderer> {
    let palette = theme.theme.iced_palette();
    let base_color = palette.dim_foreground;
    let hover_color = palette.blue;
    let active_color = palette.blue;

    let icon_svg = svg::Svg::new(svg::Handle::from_memory(icon))
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

    let icon_container = container(icon_svg)
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
