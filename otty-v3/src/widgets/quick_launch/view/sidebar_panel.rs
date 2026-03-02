use iced::widget::{Space, column, container, row, text};
use iced::{Element, Length, Theme};

use super::sidebar_tree;
use crate::components::primitive::icon_button::{
    self, IconButtonProps, IconButtonVariant,
};
use crate::icons;
use crate::theme::ThemeProps;
use crate::widgets::quick_launch::event::QuickLaunchIntent;
use crate::widgets::quick_launch::model::QuickLaunchTreeViewModel;

const HEADER_HEIGHT: f32 = 28.0;
const TITLE_SIZE: f32 = 13.0;
const HEADER_PADDING_H: f32 = 10.0;
const ADD_BUTTON_ICON_SIZE: f32 = 14.0;

/// Props for the quick launch sidebar panel.
pub(crate) struct SidebarPanelProps<'a> {
    pub(crate) vm: QuickLaunchTreeViewModel<'a>,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the quick launch sidebar panel with header and tree.
pub(crate) fn view(
    props: SidebarPanelProps<'_>,
) -> Element<'_, QuickLaunchIntent, Theme, iced::Renderer> {
    let palette = props.theme.theme.iced_palette();
    let overlay_bg = palette.overlay;

    let add_button = icon_button::view(IconButtonProps {
        icon: icons::ADD_TAB_HEADER,
        theme: props.theme,
        size: HEADER_HEIGHT,
        icon_size: ADD_BUTTON_ICON_SIZE,
        variant: IconButtonVariant::Standard,
    })
    .map(|_| QuickLaunchIntent::HeaderAddButtonPressed);

    let header = container(
        row![
            text("SHELL").size(TITLE_SIZE),
            Space::new().width(Length::Fill),
            add_button,
        ]
        .align_y(iced::alignment::Vertical::Center)
        .width(Length::Fill)
        .height(Length::Fixed(HEADER_HEIGHT)),
    )
    .width(Length::Fill)
    .height(Length::Fixed(HEADER_HEIGHT))
    .padding([0, HEADER_PADDING_H as u16])
    .style(move |_| iced::widget::container::Style {
        background: Some(overlay_bg.into()),
        ..Default::default()
    });

    let tree = sidebar_tree::view(sidebar_tree::SidebarTreeProps {
        data: props.vm.data,
        selected_path: props.vm.selected_path,
        hovered_path: props.vm.hovered_path,
        inline_edit: props.vm.inline_edit,
        launching: props.vm.launching,
        drop_target: props.vm.drop_target,
        theme: props.theme,
    });

    column![header, tree]
        .width(Length::Fill)
        .height(Length::Fill)
        .spacing(0)
        .into()
}
