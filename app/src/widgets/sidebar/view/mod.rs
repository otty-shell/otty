use iced::widget::{Space, column, container, scrollable, svg};
use iced::{Element, Length, Theme};
use otty_ui_kit::components::rail_button::{RailButton, rail_separator};
use otty_ui_kit::components::surface::{Surface, surface};
use otty_ui_kit::size;

use super::event::SidebarIntent;
use super::model::SidebarViewModel;
use super::types::SidebarItem;
use crate::icons;
use crate::theme::ThemeProps;

/// Props for the sidebar view aggregator.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SidebarViewProps<'a> {
    pub(crate) vm: SidebarViewModel,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the sidebar menu rail. Workspace content is composed externally.
pub(crate) fn view(
    props: SidebarViewProps<'_>,
) -> Element<'_, SidebarIntent, Theme, iced::Renderer> {
    if props.vm.is_hidden {
        return container(Space::new())
            .width(Length::Shrink)
            .height(Length::Fill)
            .into();
    }

    let palette = props.theme.theme.ui();

    let terminal_button = rail_button(
        icons::SIDEBAR_TERMINAL,
        "Terminal",
        props.vm.active_item == SidebarItem::Terminal,
        palette,
        SidebarIntent::SelectTerminal,
    );

    let explorer_button = rail_button(
        icons::SIDEBAR_EXPLORER,
        "Explorer",
        props.vm.active_item == SidebarItem::Explorer,
        palette,
        SidebarIntent::SelectExplorer,
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

    let settings_button = rail_button(
        icons::SIDEBAR_SETTINGS,
        "Settings",
        false,
        palette,
        SidebarIntent::OpenSettings,
    );

    let (toggle_icon, toggle_label) = if props.vm.is_workspace_open {
        (icons::SIDEBAR_COLLAPSE, "Collapse workspace")
    } else {
        (icons::SIDEBAR_EXPAND, "Expand workspace")
    };

    let toggle_button = rail_button(
        toggle_icon,
        toggle_label,
        false,
        palette,
        SidebarIntent::ToggleWorkspace,
    );

    let meta_menu =
        column![rail_separator(palette), toggle_button, settings_button];

    let content = column![main_scroll, meta_menu]
        .width(Length::Fill)
        .height(Length::Fill);

    surface(content, Surface::Chrome, palette)
        .width(Length::Fixed(size::RAIL))
        .into()
}

fn rail_button<'a>(
    icon: &'static [u8],
    label: &'static str,
    is_active: bool,
    palette: &otty_ui_kit::palette::UiPalette,
    on_press: SidebarIntent,
) -> Element<'a, SidebarIntent, Theme, iced::Renderer> {
    RailButton::new(svg::Handle::from_memory(icon), label, palette)
        .active(is_active)
        .on_press(on_press)
        .into()
}
