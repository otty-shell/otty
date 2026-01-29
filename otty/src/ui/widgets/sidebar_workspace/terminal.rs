use iced::alignment;
use iced::widget::text::Wrapping;
use iced::widget::{column, container, row, text};
use iced::{Element, Length, Size, Theme};

use crate::features::quick_commands::state::QuickCommandsState;
use crate::icons;
use crate::theme::ThemeProps;
use crate::ui::components::icon_button::{
    IconButton, IconButtonProps, IconButtonVariant,
};
use crate::ui::widgets::quick_commands;

const WORKSPACE_TITLE_SIZE: f32 = 13.0;
const WORKSPACE_PADDING_HORIZONTAL: f32 = 12.0;
const WORKSPACE_PADDING_VERTICAL: f32 = 10.0;
const WORKSPACE_ADD_BUTTON_SIZE: f32 = 22.0;
const WORKSPACE_ADD_ICON_SIZE: f32 = 16.0;

/// Props for rendering the terminal workspace header.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Props<'a> {
    pub(crate) theme: ThemeProps<'a>,
    pub(crate) quick_commands: &'a QuickCommandsState,
    pub(crate) workspace_size: Size,
}

pub(crate) fn view<'a>(
    props: Props<'a>,
) -> Element<'a, super::Event, Theme, iced::Renderer> {
    let title = text("SHELL")
        .size(WORKSPACE_TITLE_SIZE)
        .width(Length::Fill)
        .wrapping(Wrapping::None)
        .align_x(alignment::Horizontal::Left);

    let add_button = IconButton::new(IconButtonProps {
        icon: icons::ADD_TAB_HEADER,
        theme: props.theme,
        size: WORKSPACE_ADD_BUTTON_SIZE,
        icon_size: WORKSPACE_ADD_ICON_SIZE,
        variant: IconButtonVariant::Standard,
    })
    .view()
    .map(|_| super::Event::TerminalNewTab);

    let title_container = container(title)
        .width(Length::Fill)
        .height(Length::Shrink)
        .clip(true)
        .align_x(alignment::Horizontal::Left)
        .align_y(alignment::Vertical::Center);

    let header = row![title_container, add_button]
        .width(Length::Fill)
        .align_y(alignment::Vertical::Center);

    let quick_commands =
        quick_commands::sidebar::view(quick_commands::sidebar::Props {
            state: props.quick_commands,
            theme: props.theme,
            workspace_size: props.workspace_size,
        })
        .map(super::Event::QuickCommands);

    let content = column![header, quick_commands]
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
