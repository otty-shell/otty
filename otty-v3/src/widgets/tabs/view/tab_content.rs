use iced::widget::{container, text};
use iced::{Element, Length, Theme, alignment};

use super::super::model::TabsViewModel;
use crate::theme::ThemeProps;

/// Props for rendering tab content area.
#[derive(Debug, Clone)]
pub(crate) struct TabContentProps<'a> {
    pub(crate) vm: &'a TabsViewModel,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the content area for the active tab.
///
/// In Phase 2 this is a placeholder. Terminal, settings, and other content
/// widgets will be composed here in later phases.
pub(crate) fn view<'a, Message: 'a>(
    props: TabContentProps<'a>,
) -> Element<'a, Message, Theme, iced::Renderer> {
    let palette = props.theme.theme.iced_palette();

    if !props.vm.has_tabs {
        return container(text("No tabs").color(palette.dim_foreground))
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(alignment::Horizontal::Center)
            .align_y(alignment::Vertical::Center)
            .into();
    }

    // TODO: render active tab content based on tab content type (Phase 4)
    container(text("Tab content placeholder").color(palette.dim_foreground))
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .into()
}
