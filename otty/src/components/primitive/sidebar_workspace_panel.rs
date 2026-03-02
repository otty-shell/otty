use iced::widget::{Space, container};
use iced::{Element, Length, Theme};

use crate::theme::ThemeProps;

/// Props for rendering sidebar workspace panel container.
pub(crate) struct SidebarWorkspacePanelProps<'a, Message> {
    pub(crate) content: Element<'a, Message, Theme, iced::Renderer>,
    pub(crate) visible: bool,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the sidebar workspace content panel.
pub(crate) fn view<'a, Message: 'a>(
    props: SidebarWorkspacePanelProps<'a, Message>,
) -> Element<'a, Message, Theme, iced::Renderer> {
    if !props.visible {
        return container(Space::new())
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
    }

    let palette = props.theme.theme.iced_palette();
    container(props.content)
        .width(Length::Fill)
        .height(Length::Fill)
        .clip(true)
        .style(move |_| iced::widget::container::Style {
            background: Some(palette.dim_black.into()),
            text_color: Some(palette.foreground),
            ..Default::default()
        })
        .into()
}
