use iced::widget::text::Wrapping;
use iced::widget::{column, container, row, text};
use iced::{Element, Length, Theme, alignment};

use crate::features::quick_launch::QuickLaunchState;
use crate::icons;
use crate::theme::ThemeProps;
use crate::ui::components::icon_button::{
    IconButtonProps, IconButtonVariant, view as icon_button_view,
};
use crate::ui::widgets::{quick_launches_sidebar, sidebar_workspace};

const WORKSPACE_TITLE_SIZE: f32 = 13.0;
const WORKSPACE_HEADER_PADDING_HORIZONTAL: f32 = 10.0;
const WORKSPACE_HEADER_PADDING_VERTICAL: f32 = 0.0;
const WORKSPACE_PADDING_HORIZONTAL: f32 = 0.0;
const WORKSPACE_PADDING_VERTICAL: f32 = 10.0;
const WORKSPACE_ADD_BUTTON_SIZE: f32 = 22.0;
const WORKSPACE_ADD_ICON_SIZE: f32 = 16.0;

/// Props for rendering the terminal workspace header.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SidebarWorkspaceTerminalProps<'a> {
    pub(crate) theme: ThemeProps<'a>,
    pub(crate) quick_launches: &'a QuickLaunchState,
}

/// Events emitted by sidebar workspace terminal widget.
pub(crate) type SidebarWorkspaceTerminalEvent =
    sidebar_workspace::SidebarWorkspaceEvent;

pub(crate) fn view<'a>(
    props: SidebarWorkspaceTerminalProps<'a>,
) -> Element<'a, SidebarWorkspaceTerminalEvent, Theme, iced::Renderer> {
    let title = text("SHELL")
        .size(WORKSPACE_TITLE_SIZE)
        .width(Length::Fill)
        .wrapping(Wrapping::None)
        .align_x(alignment::Horizontal::Left);

    let add_button = icon_button_view(IconButtonProps {
        icon: icons::ADD_TAB_HEADER,
        theme: props.theme,
        size: WORKSPACE_ADD_BUTTON_SIZE,
        icon_size: WORKSPACE_ADD_ICON_SIZE,
        variant: IconButtonVariant::Standard,
    })
    .map(|_| SidebarWorkspaceTerminalEvent::TerminalAddMenuOpen);

    let title_container = container(title)
        .width(Length::Fill)
        .height(Length::Shrink)
        .clip(true)
        .align_x(alignment::Horizontal::Left)
        .align_y(alignment::Vertical::Center);

    let header = row![title_container, add_button]
        .width(Length::Fill)
        .padding([
            WORKSPACE_HEADER_PADDING_VERTICAL,
            WORKSPACE_HEADER_PADDING_HORIZONTAL,
        ])
        .align_y(alignment::Vertical::Center);

    let quick_launches = quick_launches_sidebar::view(
        quick_launches_sidebar::QuickLaunchesSidebarProps {
            state: props.quick_launches,
            theme: props.theme,
        },
    )
    .map(SidebarWorkspaceTerminalEvent::QuickLaunch);

    let content = column![header, quick_launches]
        .spacing(10)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(alignment::Horizontal::Left);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding([WORKSPACE_PADDING_VERTICAL, WORKSPACE_PADDING_HORIZONTAL])
        .into()
}
