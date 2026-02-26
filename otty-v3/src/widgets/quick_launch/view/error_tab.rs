use iced::widget::{column, container, text};
use iced::{Element, Length, Theme};

use crate::shared::ui::theme::ThemeProps;
use crate::widgets::quick_launch::event::QuickLaunchEvent;
use crate::widgets::quick_launch::state::QuickLaunchErrorState;

const ERROR_TITLE_SIZE: f32 = 18.0;
const ERROR_MESSAGE_SIZE: f32 = 13.0;
const ERROR_PADDING: f32 = 24.0;
const ERROR_SPACING: f32 = 8.0;

/// Props for the quick launch error tab view.
pub(crate) struct ErrorTabProps<'a> {
    pub(crate) error: &'a QuickLaunchErrorState,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the quick launch error tab content.
pub(crate) fn view(
    props: ErrorTabProps<'_>,
) -> Element<'_, QuickLaunchEvent, Theme, iced::Renderer> {
    let palette = props.theme.theme.iced_palette();
    let red = palette.red;

    let title = text(props.error.title()).size(ERROR_TITLE_SIZE);

    let message = text(props.error.message()).size(ERROR_MESSAGE_SIZE);

    let content = column![title, message]
        .spacing(ERROR_SPACING)
        .width(Length::Fill);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(ERROR_PADDING)
        .style(move |_| iced::widget::container::Style {
            text_color: Some(red),
            ..Default::default()
        })
        .into()
}
